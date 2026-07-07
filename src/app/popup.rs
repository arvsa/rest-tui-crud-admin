#[derive(Clone, Debug)]
pub enum Popup {
    ConfirmDelete {
        record_display: String,
        record_id: String,
        endpoint: String,
    },
    Form {
        title: String,
        fields: Vec<FormField>,
        focused_field: usize,
        mode: FormMode,
        endpoint: String,
        id_field: String,
        edit_mode: EditMode,
        pending_op: Option<char>,
    },
    Help,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormMode {
    Create,
    Edit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditMode {
    Normal,
    Insert,
}

#[derive(Clone, Debug)]
pub struct FormField {
    pub label: String,
    pub value: String,
    /// Char index (not byte index) of the cursor within `value`.
    pub cursor: usize,
}

/// Converts a char index into a byte offset, char-boundary safe for UTF-8.
fn byte_offset_for_char_idx(s: &str, idx: usize) -> usize {
    s.char_indices().nth(idx).map(|(b, _)| b).unwrap_or(s.len())
}

impl FormField {
    /// New field with the cursor placed at the end of `value` (matches today's
    /// append-only behavior by default).
    pub fn new(label: String, value: String) -> Self {
        let cursor = value.chars().count();
        Self {
            label,
            value,
            cursor,
        }
    }

    fn char_count(&self) -> usize {
        self.value.chars().count()
    }

    fn clamp_cursor(&mut self) {
        self.cursor = self.cursor.min(self.char_count());
    }

    pub fn insert_char(&mut self, c: char) {
        let b = byte_offset_for_char_idx(&self.value, self.cursor);
        self.value.insert(b, c);
        self.cursor += 1;
    }

    /// Deletes the char before the cursor (vim/readline backspace).
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = byte_offset_for_char_idx(&self.value, self.cursor - 1);
        let end = byte_offset_for_char_idx(&self.value, self.cursor);
        self.value.replace_range(start..end, "");
        self.cursor -= 1;
    }

    /// Deletes the char under the cursor (vim `x`).
    pub fn delete_char_under_cursor(&mut self) {
        if self.cursor >= self.char_count() {
            return;
        }
        let start = byte_offset_for_char_idx(&self.value, self.cursor);
        let end = byte_offset_for_char_idx(&self.value, self.cursor + 1);
        self.value.replace_range(start..end, "");
        self.clamp_cursor();
    }

    /// Zero-indexed (row, col) of the cursor, splitting `value` on `\n`.
    pub fn line_col(&self) -> (usize, usize) {
        let mut row = 0;
        let mut col = 0;
        for (i, c) in self.value.chars().enumerate() {
            if i == self.cursor {
                break;
            }
            if c == '\n' {
                row += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (row, col)
    }

    fn line_len(&self, row: usize) -> usize {
        self.value
            .split('\n')
            .nth(row)
            .map(|l| l.chars().count())
            .unwrap_or(0)
    }

    fn line_count(&self) -> usize {
        self.value.split('\n').count()
    }

    /// Inverse of `line_col` — converts (row, col) back to a char cursor index,
    /// clamping both row and col into range.
    pub fn cursor_for_line_col(&self, row: usize, col: usize) -> usize {
        let lines: Vec<&str> = self.value.split('\n').collect();
        let row = row.min(lines.len().saturating_sub(1));
        let line_len = lines.get(row).map(|l| l.chars().count()).unwrap_or(0);
        let col = col.min(line_len);
        let mut idx = 0;
        for line in lines.iter().take(row) {
            idx += line.chars().count() + 1; // +1 for the '\n' separator
        }
        idx + col
    }

    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.char_count());
    }

    pub fn move_up(&mut self) {
        let (row, col) = self.line_col();
        if row == 0 {
            return;
        }
        self.cursor = self.cursor_for_line_col(row - 1, col);
    }

    pub fn move_down(&mut self) {
        let (row, col) = self.line_col();
        if row + 1 >= self.line_count() {
            return;
        }
        self.cursor = self.cursor_for_line_col(row + 1, col);
    }

    /// vim `0`
    pub fn line_start(&mut self) {
        let (row, _) = self.line_col();
        self.cursor = self.cursor_for_line_col(row, 0);
    }

    /// vim `$`
    pub fn line_end(&mut self) {
        let (row, _) = self.line_col();
        let len = self.line_len(row);
        self.cursor = self.cursor_for_line_col(row, len);
    }

    /// vim `w` — jump to the start of the next word.
    pub fn word_forward(&mut self) {
        let chars: Vec<char> = self.value.chars().collect();
        let n = chars.len();
        let mut i = self.cursor;
        if i >= n {
            return;
        }
        if !chars[i].is_whitespace() {
            while i < n && !chars[i].is_whitespace() {
                i += 1;
            }
        }
        while i < n && chars[i].is_whitespace() {
            i += 1;
        }
        self.cursor = i;
    }

    /// vim `b` — jump to the start of the previous word.
    pub fn word_back(&mut self) {
        let chars: Vec<char> = self.value.chars().collect();
        if self.cursor == 0 {
            return;
        }
        let mut i = self.cursor - 1;
        while i > 0 && chars[i].is_whitespace() {
            i -= 1;
        }
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }
        self.cursor = i;
    }

    /// vim `dd` — delete the whole line the cursor is on.
    pub fn delete_current_line(&mut self) {
        let (row, _) = self.line_col();
        let mut lines: Vec<String> = self.value.split('\n').map(String::from).collect();
        if row >= lines.len() {
            return;
        }
        lines.remove(row);
        self.value = lines.join("\n");
        let new_row = row.min(lines.len().saturating_sub(1));
        self.cursor = self.cursor_for_line_col(new_row, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_places_cursor_at_end() {
        let f = FormField::new("desc".into(), "hello".into());
        assert_eq!(f.cursor, 5);
    }

    #[test]
    fn insert_and_backspace() {
        let mut f = FormField::new("desc".into(), String::new());
        f.insert_char('a');
        f.insert_char('b');
        f.insert_char('c');
        assert_eq!(f.value, "abc");
        assert_eq!(f.cursor, 3);
        f.move_left();
        f.insert_char('X');
        assert_eq!(f.value, "abXc");
        assert_eq!(f.cursor, 3);
        f.backspace();
        assert_eq!(f.value, "abc");
        assert_eq!(f.cursor, 2);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut f = FormField::new("desc".into(), "abc".into());
        f.cursor = 0;
        f.backspace();
        assert_eq!(f.value, "abc");
        assert_eq!(f.cursor, 0);
    }

    #[test]
    fn delete_char_under_cursor() {
        let mut f = FormField::new("desc".into(), "abc".into());
        f.cursor = 0;
        f.delete_char_under_cursor();
        assert_eq!(f.value, "bc");
        assert_eq!(f.cursor, 0);
        f.cursor = 2; // past end
        f.delete_char_under_cursor();
        assert_eq!(f.value, "bc");
    }

    #[test]
    fn move_left_right_clamped() {
        let mut f = FormField::new("desc".into(), "ab".into());
        f.cursor = 0;
        f.move_left();
        assert_eq!(f.cursor, 0);
        f.move_right();
        f.move_right();
        f.move_right();
        assert_eq!(f.cursor, 2);
    }

    #[test]
    fn line_col_round_trip_multiline() {
        let f = FormField::new("desc".into(), "ab\ncde\nf".into());
        let mut g = f.clone();
        g.cursor = 5; // 'e' in "cde" (a=0,b=1,\n=2,c=3,d=4,e=5)
        assert_eq!(g.line_col(), (1, 2));
        assert_eq!(g.cursor_for_line_col(1, 2), 5);
    }

    #[test]
    fn move_up_down_preserves_column() {
        let mut f = FormField::new("desc".into(), "abcd\nxy\nefgh".into());
        f.cursor = f.cursor_for_line_col(0, 3); // 'd' on row 0
        f.move_down();
        assert_eq!(f.line_col(), (1, 2)); // clamped to end of "xy"
        f.move_down();
        assert_eq!(f.line_col(), (2, 2)); // 'g' on row 2 (col preserved isn't possible past clamp, so col 2)
        f.move_up();
        assert_eq!(f.line_col(), (1, 2));
        f.move_up();
        f.move_up(); // no-op at row 0
        assert_eq!(f.line_col().0, 0);
    }

    #[test]
    fn line_start_and_end() {
        let mut f = FormField::new("desc".into(), "hello\nworld".into());
        f.cursor = f.cursor_for_line_col(1, 2);
        f.line_start();
        assert_eq!(f.line_col(), (1, 0));
        f.line_end();
        assert_eq!(f.line_col(), (1, 5));
    }

    #[test]
    fn word_forward_and_back() {
        let mut f = FormField::new("desc".into(), "foo bar  baz".into());
        f.cursor = 0;
        f.word_forward();
        assert_eq!(f.cursor, 4); // start of "bar"
        f.word_forward();
        assert_eq!(f.cursor, 9); // start of "baz"
        f.word_back();
        assert_eq!(f.cursor, 4); // back to start of "bar"
        f.word_back();
        assert_eq!(f.cursor, 0);
    }

    #[test]
    fn delete_current_line_middle() {
        let mut f = FormField::new("desc".into(), "a\nb\nc".into());
        f.cursor = f.cursor_for_line_col(1, 0);
        f.delete_current_line();
        assert_eq!(f.value, "a\nc");
        assert_eq!(f.line_col(), (1, 0));
    }

    #[test]
    fn delete_current_line_only_line() {
        let mut f = FormField::new("desc".into(), "onlyline".into());
        f.cursor = 3;
        f.delete_current_line();
        assert_eq!(f.value, "");
        assert_eq!(f.cursor, 0);
    }

    #[test]
    fn utf8_multibyte_safe_editing() {
        let mut f = FormField::new("desc".into(), "café 日本語".into());
        assert_eq!(f.cursor, f.value.chars().count());
        f.line_start();
        assert_eq!(f.cursor, 0);
        f.move_right();
        f.move_right();
        f.move_right(); // cursor now on 'é' boundary (index 3: c,a,f,é...)
        f.delete_char_under_cursor(); // delete 'é'
        assert_eq!(f.value, "caf 日本語");
        f.line_end();
        f.word_back();
        f.delete_current_line();
        assert_eq!(f.value, "");
    }
}

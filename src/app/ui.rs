use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
    Wrap,
};
use ratatui::Frame;

use super::popup::{FormMode, Popup};
use super::state::{ActiveComponent, AppState, FetchState};
use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(25), Constraint::Min(10)])
        .split(size);

    draw_sidebar(frame, layout[0], &app.state);

    // Clone what we need to avoid borrow conflicts
    let state_snapshot = app.state.clone();
    draw_main(frame, layout[1], &state_snapshot);

    // Popups drawn last (on top)
    if let AppState::Initialized { ref popups, .. } = app.state {
        if let Some(popup) = popups.last() {
            draw_popup(frame, size, popup);
        }
    }
}

fn draw_sidebar(frame: &mut Frame, area: Rect, state: &AppState) {
    let (models, cursor, active) = match state {
        AppState::Initialized {
            models,
            sidebar_cursor,
            active,
            ..
        } => (models.as_slice(), *sidebar_cursor, *active),
        _ => return,
    };

    let focused = active == ActiveComponent::Sidebar;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = models
        .iter()
        .map(|m| ListItem::new(m.name.as_str()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Models ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(cursor));
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_main(frame: &mut Frame, area: Rect, state: &AppState) {
    let (models, sidebar_cursor, records, fetch_state, table_cursor, active) = match state {
        AppState::Initialized {
            models,
            sidebar_cursor,
            records,
            fetch_state,
            table_cursor,
            active,
            ..
        } => (
            models.as_slice(),
            *sidebar_cursor,
            records.as_slice(),
            fetch_state,
            *table_cursor,
            *active,
        ),
        _ => return,
    };

    let focused = active == ActiveComponent::Main;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let model = models.get(sidebar_cursor);
    let title = model
        .map(|m| format!(" {} ", m.name))
        .unwrap_or_else(|| " Records ".to_string());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    match fetch_state {
        FetchState::Loading => {
            let p = Paragraph::new("Loading…")
                .block(block)
                .alignment(Alignment::Left);
            frame.render_widget(p, area);
        }
        FetchState::Error(msg) => {
            let p = Paragraph::new(format!("Error: {}", msg))
                .style(Style::default().fg(Color::Red))
                .block(block);
            frame.render_widget(p, area);
        }
        FetchState::Idle => {
            if records.is_empty() {
                let hint = if model.is_some() {
                    "No records — press r to refresh"
                } else {
                    "Select a model from the sidebar"
                };
                let p = Paragraph::new(hint)
                    .style(Style::default().fg(Color::DarkGray))
                    .block(block);
                frame.render_widget(p, area);
                return;
            }

            // Determine columns: from model.fields, else from first record's keys
            let col_names: Vec<String> = if let Some(model) = model {
                if let Some(ref fields) = model.fields {
                    fields.clone()
                } else if let Some(first) = records.first() {
                    first
                        .as_object()
                        .map(|o| o.keys().cloned().collect())
                        .unwrap_or_default()
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            let header_cells: Vec<Cell> = col_names
                .iter()
                .map(|n| {
                    Cell::from(n.as_str()).style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                })
                .collect();
            let header = Row::new(header_cells).height(1);

            let rows: Vec<Row> = records
                .iter()
                .map(|rec| {
                    let cells: Vec<Cell> = col_names
                        .iter()
                        .map(|col| Cell::from(json_str_field(rec, col)))
                        .collect();
                    Row::new(cells)
                })
                .collect();

            let col_count = col_names.len().max(1);
            let widths: Vec<Constraint> = (0..col_count)
                .map(|_| Constraint::Ratio(1, col_count as u32))
                .collect();

            let table = Table::new(rows, widths)
                .header(header)
                .block(block)
                .highlight_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            let mut table_state = TableState::default();
            table_state.select(Some(table_cursor));
            frame.render_stateful_widget(table, area, &mut table_state);
        }
    }
}

fn draw_popup(frame: &mut Frame, area: Rect, popup: &Popup) {
    match popup {
        Popup::Help => draw_help_popup(frame, area),
        Popup::ConfirmDelete {
            record_display,
            record_id,
            ..
        } => draw_confirm_popup(frame, area, record_display, record_id),
        Popup::Form {
            title,
            fields,
            focused_field,
            mode,
            ..
        } => draw_form_popup(frame, area, title, fields, *focused_field, *mode),
    }
}

fn draw_confirm_popup(frame: &mut Frame, area: Rect, display: &str, id: &str) {
    let popup_area = centered_rect(50, 7, area);
    frame.render_widget(Clear, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Delete \"{}\" (id: {})?", display, id),
            Style::default().fg(Color::Red),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [y]", Style::default().fg(Color::Green)),
            Span::raw(" Yes   "),
            Span::styled("[n/Esc]", Style::default().fg(Color::Yellow)),
            Span::raw(" No"),
        ]),
    ];

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .title(" Confirm Delete ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(paragraph, popup_area);
}

fn draw_form_popup(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    fields: &[super::popup::FormField],
    focused_field: usize,
    mode: FormMode,
) {
    // Compute the text width inside a field:
    // popup (64 or area.width) − 2 outer borders − 2 field borders
    let popup_width = 64_u16.min(area.width);
    let inner_text_width = popup_width.saturating_sub(4) as usize;

    let field_heights: Vec<u16> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            // Add cursor char to focused field so height accounts for it
            let text = if i == focused_field {
                format!("{}_", f.value)
            } else {
                f.value.clone()
            };
            let visual = visual_line_count(&text, inner_text_width);
            (visual as u16 + 2).max(3) // +2 for top/bottom borders
        })
        .collect();
    let total_height =
        (field_heights.iter().sum::<u16>() + 3).min(area.height.saturating_sub(2));

    let popup_area = centered_rect_abs(64, total_height, area);
    frame.render_widget(Clear, popup_area);

    let hint = match mode {
        FormMode::Create | FormMode::Edit => {
            "[Tab] next field   [Enter] newline   [Ctrl+s] submit   [Esc] cancel"
        }
    };

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner_area = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Split inner area: one chunk per field + 1 hint line
    let field_constraints: Vec<Constraint> = field_heights
        .iter()
        .map(|&h| Constraint::Length(h))
        .chain(std::iter::once(Constraint::Length(1)))
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(field_constraints)
        .split(inner_area);

    for (i, field) in fields.iter().enumerate() {
        if i >= chunks.len() {
            break;
        }
        let is_focused = i == focused_field;
        let border_style = if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let value_style = if is_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };
        // Append cursor to end of last line when focused
        let display = if is_focused {
            format!("{}_", field.value)
        } else {
            field.value.clone()
        };
        let p = Paragraph::new(display)
            .style(value_style)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(field.label.as_str())
                    .borders(Borders::ALL)
                    .border_style(border_style),
            );
        frame.render_widget(p, chunks[i]);
    }

    if let Some(hint_area) = chunks.get(fields.len()) {
        let p = Paragraph::new(hint).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(p, *hint_area);
    }
}

fn draw_help_popup(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 22, area);
    frame.render_widget(Clear, popup_area);

    let key_style = Style::default().fg(Color::Cyan);
    let desc_style = Style::default().fg(Color::Gray);

    let entries: &[(&str, &str)] = &[
        ("j / ↓", "Move down"),
        ("k / ↑", "Move up"),
        ("h", "Focus sidebar"),
        ("l / Enter", "Focus main / select model"),
        ("r", "Refresh records"),
        ("n", "New record"),
        ("e / Enter", "Edit selected record"),
        ("d", "Delete selected record"),
        ("y", "Confirm delete"),
        ("n / Esc", "Cancel / close popup"),
        ("Tab", "Next form field"),
        ("Ctrl+s / Enter", "Submit form"),
        ("?", "Toggle this help"),
        ("q / Ctrl+c", "Quit"),
    ];

    let rows: Vec<Row> = entries
        .iter()
        .map(|(key, desc)| {
            Row::new(vec![
                Cell::from(*key).style(key_style),
                Cell::from(*desc).style(desc_style),
            ])
        })
        .collect();

    let widths = [Constraint::Length(18), Constraint::Min(20)];
    let table = Table::new(rows, widths)
        .block(
            Block::default()
                .title(" Help — press ? or Esc to close ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .column_spacing(1);

    frame.render_widget(table, popup_area);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let w = (area.width * percent_x / 100).min(area.width);
    let x = (area.width.saturating_sub(w)) / 2;
    let h = height.min(area.height);
    let y = (area.height.saturating_sub(h)) / 2;
    Rect::new(area.x + x, area.y + y, w, h)
}

fn centered_rect_abs(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let x = (area.width.saturating_sub(w)) / 2;
    let h = height.min(area.height);
    let y = (area.height.saturating_sub(h)) / 2;
    Rect::new(area.x + x, area.y + y, w, h)
}

/// Count how many terminal lines `text` occupies when rendered at `width` chars.
/// Mirrors ratatui's word-wrap behaviour well enough for height pre-computation.
fn visual_line_count(text: &str, width: usize) -> usize {
    if width == 0 {
        return text.len().max(1);
    }
    text.lines()
        .map(|line| {
            if line.is_empty() {
                1
            } else {
                // ceil(char_count / width)
                line.chars().count().div_ceil(width)
            }
        })
        .sum::<usize>()
        .max(1)
}

fn json_str_field(record: &serde_json::Value, field: &str) -> String {
    record
        .get(field)
        .and_then(|v| {
            v.as_str()
                .map(str::to_string)
                .or_else(|| Some(v.to_string()))
        })
        .unwrap_or_default()
}

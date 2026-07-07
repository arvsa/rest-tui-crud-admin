use crate::inputs::key::Key;

use super::popup::EditMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveContext {
    Sidebar,
    Main,
    Popup(PopupContext),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupContext {
    Form(EditMode, Option<char>),
    ConfirmDelete,
    Help,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Quit,
    Back,
    ToggleHelp,
    // Sidebar
    SidebarUp,
    SidebarDown,
    SidebarSelect,
    FocusSidebar,
    // Main panel
    FocusMain,
    MainUp,
    MainDown,
    MainRefresh,
    CreateRecord,
    EditRecord,
    DeleteRecord,
    LoadNextPage,
    LoadPrevPage,
    // Form popup
    FormNextField,
    FormPrevField,
    FormInput(char),
    FormBackspace,
    FormSubmit,
    // Form popup — vim mode switches
    VimEnterInsertBefore,
    VimEnterInsertAfter,
    VimEnterInsertLineStart,
    VimEnterInsertLineEnd,
    VimOpenLineBelow,
    VimExitInsert,
    // Form popup — vim motions (Normal mode)
    VimMoveLeft,
    VimMoveRight,
    VimMoveUp,
    VimMoveDown,
    VimWordForward,
    VimWordBack,
    VimLineStart,
    VimLineEnd,
    // Form popup — vim edits (Normal mode)
    VimDeleteChar,
    VimPendingD,
    VimDeleteLine,
    VimClearPending,
    // Confirm popup
    ConfirmYes,
    ConfirmNo,
}

pub fn resolve_universal(key: Key) -> Option<Command> {
    match key {
        Key::Ctrl('c') | Key::Char('q') => Some(Command::Quit),
        Key::Char('?') => Some(Command::ToggleHelp),
        _ => None,
    }
}

pub fn resolve_contextual(context: ActiveContext, key: Key) -> Option<Command> {
    match context {
        ActiveContext::Sidebar => match key {
            Key::Char('j') | Key::Down => Some(Command::SidebarDown),
            Key::Char('k') | Key::Up => Some(Command::SidebarUp),
            Key::Char('l') | Key::Enter => Some(Command::SidebarSelect),
            _ => None,
        },
        ActiveContext::Main => match key {
            Key::Char('j') | Key::Down => Some(Command::MainDown),
            Key::Char('k') | Key::Up => Some(Command::MainUp),
            Key::Char('h') => Some(Command::FocusSidebar),
            Key::Char('r') => Some(Command::MainRefresh),
            Key::Char('n') => Some(Command::CreateRecord),
            Key::Char('e') | Key::Enter => Some(Command::EditRecord),
            Key::Char('d') => Some(Command::DeleteRecord),
            Key::Char('L') => Some(Command::LoadNextPage),
            Key::Char('H') => Some(Command::LoadPrevPage),
            Key::Esc => Some(Command::Back),
            _ => None,
        },
        ActiveContext::Popup(PopupContext::Form(EditMode::Insert, _)) => match key {
            Key::Esc => Some(Command::VimExitInsert),
            Key::Tab => Some(Command::FormNextField),
            Key::BackTab => Some(Command::FormPrevField),
            Key::Ctrl('s') => Some(Command::FormSubmit),
            Key::Enter => Some(Command::FormInput('\n')),
            Key::Backspace => Some(Command::FormBackspace),
            Key::Left => Some(Command::VimMoveLeft),
            Key::Right => Some(Command::VimMoveRight),
            Key::Up => Some(Command::VimMoveUp),
            Key::Down => Some(Command::VimMoveDown),
            Key::Home => Some(Command::VimLineStart),
            Key::End => Some(Command::VimLineEnd),
            Key::Char(c) => Some(Command::FormInput(c)),
            _ => None,
        },
        ActiveContext::Popup(PopupContext::Form(EditMode::Normal, Some('d'))) => match key {
            Key::Char('d') => Some(Command::VimDeleteLine),
            _ => Some(Command::VimClearPending),
        },
        ActiveContext::Popup(PopupContext::Form(EditMode::Normal, _)) => match key {
            Key::Char('i') => Some(Command::VimEnterInsertBefore),
            Key::Char('a') => Some(Command::VimEnterInsertAfter),
            Key::Char('I') => Some(Command::VimEnterInsertLineStart),
            Key::Char('A') => Some(Command::VimEnterInsertLineEnd),
            Key::Char('o') => Some(Command::VimOpenLineBelow),
            Key::Char('h') | Key::Left => Some(Command::VimMoveLeft),
            Key::Char('l') | Key::Right => Some(Command::VimMoveRight),
            Key::Char('j') | Key::Down => Some(Command::VimMoveDown),
            Key::Char('k') | Key::Up => Some(Command::VimMoveUp),
            Key::Char('w') => Some(Command::VimWordForward),
            Key::Char('b') => Some(Command::VimWordBack),
            Key::Char('0') => Some(Command::VimLineStart),
            Key::Char('$') => Some(Command::VimLineEnd),
            Key::Char('x') => Some(Command::VimDeleteChar),
            Key::Char('d') => Some(Command::VimPendingD),
            Key::Tab => Some(Command::FormNextField),
            Key::BackTab => Some(Command::FormPrevField),
            Key::Ctrl('s') => Some(Command::FormSubmit),
            Key::Esc => Some(Command::Back),
            _ => None,
        },
        ActiveContext::Popup(PopupContext::ConfirmDelete) => match key {
            Key::Char('y') | Key::Char('Y') => Some(Command::ConfirmYes),
            Key::Char('n') | Key::Char('N') | Key::Esc => Some(Command::ConfirmNo),
            _ => None,
        },
        ActiveContext::Popup(PopupContext::Help) => match key {
            Key::Char('?') | Key::Esc | Key::Char('q') => Some(Command::Back),
            _ => None,
        },
    }
}

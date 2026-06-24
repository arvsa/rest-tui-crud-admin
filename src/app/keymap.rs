use crate::inputs::key::Key;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveContext {
    Sidebar,
    Main,
    Popup(PopupContext),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupContext {
    Form,
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
    // Form popup
    FormNextField,
    FormPrevField,
    FormInput(char),
    FormBackspace,
    FormSubmit,
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
            Key::Esc => Some(Command::Back),
            _ => None,
        },
        ActiveContext::Popup(PopupContext::Form) => match key {
            Key::Tab => Some(Command::FormNextField),
            Key::BackTab => Some(Command::FormPrevField),
            Key::Ctrl('s') => Some(Command::FormSubmit),
            Key::Enter => Some(Command::FormInput('\n')),
            Key::Esc => Some(Command::Back),
            Key::Backspace => Some(Command::FormBackspace),
            Key::Char(c) => Some(Command::FormInput(c)),
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

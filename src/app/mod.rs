pub mod keymap;
pub mod popup;
pub mod state;
pub mod ui;

fn resolve_endpoint_with_id(template: &str, id: &str) -> String {
    if template.contains("{id}") {
        template.replace("{id}", id)
    } else {
        format!("{}/{}", template.trim_end_matches('/'), id)
    }
}

use keymap::{resolve_contextual, resolve_universal, ActiveContext, Command, PopupContext};
use popup::{FormField, FormMode, Popup};
use state::{ActiveComponent, AppState, FetchState};

use crate::config::{AppConfig, ModelConfig};
use crate::inputs::key::Key;
use crate::io::{IoEvent, PostMode};

#[derive(Debug, PartialEq, Eq)]
pub enum AppReturn {
    Exit,
    Continue,
}

pub struct App {
    io_tx: tokio::sync::mpsc::Sender<IoEvent>,
    pub state: AppState,
}

impl App {
    pub fn new(
        io_tx: tokio::sync::mpsc::Sender<IoEvent>,
        config: AppConfig,
        models: Vec<ModelConfig>,
    ) -> Self {
        Self {
            io_tx,
            state: AppState::Init { config, models },
        }
    }

    pub async fn do_action(&mut self, key: Key) -> AppReturn {
        let ctx = self.active_context();

        if let Some(cmd) = resolve_universal(key) {
            match cmd {
                Command::Quit => return AppReturn::Exit,
                cmd => self.update(cmd),
            }
        } else if let Some(cmd) = resolve_contextual(ctx, key) {
            if cmd == Command::Quit {
                return AppReturn::Exit;
            }
            self.update(cmd);
        }

        AppReturn::Continue
    }

    pub async fn update_on_tick(&mut self) -> AppReturn {
        AppReturn::Continue
    }

    fn active_context(&self) -> ActiveContext {
        match &self.state {
            AppState::Initialized { active, popups, .. } => match active {
                ActiveComponent::Sidebar => ActiveContext::Sidebar,
                ActiveComponent::Main => ActiveContext::Main,
                ActiveComponent::Popup => match popups.last() {
                    Some(Popup::Form { .. }) => ActiveContext::Popup(PopupContext::Form),
                    Some(Popup::ConfirmDelete { .. }) => {
                        ActiveContext::Popup(PopupContext::ConfirmDelete)
                    }
                    Some(Popup::Help) => ActiveContext::Popup(PopupContext::Help),
                    None => ActiveContext::Main,
                },
            },
            _ => ActiveContext::Sidebar,
        }
    }

    fn update(&mut self, cmd: Command) {
        match cmd {
            Command::Quit => {}
            Command::Back => {
                if let AppState::Initialized {
                    ref mut popups,
                    ref mut active,
                    ..
                } = self.state
                {
                    if !popups.is_empty() {
                        popups.pop();
                        *active = if popups.is_empty() {
                            ActiveComponent::Main
                        } else {
                            ActiveComponent::Popup
                        };
                    } else if *active == ActiveComponent::Main {
                        *active = ActiveComponent::Sidebar;
                    }
                }
            }
            Command::ToggleHelp => {
                if let AppState::Initialized {
                    ref mut popups,
                    ref mut active,
                    ..
                } = self.state
                {
                    if matches!(popups.last(), Some(Popup::Help)) {
                        popups.pop();
                        *active = if popups.is_empty() {
                            ActiveComponent::Main
                        } else {
                            ActiveComponent::Popup
                        };
                    } else {
                        popups.push(Popup::Help);
                        *active = ActiveComponent::Popup;
                    }
                }
            }
            Command::SidebarUp => {
                if let AppState::Initialized {
                    ref mut sidebar_cursor,
                    ref models,
                    ..
                } = self.state
                {
                    let len = models.len();
                    if len > 0 {
                        *sidebar_cursor = if *sidebar_cursor == 0 {
                            len - 1
                        } else {
                            *sidebar_cursor - 1
                        };
                    }
                }
            }
            Command::SidebarDown => {
                if let AppState::Initialized {
                    ref mut sidebar_cursor,
                    ref models,
                    ..
                } = self.state
                {
                    let len = models.len();
                    if len > 0 {
                        *sidebar_cursor = (*sidebar_cursor + 1) % len;
                    }
                }
            }
            Command::SidebarSelect | Command::FocusMain => {
                if let AppState::Initialized { ref mut active, .. } = self.state {
                    *active = ActiveComponent::Main;
                }
                self.fetch_selected_model();
            }
            Command::FocusSidebar => {
                if let AppState::Initialized { ref mut active, .. } = self.state {
                    *active = ActiveComponent::Sidebar;
                }
            }
            Command::MainUp => {
                if let AppState::Initialized {
                    ref mut table_cursor,
                    ref records,
                    ..
                } = self.state
                {
                    let len = records.len();
                    if len > 0 {
                        *table_cursor = if *table_cursor == 0 {
                            len - 1
                        } else {
                            *table_cursor - 1
                        };
                    }
                }
            }
            Command::MainDown => {
                if let AppState::Initialized {
                    ref mut table_cursor,
                    ref records,
                    ..
                } = self.state
                {
                    let len = records.len();
                    if len > 0 {
                        *table_cursor = (*table_cursor + 1) % len;
                    }
                }
            }
            Command::MainRefresh => {
                self.fetch_selected_model();
            }
            Command::CreateRecord => {
                self.open_create_form();
            }
            Command::EditRecord => {
                self.open_edit_form();
            }
            Command::DeleteRecord => {
                self.open_delete_confirm();
            }
            Command::ConfirmYes => {
                self.dispatch_delete();
            }
            Command::ConfirmNo => {
                if let AppState::Initialized {
                    ref mut popups,
                    ref mut active,
                    ..
                } = self.state
                {
                    popups.pop();
                    *active = if popups.is_empty() {
                        ActiveComponent::Main
                    } else {
                        ActiveComponent::Popup
                    };
                }
            }
            Command::FormNextField => {
                if let AppState::Initialized { ref mut popups, .. } = self.state {
                    if let Some(Popup::Form {
                        ref mut focused_field,
                        ref fields,
                        ..
                    }) = popups.last_mut()
                    {
                        let len = fields.len().max(1);
                        *focused_field = (*focused_field + 1) % len;
                    }
                }
            }
            Command::FormPrevField => {
                if let AppState::Initialized { ref mut popups, .. } = self.state {
                    if let Some(Popup::Form {
                        ref mut focused_field,
                        ref fields,
                        ..
                    }) = popups.last_mut()
                    {
                        let len = fields.len().max(1);
                        *focused_field = (*focused_field + len - 1) % len;
                    }
                }
            }
            Command::FormInput(c) => {
                if let AppState::Initialized { ref mut popups, .. } = self.state {
                    if let Some(Popup::Form {
                        ref mut fields,
                        ref focused_field,
                        ..
                    }) = popups.last_mut()
                    {
                        let idx = *focused_field;
                        if let Some(field) = fields.get_mut(idx) {
                            field.value.push(c);
                        }
                    }
                }
            }
            Command::FormBackspace => {
                if let AppState::Initialized { ref mut popups, .. } = self.state {
                    if let Some(Popup::Form {
                        ref mut fields,
                        ref focused_field,
                        ..
                    }) = popups.last_mut()
                    {
                        let idx = *focused_field;
                        if let Some(field) = fields.get_mut(idx) {
                            field.value.pop();
                        }
                    }
                }
            }
            Command::FormSubmit => {
                self.dispatch_post();
            }
        }
    }

    fn fetch_selected_model(&mut self) {
        let endpoint = match &self.state {
            AppState::Initialized {
                models,
                sidebar_cursor,
                ..
            } => models.get(*sidebar_cursor).map(|m| m.endpoint.clone()),
            _ => None,
        };

        if let Some(endpoint) = endpoint {
            if let AppState::Initialized {
                ref mut fetch_state,
                ref mut table_cursor,
                ref mut records,
                ..
            } = self.state
            {
                *fetch_state = FetchState::Loading;
                *table_cursor = 0;
                *records = vec![];
            }
            self.dispatch_sync(IoEvent::FetchList { endpoint });
        }
    }

    fn open_create_form(&mut self) {
        let info = match &self.state {
            AppState::Initialized {
                models,
                sidebar_cursor,
                records,
                ..
            } => models.get(*sidebar_cursor).and_then(|model| {
                model.create_endpoint.as_ref().map(|ep| {
                    let field_names: Vec<String> = if let Some(ref fields) = model.fields {
                        fields.clone()
                    } else if let Some(first) = records.first() {
                        if let Some(obj) = first.as_object() {
                            obj.keys().cloned().collect()
                        } else {
                            vec![model.display_field.clone()]
                        }
                    } else {
                        vec![model.display_field.clone()]
                    };
                    (
                        ep.clone(),
                        model.id_field.clone(),
                        format!("New {}", model.name),
                        field_names,
                    )
                })
            }),
            _ => None,
        };

        if let Some((endpoint, id_field, title, field_names)) = info {
            let fields = field_names
                .into_iter()
                .map(|label| FormField {
                    label,
                    value: String::new(),
                })
                .collect();

            if let AppState::Initialized {
                ref mut popups,
                ref mut active,
                ..
            } = self.state
            {
                popups.push(Popup::Form {
                    title,
                    fields,
                    focused_field: 0,
                    mode: FormMode::Create,
                    endpoint,
                    id_field,
                });
                *active = ActiveComponent::Popup;
            }
        }
    }

    fn open_edit_form(&mut self) {
        let selected_record = match &self.state {
            AppState::Initialized {
                records,
                table_cursor,
                ..
            } => records.get(*table_cursor).cloned(),
            _ => None,
        };

        let model_info = match &self.state {
            AppState::Initialized {
                models,
                sidebar_cursor,
                ..
            } => models.get(*sidebar_cursor).and_then(|m| {
                m.update_endpoint.as_ref().map(|ep| {
                    (m.id_field.clone(), ep.clone(), format!("Edit {}", m.name))
                })
            }),
            _ => None,
        };

        if let (Some(record), Some((id_field, endpoint_template, title))) =
            (selected_record, model_info)
        {
            let original_id = record
                .get(&id_field)
                .and_then(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .or_else(|| Some(v.to_string()))
                })
                .unwrap_or_default();

            let fields: Vec<FormField> = record
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .filter(|(k, _)| k.as_str() != id_field)
                        .map(|(k, v)| FormField {
                            label: k.clone(),
                            value: if let serde_json::Value::Array(arr) = v {
                                arr.iter()
                                    .map(|s| s.as_str().unwrap_or("").to_string())
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            } else {
                                v.as_str()
                                    .map(str::to_string)
                                    .unwrap_or_else(|| v.to_string())
                            },
                        })
                        .collect()
                })
                .unwrap_or_default();

            let endpoint = resolve_endpoint_with_id(&endpoint_template, &original_id);

            if let AppState::Initialized {
                ref mut popups,
                ref mut active,
                ..
            } = self.state
            {
                popups.push(Popup::Form {
                    title,
                    fields,
                    focused_field: 0,
                    mode: FormMode::Edit,
                    endpoint,
                    id_field,
                });
                *active = ActiveComponent::Popup;
            }
        }
    }

    fn open_delete_confirm(&mut self) {
        let selected = match &self.state {
            AppState::Initialized {
                records,
                table_cursor,
                ..
            } => records.get(*table_cursor).cloned(),
            _ => None,
        };

        let model_info = match &self.state {
            AppState::Initialized {
                models,
                sidebar_cursor,
                ..
            } => models.get(*sidebar_cursor).and_then(|m| {
                m.delete_endpoint.as_ref().map(|ep| {
                    (m.id_field.clone(), ep.clone(), m.display_field.clone())
                })
            }),
            _ => None,
        };

        if let (Some(record), Some((id_field, endpoint_template, display_field))) =
            (selected, model_info)
        {
            let record_id = record
                .get(&id_field)
                .and_then(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .or_else(|| Some(v.to_string()))
                })
                .unwrap_or_default();

            let record_display = record
                .get(&display_field)
                .and_then(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .or_else(|| Some(v.to_string()))
                })
                .unwrap_or_else(|| record_id.clone());

            let endpoint = resolve_endpoint_with_id(&endpoint_template, &record_id);

            if let AppState::Initialized {
                ref mut popups,
                ref mut active,
                ..
            } = self.state
            {
                popups.push(Popup::ConfirmDelete {
                    record_display,
                    record_id,
                    endpoint,
                });
                *active = ActiveComponent::Popup;
            }
        }
    }

    fn dispatch_delete(&mut self) {
        let delete_info = match &self.state {
            AppState::Initialized { popups, .. } => {
                if let Some(Popup::ConfirmDelete {
                    record_id,
                    endpoint,
                    ..
                }) = popups.last()
                {
                    Some((record_id.clone(), endpoint.clone()))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some((record_id, endpoint)) = delete_info {
            if let AppState::Initialized {
                ref mut popups,
                ref mut active,
                ..
            } = self.state
            {
                popups.pop();
                *active = if popups.is_empty() {
                    ActiveComponent::Main
                } else {
                    ActiveComponent::Popup
                };
            }
            self.dispatch_sync(IoEvent::DeleteRecord { endpoint, record_id });
        }
    }

    fn dispatch_post(&mut self) {
        let form_data = match &self.state {
            AppState::Initialized { popups, .. } => {
                if let Some(Popup::Form {
                    fields,
                    endpoint,
                    mode,
                    ..
                }) = popups.last()
                {
                    let body = serde_json::Value::Object(
                        fields
                            .iter()
                            .map(|f| {
                                let value = if f.value.contains('\n') {
                                    serde_json::Value::Array(
                                        f.value
                                            .lines()
                                            .map(|l| serde_json::Value::String(l.to_string()))
                                            .collect(),
                                    )
                                } else {
                                    serde_json::Value::String(f.value.clone())
                                };
                                (f.label.clone(), value)
                            })
                            .collect(),
                    );
                    let post_mode = match mode {
                        FormMode::Create => PostMode::Create,
                        FormMode::Edit => PostMode::Update,
                    };
                    Some((endpoint.clone(), body, post_mode))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some((endpoint, body, post_mode)) = form_data {
            if let AppState::Initialized {
                ref mut popups,
                ref mut active,
                ..
            } = self.state
            {
                popups.pop();
                *active = if popups.is_empty() {
                    ActiveComponent::Main
                } else {
                    ActiveComponent::Popup
                };
            }
            self.dispatch_sync(IoEvent::PostRecord {
                endpoint,
                body,
                mode: post_mode,
            });
        }
    }

    fn dispatch_sync(&mut self, event: IoEvent) {
        if let Err(e) = self.io_tx.try_send(event) {
            log::error!("dispatch error: {}", e);
        }
    }

    pub async fn dispatch(&mut self, action: IoEvent) {
        if let Err(e) = self.io_tx.send(action).await {
            log::error!("Error from dispatch {}", e);
        }
    }

    pub fn initialized(&mut self) {
        let (config, models) = match &self.state {
            AppState::Init { config, models } => (config.clone(), models.clone()),
            _ => return,
        };
        self.state = AppState::initialized(config, models);
    }

    pub fn finish_fetch(&mut self, result: Result<Vec<serde_json::Value>, String>) {
        if let AppState::Initialized {
            ref mut records,
            ref mut fetch_state,
            ref mut table_cursor,
            ..
        } = self.state
        {
            match result {
                Ok(data) => {
                    *records = data;
                    *table_cursor = 0;
                    *fetch_state = FetchState::Idle;
                }
                Err(e) => {
                    *fetch_state = FetchState::Error(e);
                }
            }
        }
    }

    pub fn finish_post(&mut self, result: Result<(), String>) {
        let ok = result.is_ok();
        if let AppState::Initialized { ref mut fetch_state, .. } = self.state {
            if let Err(e) = result {
                *fetch_state = FetchState::Error(e);
            }
        }
        if ok {
            self.fetch_selected_model();
        }
    }

    pub fn finish_delete(&mut self, result: Result<(), String>) {
        let ok = result.is_ok();
        if let AppState::Initialized { ref mut fetch_state, .. } = self.state {
            if let Err(e) = result {
                *fetch_state = FetchState::Error(e);
                return;
            }
        }
        if ok {
            self.fetch_selected_model();
        }
    }
}

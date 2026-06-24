use crate::config::{AppConfig, ModelConfig};

use super::popup::Popup;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveComponent {
    Sidebar,
    Main,
    Popup,
}

#[derive(Clone, Debug)]
pub enum FetchState {
    Idle,
    Loading,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct RequestConfig {
    pub base_url: String,
    pub headers: Vec<(String, String)>,
}

#[derive(Clone)]
pub enum AppState {
    Init {
        config: AppConfig,
        models: Vec<ModelConfig>,
    },
    Initialized {
        models: Vec<ModelConfig>,
        sidebar_cursor: usize,
        records: Vec<serde_json::Value>,
        fetch_state: FetchState,
        table_cursor: usize,
        popups: Vec<Popup>,
        active: ActiveComponent,
        request_config: RequestConfig,
    },
}

impl AppState {
    pub fn initialized(config: AppConfig, models: Vec<ModelConfig>) -> Self {
        let headers = config
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let request_config = RequestConfig {
            base_url: config.base_url.clone(),
            headers,
        };
        Self::Initialized {
            models,
            sidebar_cursor: 0,
            records: vec![],
            fetch_state: FetchState::Idle,
            table_cursor: 0,
            popups: vec![],
            active: ActiveComponent::Sidebar,
            request_config,
        }
    }

    pub fn is_initialized(&self) -> bool {
        matches!(self, Self::Initialized { .. })
    }

    pub fn request_base_url(&self) -> String {
        match self {
            Self::Initialized { request_config, .. } => {
                request_config.base_url.trim_end_matches('/').to_string()
            }
            _ => String::new(),
        }
    }

    pub fn request_headers(&self) -> Vec<(String, String)> {
        match self {
            Self::Initialized { request_config, .. } => request_config.headers.clone(),
            _ => vec![],
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::Init {
            config: AppConfig::default(),
            models: vec![],
        }
    }
}

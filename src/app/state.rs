use crate::config::{AppConfig, ModelConfig, PaginationConfig};
use crate::io::PageState;

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
    /// Fetching an additional page; existing records stay visible while this loads.
    LoadingMore,
    Error(String),
}

/// Tracks pagination progress for the currently selected (paginated) model.
///
/// Fetched pages are cached in `pages` so navigating backward (`H`) never
/// re-fetches; only advancing past the cached frontier (`L`) hits the network.
#[derive(Clone, Debug)]
pub struct PaginationState {
    pub config: PaginationConfig,
    /// All pages fetched so far, in order (index 0 = first page).
    pub pages: Vec<Vec<serde_json::Value>>,
    /// Index into `pages` currently displayed.
    pub current_index: usize,
    /// Page state to use when fetching the next not-yet-cached page.
    pub next: PageState,
    /// Whether a page beyond the cached frontier is known to exist.
    pub has_more: bool,
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
        // Boxed: PaginationConfig carries several Strings, which would otherwise
        // make this the by-far-largest AppState variant (clippy::large_enum_variant).
        pagination_state: Option<Box<PaginationState>>,
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
            pagination_state: None,
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

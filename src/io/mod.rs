use std::time::Duration;

use crate::config::PaginationConfig;

pub mod handler;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostMode {
    Create,
    Update,
}

/// Tracks what to request next for a paginated model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PageState {
    #[default]
    First,
    Page(u32),
    Offset(u32),
    Cursor(String),
}

/// Result of a `FetchList`: the records themselves, plus what page (if any)
/// should be requested next.
#[derive(Clone, Debug, Default)]
pub struct FetchListResult {
    pub records: Vec<serde_json::Value>,
    pub next_page_state: Option<PageState>,
}

#[derive(Clone, Debug)]
pub enum IoEvent {
    Initialize,
    Sleep(Duration),
    FetchList {
        endpoint: String,
        pagination: Option<PaginationConfig>,
        page_state: PageState,
        /// `false` replaces the currently loaded records (first fetch / refresh);
        /// `true` appends (load-more).
        append: bool,
    },
    PostRecord {
        endpoint: String,
        body: serde_json::Value,
        mode: PostMode,
    },
    DeleteRecord {
        endpoint: String,
        record_id: String,
    },
}

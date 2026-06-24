use std::time::Duration;

pub mod handler;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostMode {
    Create,
    Update,
}

#[derive(Clone, Debug)]
pub enum IoEvent {
    Initialize,
    Sleep(Duration),
    FetchList {
        endpoint: String,
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

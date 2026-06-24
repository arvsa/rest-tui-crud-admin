use std::sync::Arc;

use super::IoEvent;
use crate::api::handler::ApiServiceHandler;
use crate::app::App;
use crate::io::PostMode;
use log::{error, info};

pub struct IoHandler {
    api: ApiServiceHandler,
    app: Arc<tokio::sync::Mutex<App>>,
}

impl IoHandler {
    pub fn new(api: ApiServiceHandler, app: Arc<tokio::sync::Mutex<App>>) -> Self {
        Self { api, app }
    }

    pub async fn handle_event(&self, event: IoEvent) {
        match event {
            IoEvent::Initialize => {
                let mut app = self.app.lock().await;
                app.initialized();
            }

            IoEvent::FetchList { endpoint } => {
                let (base_url, headers) = {
                    let app = self.app.lock().await;
                    (
                        app.state.request_base_url(),
                        app.state.request_headers(),
                    )
                };
                let path = format!("{}{}", base_url, endpoint);
                match self.api.get_json(&path, &headers).await {
                    Ok(json) => {
                        let records = extract_records(json);
                        let mut app = self.app.lock().await;
                        app.finish_fetch(Ok(records));
                        info!("Fetched {}", endpoint);
                    }
                    Err(err) => {
                        let mut app = self.app.lock().await;
                        app.finish_fetch(Err(err.to_string()));
                        error!("FetchList {}: {}", endpoint, err);
                    }
                }
            }

            IoEvent::PostRecord { endpoint, body, mode } => {
                let (base_url, headers) = {
                    let app = self.app.lock().await;
                    (
                        app.state.request_base_url(),
                        app.state.request_headers(),
                    )
                };
                let url = format!("{}{}", base_url, endpoint);
                let result = match mode {
                    PostMode::Create => self.api.post_json(&url, &headers, &body).await,
                    PostMode::Update => self.api.put_json(&url, &headers, &body).await,
                }
                .map_err(|e| e.to_string());

                let mut app = self.app.lock().await;
                app.finish_post(result);
                info!("PostRecord {}", endpoint);
            }

            IoEvent::DeleteRecord { endpoint, record_id } => {
                let (base_url, headers) = {
                    let app = self.app.lock().await;
                    (
                        app.state.request_base_url(),
                        app.state.request_headers(),
                    )
                };
                let url = format!("{}{}", base_url, endpoint);
                let result = self
                    .api
                    .delete(&url, &headers)
                    .await
                    .map_err(|e| e.to_string());

                let mut app = self.app.lock().await;
                if result.is_err() {
                    error!("DeleteRecord {}/{}: {:?}", endpoint, record_id, result);
                } else {
                    info!("Deleted {}/{}", endpoint, record_id);
                }
                app.finish_delete(result);
            }

            IoEvent::Sleep(dur) => {
                tokio::time::sleep(dur).await;
            }
        }
    }
}

fn extract_records(json: serde_json::Value) -> Vec<serde_json::Value> {
    match json {
        serde_json::Value::Array(arr) => arr,
        serde_json::Value::Object(map) => {
            // Return the first array field found
            for (_, v) in map {
                if let serde_json::Value::Array(arr) = v {
                    return arr;
                }
            }
            vec![]
        }
        _ => vec![],
    }
}

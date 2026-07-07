use std::sync::Arc;

use super::IoEvent;
use crate::api::handler::ApiServiceHandler;
use crate::app::App;
use crate::config::PaginationConfig;
use crate::io::{FetchListResult, PageState, PostMode};
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

            IoEvent::FetchList {
                endpoint,
                pagination,
                page_state,
                append,
            } => {
                let (base_url, headers) = {
                    let app = self.app.lock().await;
                    (app.state.request_base_url(), app.state.request_headers())
                };
                let path = format!("{}{}", base_url, endpoint);
                let query = build_query_params(&pagination, &page_state);
                match self.api.get_json(&path, &headers, &query).await {
                    Ok(json) => {
                        let result = extract_records(json, pagination.as_ref(), &page_state);
                        let mut app = self.app.lock().await;
                        app.finish_fetch(Ok(result), append);
                        info!("Fetched {}", endpoint);
                    }
                    Err(err) => {
                        let mut app = self.app.lock().await;
                        app.finish_fetch(Err(err.to_string()), append);
                        error!("FetchList {}: {}", endpoint, err);
                    }
                }
            }

            IoEvent::PostRecord {
                endpoint,
                body,
                mode,
            } => {
                let (base_url, headers) = {
                    let app = self.app.lock().await;
                    (app.state.request_base_url(), app.state.request_headers())
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

            IoEvent::DeleteRecord {
                endpoint,
                record_id,
            } => {
                let (base_url, headers) = {
                    let app = self.app.lock().await;
                    (app.state.request_base_url(), app.state.request_headers())
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

fn build_query_params(
    pagination: &Option<PaginationConfig>,
    page_state: &PageState,
) -> Vec<(String, String)> {
    match pagination {
        None => vec![],
        Some(PaginationConfig::Page {
            page_param,
            size_param,
            page_size,
            first_page,
            ..
        }) => {
            let page = match page_state {
                PageState::Page(n) => *n,
                _ => *first_page,
            };
            vec![
                (page_param.clone(), page.to_string()),
                (size_param.clone(), page_size.to_string()),
            ]
        }
        Some(PaginationConfig::Offset {
            offset_param,
            limit_param,
            limit,
        }) => {
            let offset = match page_state {
                PageState::Offset(n) => *n,
                _ => 0,
            };
            vec![
                (offset_param.clone(), offset.to_string()),
                (limit_param.clone(), limit.to_string()),
            ]
        }
        Some(PaginationConfig::Cursor { cursor_param, .. }) => match page_state {
            PageState::Cursor(tok) => vec![(cursor_param.clone(), tok.clone())],
            _ => vec![],
        },
    }
}

/// Looks up a `.`-separated path of object keys in a JSON value, e.g. "meta.next_cursor".
fn dot_path<'a>(json: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    path.split('.').try_fold(json, |acc, key| acc.get(key))
}

fn dot_path_str(json: &serde_json::Value, path: &str) -> Option<String> {
    dot_path(json, path)?.as_str().map(String::from)
}

fn dot_path_bool(json: &serde_json::Value, path: &str) -> Option<bool> {
    dot_path(json, path)?.as_bool()
}

fn dot_path_u64(json: &serde_json::Value, path: &str) -> Option<u64> {
    dot_path(json, path)?.as_u64()
}

fn extract_records(
    json: serde_json::Value,
    pagination: Option<&PaginationConfig>,
    current: &PageState,
) -> FetchListResult {
    let records = match &json {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(map) => map
            .values()
            .find_map(|v| v.as_array().cloned())
            .unwrap_or_default(),
        _ => vec![],
    };

    let next_page_state = match pagination {
        None => None,
        Some(PaginationConfig::Page {
            page_size,
            total_pages_field,
            has_more_field,
            first_page,
            ..
        }) => {
            let current_page = match current {
                PageState::Page(n) => *n,
                _ => *first_page,
            };
            let more = if let Some(field) = has_more_field {
                dot_path_bool(&json, field).unwrap_or(false)
            } else if let Some(field) = total_pages_field {
                dot_path_u64(&json, field)
                    .map(|total| (current_page as u64) < total)
                    .unwrap_or(false)
            } else {
                records.len() as u32 >= *page_size
            };
            more.then(|| PageState::Page(current_page + 1))
        }
        Some(PaginationConfig::Offset { limit, .. }) => {
            let current_offset = match current {
                PageState::Offset(n) => *n,
                _ => 0,
            };
            (records.len() as u32 >= *limit).then(|| PageState::Offset(current_offset + limit))
        }
        Some(PaginationConfig::Cursor {
            next_cursor_field, ..
        }) => dot_path_str(&json, next_cursor_field).map(PageState::Cursor),
    };

    FetchListResult {
        records,
        next_page_state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn page_config() -> PaginationConfig {
        PaginationConfig::Page {
            page_param: "page".to_string(),
            size_param: "per_page".to_string(),
            page_size: 2,
            first_page: 1,
            total_pages_field: None,
            has_more_field: None,
        }
    }

    fn offset_config() -> PaginationConfig {
        PaginationConfig::Offset {
            offset_param: "offset".to_string(),
            limit_param: "limit".to_string(),
            limit: 2,
        }
    }

    fn cursor_config() -> PaginationConfig {
        PaginationConfig::Cursor {
            cursor_param: "cursor".to_string(),
            next_cursor_field: "meta.next_cursor".to_string(),
        }
    }

    #[test]
    fn no_pagination_produces_no_query_and_no_next_state() {
        assert_eq!(build_query_params(&None, &PageState::First), vec![]);
        let result = extract_records(json!([{"id": 1}]), None, &PageState::First);
        assert_eq!(result.records.len(), 1);
        assert!(result.next_page_state.is_none());
    }

    #[test]
    fn page_style_query_and_length_heuristic() {
        let cfg = Some(page_config());
        assert_eq!(
            build_query_params(&cfg, &PageState::First),
            vec![
                ("page".to_string(), "1".to_string()),
                ("per_page".to_string(), "2".to_string()),
            ]
        );
        assert_eq!(
            build_query_params(&cfg, &PageState::Page(3)),
            vec![
                ("page".to_string(), "3".to_string()),
                ("per_page".to_string(), "2".to_string()),
            ]
        );

        // Full page (>= page_size) => more pages assumed.
        let full = extract_records(
            json!([{"id": 1}, {"id": 2}]),
            cfg.as_ref(),
            &PageState::Page(1),
        );
        assert_eq!(full.next_page_state, Some(PageState::Page(2)));

        // Short page => exhausted.
        let short = extract_records(json!([{"id": 1}]), cfg.as_ref(), &PageState::Page(2));
        assert_eq!(short.next_page_state, None);
    }

    #[test]
    fn page_style_has_more_field_takes_priority() {
        let cfg = Some(PaginationConfig::Page {
            page_param: "page".to_string(),
            size_param: "per_page".to_string(),
            page_size: 50,
            first_page: 1,
            total_pages_field: None,
            has_more_field: Some("meta.has_more".to_string()),
        });
        let body = json!({"data": [{"id": 1}], "meta": {"has_more": true}});
        let result = extract_records(body, cfg.as_ref(), &PageState::Page(1));
        assert_eq!(result.next_page_state, Some(PageState::Page(2)));

        let body = json!({"data": [{"id": 1}], "meta": {"has_more": false}});
        let result = extract_records(body, cfg.as_ref(), &PageState::Page(1));
        assert_eq!(result.next_page_state, None);
    }

    #[test]
    fn page_style_total_pages_field_stops_on_last_page() {
        let cfg = Some(PaginationConfig::Page {
            page_param: "page".to_string(),
            size_param: "per_page".to_string(),
            page_size: 50,
            first_page: 1,
            total_pages_field: Some("meta.total_pages".to_string()),
            has_more_field: None,
        });
        let body = json!({"data": [{"id": 1}], "meta": {"total_pages": 3}});

        let page1 = extract_records(body.clone(), cfg.as_ref(), &PageState::Page(1));
        assert_eq!(page1.next_page_state, Some(PageState::Page(2)));

        let page2 = extract_records(body.clone(), cfg.as_ref(), &PageState::Page(2));
        assert_eq!(page2.next_page_state, Some(PageState::Page(3)));

        let page3 = extract_records(body, cfg.as_ref(), &PageState::Page(3));
        assert_eq!(page3.next_page_state, None);
    }

    #[test]
    fn offset_style_query_and_heuristic() {
        let cfg = Some(offset_config());
        assert_eq!(
            build_query_params(&cfg, &PageState::First),
            vec![
                ("offset".to_string(), "0".to_string()),
                ("limit".to_string(), "2".to_string()),
            ]
        );
        assert_eq!(
            build_query_params(&cfg, &PageState::Offset(4)),
            vec![
                ("offset".to_string(), "4".to_string()),
                ("limit".to_string(), "2".to_string()),
            ]
        );

        let full = extract_records(
            json!([{"id": 1}, {"id": 2}]),
            cfg.as_ref(),
            &PageState::Offset(0),
        );
        assert_eq!(full.next_page_state, Some(PageState::Offset(2)));

        let short = extract_records(json!([{"id": 1}]), cfg.as_ref(), &PageState::Offset(4));
        assert_eq!(short.next_page_state, None);
    }

    #[test]
    fn cursor_style_query_and_next_cursor() {
        let cfg = Some(cursor_config());
        assert_eq!(build_query_params(&cfg, &PageState::First), vec![]);
        assert_eq!(
            build_query_params(&cfg, &PageState::Cursor("abc".to_string())),
            vec![("cursor".to_string(), "abc".to_string())]
        );

        let body = json!({"data": [{"id": 1}], "meta": {"next_cursor": "next-token"}});
        let result = extract_records(body, cfg.as_ref(), &PageState::First);
        assert_eq!(result.records.len(), 1);
        assert_eq!(
            result.next_page_state,
            Some(PageState::Cursor("next-token".to_string()))
        );

        let body = json!({"data": [{"id": 1}], "meta": {}});
        let result = extract_records(body, cfg.as_ref(), &PageState::First);
        assert_eq!(result.next_page_state, None);
    }

    #[test]
    fn dot_path_missing_and_nested_lookup() {
        let body = json!({"meta": {"next_cursor": "tok"}});
        assert_eq!(
            dot_path_str(&body, "meta.next_cursor").as_deref(),
            Some("tok")
        );
        assert_eq!(dot_path_str(&body, "meta.missing"), None);
        assert_eq!(dot_path_str(&body, "missing.next_cursor"), None);
        assert_eq!(dot_path_bool(&json!({"a": true}), "a"), Some(true));
        assert_eq!(dot_path_u64(&json!({"a": 5}), "a"), Some(5));
    }
}

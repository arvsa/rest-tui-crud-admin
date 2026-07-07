use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AppConfig {
    pub base_url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModelConfig {
    pub name: String,
    pub endpoint: String,
    #[serde(default)]
    pub create_endpoint: Option<String>,
    #[serde(default)]
    pub update_endpoint: Option<String>,
    #[serde(default)]
    pub delete_endpoint: Option<String>,
    pub id_field: String,
    pub display_field: String,
    #[serde(default)]
    pub fields: Option<Vec<String>>,
    #[serde(default)]
    pub pagination: Option<PaginationConfig>,
}

/// Per-model pagination strategy, configured under a model's `pagination:` block.
/// Absent (`None`) means the endpoint returns all records in a single response —
/// today's behavior, unchanged.
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "style", rename_all = "snake_case")]
pub enum PaginationConfig {
    /// `?page=N&per_page=SIZE`-style pagination.
    Page {
        #[serde(default = "default_page_param")]
        page_param: String,
        #[serde(default = "default_size_param")]
        size_param: String,
        page_size: u32,
        #[serde(default = "default_first_page")]
        first_page: u32,
        /// Dot-path into the response body pointing at a total-pages number.
        #[serde(default)]
        total_pages_field: Option<String>,
        /// Dot-path into the response body pointing at a "more pages available" bool.
        #[serde(default)]
        has_more_field: Option<String>,
    },
    /// `?offset=N&limit=SIZE`-style pagination.
    Offset {
        #[serde(default = "default_offset_param")]
        offset_param: String,
        #[serde(default = "default_limit_param")]
        limit_param: String,
        limit: u32,
    },
    /// Next-cursor-in-response-body pagination.
    Cursor {
        #[serde(default = "default_cursor_param")]
        cursor_param: String,
        /// Dot-path into the response body pointing at the next cursor token.
        next_cursor_field: String,
    },
}

fn default_page_param() -> String {
    "page".to_string()
}
fn default_size_param() -> String {
    "per_page".to_string()
}
fn default_first_page() -> u32 {
    1
}
fn default_offset_param() -> String {
    "offset".to_string()
}
fn default_limit_param() -> String {
    "limit".to_string()
}
fn default_cursor_param() -> String {
    "cursor".to_string()
}

#[derive(Debug, Deserialize)]
struct ModelsFile {
    models: Vec<ModelConfig>,
}

pub fn load_app_config(path: &str) -> eyre::Result<AppConfig> {
    let text = std::fs::read_to_string(path)?;
    let cfg: AppConfig = serde_yaml::from_str(&expand_env_vars(&text))?;
    Ok(cfg)
}

pub fn load_model_configs(path: &str) -> eyre::Result<Vec<ModelConfig>> {
    let text = std::fs::read_to_string(path)?;
    let file: ModelsFile = serde_yaml::from_str(&expand_env_vars(&text))?;
    Ok(file.models)
}

/// Replaces `${VAR_NAME}` placeholders with values from the environment.
fn expand_env_vars(input: &str) -> String {
    let mut result = input.to_string();
    while let Some(start) = result.find("${") {
        let Some(end_offset) = result[start..].find('}') else {
            break;
        };
        let end = start + end_offset;
        let var_name = result[start + 2..end].to_string();
        let value = std::env::var(&var_name).unwrap_or_default();
        result = format!("{}{}{}", &result[..start], value, &result[end + 1..]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_example_yaml_parses_including_pagination_styles() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/models.example.yaml");
        let models = load_model_configs(path).expect("models.example.yaml should parse");

        let users = models.iter().find(|m| m.name == "Users").unwrap();
        assert!(users.pagination.is_none());

        let widgets = models.iter().find(|m| m.name == "Widgets").unwrap();
        match widgets.pagination.as_ref().unwrap() {
            PaginationConfig::Page {
                page_param,
                size_param,
                page_size,
                first_page,
                ..
            } => {
                assert_eq!(page_param, "page");
                assert_eq!(size_param, "per_page");
                assert_eq!(*page_size, 50);
                assert_eq!(*first_page, 1);
            }
            other => panic!("expected Page pagination, got {other:?}"),
        }

        let orders = models.iter().find(|m| m.name == "Orders").unwrap();
        match orders.pagination.as_ref().unwrap() {
            PaginationConfig::Offset {
                offset_param,
                limit_param,
                limit,
            } => {
                assert_eq!(offset_param, "offset");
                assert_eq!(limit_param, "limit");
                assert_eq!(*limit, 100);
            }
            other => panic!("expected Offset pagination, got {other:?}"),
        }

        let events = models.iter().find(|m| m.name == "Events").unwrap();
        match events.pagination.as_ref().unwrap() {
            PaginationConfig::Cursor {
                cursor_param,
                next_cursor_field,
            } => {
                assert_eq!(cursor_param, "cursor");
                assert_eq!(next_cursor_field, "meta.next_cursor");
            }
            other => panic!("expected Cursor pagination, got {other:?}"),
        }
    }
}

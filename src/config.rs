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

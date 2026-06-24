use std::sync::Arc;

use eyre::Result;
use rest_tui_crud_admin::api::handler::ApiServiceHandler;
use rest_tui_crud_admin::app::App;
use rest_tui_crud_admin::config::{load_app_config, load_model_configs};
use rest_tui_crud_admin::io::handler::IoHandler;
use rest_tui_crud_admin::io::IoEvent;
use rest_tui_crud_admin::start_ui;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let (sync_io_tx, mut sync_io_rx) = tokio::sync::mpsc::channel::<IoEvent>(100);

    let config = load_app_config("config.yaml").unwrap_or_default();
    let models = load_model_configs("models.yaml").unwrap_or_default();

    let app = Arc::new(tokio::sync::Mutex::new(App::new(
        sync_io_tx.clone(),
        config,
        models,
    )));
    let app_ui = Arc::clone(&app);
    let api = ApiServiceHandler::new();

    tokio::spawn(async move {
        let handler = IoHandler::new(api, app);
        while let Some(io_event) = sync_io_rx.recv().await {
            handler.handle_event(io_event).await;
        }
    });

    start_ui(&app_ui).await?;

    Ok(())
}

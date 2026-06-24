use std::sync::Arc;

use eyre::Result;
use plop_tui::api::handler::ApiServiceHandler;
use plop_tui::app::App;
use plop_tui::config::{load_app_config, load_model_configs};
use plop_tui::io::handler::IoHandler;
use plop_tui::io::IoEvent;
use plop_tui::start_ui;

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

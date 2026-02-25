mod app;
mod audio;
mod config;
mod hooks;
mod shortcuts;
mod text_injection;
mod text_processing;
mod transcription;

use app::App;
use config::Config;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting dictator voice transcription daemon");

    let config = Config::load()?;
    config.validate()?;

    // Create LocalSet for !Send futures (needed for Recorder which holds cpal::Stream)
    let local = tokio::task::LocalSet::new();

    local
        .run_until(async move {
            let app = App::new(config).await?;
            app.run().await
        })
        .await
}

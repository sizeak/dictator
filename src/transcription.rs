use anyhow::{Context, Result};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::{AudioResponseFormat, CreateTranscriptionRequestArgs};
use std::path::Path;

/// Configuration for transcription
pub struct TranscriptionConfig {
    pub model: String,
    pub prompt: String,
    pub language: String,
}

/// Create a transcription client
pub fn create_client(api_url: &str, api_key: &str) -> Client<OpenAIConfig> {
    let openai_config = OpenAIConfig::new()
        .with_api_base(api_url.to_string())
        .with_api_key(api_key.to_string());

    Client::with_config(openai_config)
}

/// Check if the transcription service is available
pub async fn check_availability(client: &Client<OpenAIConfig>) -> Result<()> {
    use std::time::Duration;
    use tokio::time::timeout;

    tracing::info!("Checking transcription service availability...");

    let check = timeout(Duration::from_secs(5), client.models().list()).await;

    match check {
        Ok(Ok(_)) => {
            tracing::info!("Transcription service is available");
            Ok(())
        }
        Ok(Err(e)) => {
            anyhow::bail!(
                "Transcription service is unreachable or returned an error: {}. \
                 Please ensure your transcription service is running at the configured API URL.",
                e
            )
        }
        Err(_) => {
            anyhow::bail!(
                "Transcription service check timed out after 5 seconds. \
                 Please ensure your transcription service is running and accessible."
            )
        }
    }
}

pub async fn transcribe(
    audio_path: &Path,
    client: &Client<OpenAIConfig>,
    config: &TranscriptionConfig,
) -> Result<String> {
    tracing::info!("Transcribing file: {:?}", audio_path);

    let request = CreateTranscriptionRequestArgs::default()
        .file(audio_path.to_str().context("Invalid path")?)
        .model(&config.model)
        .prompt(&config.prompt)
        .language(&config.language)
        .response_format(AudioResponseFormat::Json)
        .build()
        .context("Failed to build transcription request")?;

    let response = client
        .audio()
        .transcribe(request)
        .await
        .context("Transcription API call failed")?;

    tracing::info!("Transcription complete: {} chars", response.text.len());
    Ok(response.text)
}

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
pub fn create_client(api_url: String, api_key: String) -> Client<OpenAIConfig> {
    let openai_config = OpenAIConfig::new()
        .with_api_base(api_url)
        .with_api_key(api_key);

    Client::with_config(openai_config)
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

use anyhow::{Context, Result};
use async_openai::config::OpenAIConfig;
use async_openai::types::{AudioResponseFormat, CreateTranscriptionRequestArgs};
use async_openai::Client;
use std::path::{Path, PathBuf};

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

/// Transcribe audio file to text using OpenAI-compatible API
///
/// This function:
/// - Sends the WAV file to the transcription API
/// - Waits for the response
/// - Cleans up the temporary file
/// - Returns the transcribed text
pub async fn transcribe(
    wav_path: PathBuf,
    client: &Client<OpenAIConfig>,
    config: &TranscriptionConfig,
) -> Result<String> {
    tracing::info!("Transcribing file: {:?}", wav_path);

    let result = transcribe_file(&wav_path, client, config).await;

    // Cleanup temp file
    if let Err(e) = tokio::fs::remove_file(&wav_path).await {
        tracing::warn!("Failed to remove temp file {:?}: {}", wav_path, e);
    }

    result
}

async fn transcribe_file(
    path: &Path,
    client: &Client<OpenAIConfig>,
    config: &TranscriptionConfig,
) -> Result<String> {
    let request = CreateTranscriptionRequestArgs::default()
        .file(path.to_str().context("Invalid path")?)
        .model(&config.model)
        .prompt(&config.prompt)
        .language(&config.language)
        .response_format(AudioResponseFormat::Json)
        .temperature(0.0)
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

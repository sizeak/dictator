use anyhow::{Context, Result};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::{AudioResponseFormat, CreateTranscriptionRequestArgs};
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

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
    Ok(sanitize_transcript(&response.text))
}

/// Leading `language <Name><asr_text>` framing emitted by Qwen3-ASR
static ASR_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*language\s+[^<]*<asr_text>\s*").unwrap());

/// Any stray `<asr_text>` / `</asr_text>` tag left in the body
static ASR_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</?asr_text>").unwrap());

/// Strip the model's own control framing from a raw transcript
///
/// llama.cpp serving Qwen3-ASR returns the model's raw decoded output without
/// stripping its control tokens, so `text` arrives as
/// `language English<asr_text>The actual transcript.` (see llama.cpp issue
/// #26749). Without this the framing gets injected verbatim into the user's
/// text.
///
/// Servers that don't emit the framing (parakeet.cpp, whisper.cpp, or llama.cpp
/// once #26749 is fixed) are unaffected — the text is returned trimmed but
/// otherwise unchanged, and this function can be deleted at that point.
fn sanitize_transcript(text: &str) -> String {
    let text = ASR_PREFIX.replace(text, "");
    ASR_TAG.replace_all(&text, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strips_asr_prefix() {
        assert_eq!(
            sanitize_transcript(
                "language English<asr_text>The north wind and the sun were disputing which was the stronger."
            ),
            "The north wind and the sun were disputing which was the stronger."
        );
    }

    #[test]
    fn test_strips_other_languages() {
        assert_eq!(
            sanitize_transcript("language Mandarin Chinese<asr_text> 你好世界"),
            "你好世界"
        );
        assert_eq!(
            sanitize_transcript("LANGUAGE Norwegian<ASR_TEXT>God morgen."),
            "God morgen."
        );
    }

    #[test]
    fn test_strips_stray_closing_tag() {
        assert_eq!(
            sanitize_transcript("language English<asr_text>Hello there.</asr_text>"),
            "Hello there."
        );
    }

    #[test]
    fn test_passes_through_unmarked_text() {
        assert_eq!(
            sanitize_transcript("The north wind and the sun."),
            "The north wind and the sun."
        );
        assert_eq!(sanitize_transcript("  padded  "), "padded");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(sanitize_transcript(""), "");
        assert_eq!(sanitize_transcript("   "), "");
    }

    #[test]
    fn test_leaves_spoken_word_language_alone() {
        // No marker, so nothing is a prefix to strip
        assert_eq!(
            sanitize_transcript("Language models are good at this."),
            "Language models are good at this."
        );
        assert_eq!(
            sanitize_transcript("language is a tool for thought"),
            "language is a tool for thought"
        );
    }
}

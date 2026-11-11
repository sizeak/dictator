use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PasteMode {
    None,
    Ctrl,
    CtrlShift,
    Super,
}

impl Default for PasteMode {
    fn default() -> Self {
        Self::CtrlShift
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_api_url")]
    pub api_url: String,

    #[serde(default = "default_api_key")]
    pub api_key: String,

    #[serde(default = "default_model")]
    pub model: String,

    #[serde(default)]
    pub language: Option<String>,

    #[serde(default)]
    pub whisper_prompt: Option<String>,

    #[serde(default)]
    pub paste_mode: PasteMode,

    #[serde(default)]
    pub word_overrides: HashMap<String, String>,

    /// Turn spoken commands ("period", "new line", ...) into punctuation and symbols
    #[serde(default = "default_voice_commands")]
    pub voice_commands: bool,

    /// Pause playing media (via playerctl) while recording, resuming it afterwards
    #[serde(default = "default_pause_media")]
    pub pause_media: bool,

    #[serde(default = "default_audio_feedback")]
    pub audio_feedback: bool,

    #[serde(default = "default_start_sound")]
    pub start_sound_path: PathBuf,

    #[serde(default = "default_stop_sound")]
    pub stop_sound_path: PathBuf,

    #[serde(default = "default_complete_sound")]
    pub complete_sound_path: PathBuf,

    #[serde(default = "default_timeout")]
    pub timeout: u64,

    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    #[serde(default)]
    pub on_recording_start: Option<String>,

    #[serde(default)]
    pub on_recording_stop: Option<String>,

    #[serde(default)]
    pub cleanup: CleanupConfig,
}

/// Optional LLM post-processing of transcriptions (OpenAI-compatible chat completions)
#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct CleanupConfig {
    pub enabled: bool,

    /// Falls back to the transcription `api_url` when unset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,

    /// Falls back to the transcription `api_key` when unset, unless `api_url` points at a
    /// different host (so the transcription key is never sent elsewhere)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "String::is_empty")]
    pub model: String,

    /// Falls back to the built-in prompt when unset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// `null` omits it from the request (needed for models that reject it, e.g. OpenAI o-series)
    pub temperature: Option<f32>,

    pub timeout: u64,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_url: None,
            api_key: None,
            model: String::new(),
            system_prompt: None,
            temperature: Some(0.0),
            timeout: 20,
        }
    }
}

fn default_api_url() -> String {
    "http://localhost:8000".to_string()
}

fn default_api_key() -> String {
    "dummy".to_string()
}

fn default_model() -> String {
    "Systran/faster-whisper-base".to_string()
}

fn default_voice_commands() -> bool {
    true
}

fn default_pause_media() -> bool {
    true
}

fn default_audio_feedback() -> bool {
    true
}

fn default_start_sound() -> PathBuf {
    PathBuf::from("ping-up.ogg")
}

fn default_stop_sound() -> PathBuf {
    PathBuf::from("ping-down.ogg")
}

fn default_complete_sound() -> PathBuf {
    PathBuf::from("ping-complete.ogg")
}

fn default_timeout() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    2
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            api_key: default_api_key(),
            model: default_model(),
            language: None,
            whisper_prompt: None,
            paste_mode: PasteMode::default(),
            word_overrides: HashMap::new(),
            voice_commands: default_voice_commands(),
            pause_media: default_pause_media(),
            audio_feedback: default_audio_feedback(),
            start_sound_path: default_start_sound(),
            stop_sound_path: default_stop_sound(),
            complete_sound_path: default_complete_sound(),
            timeout: default_timeout(),
            max_retries: default_max_retries(),
            on_recording_start: None,
            on_recording_stop: None,
            cleanup: CleanupConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from the default location (~/.config/dictator/config.json)
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            tracing::info!(
                "Config file not found at {:?}, creating default config",
                config_path
            );
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let contents = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

        let config: Self = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;

        tracing::info!("Loaded config from {:?}", config_path);
        Ok(config)
    }

    /// Save configuration to the default location
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let contents = serde_json::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(&config_path, contents)
            .with_context(|| format!("Failed to write config file: {:?}", config_path))?;

        tracing::info!("Saved config to {:?}", config_path);
        Ok(())
    }

    /// Get the path to the configuration file
    fn config_path() -> Result<PathBuf> {
        let config_dir = if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(dir)
        } else {
            let home = std::env::var("HOME").context("HOME environment variable not set")?;
            PathBuf::from(home).join(".config")
        };

        Ok(config_dir.join("dictator").join("config.json"))
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.api_url.is_empty() {
            return Err(anyhow::anyhow!("api_url cannot be empty"));
        }

        if self.model.is_empty() {
            return Err(anyhow::anyhow!("model cannot be empty"));
        }

        if self.cleanup.enabled {
            if let Some(url) = &self.cleanup.api_url
                && !url.starts_with("http://")
                && !url.starts_with("https://")
            {
                return Err(anyhow::anyhow!(
                    "cleanup.api_url must start with http:// or https://"
                ));
            }

            if self.cleanup.model.trim().is_empty() {
                return Err(anyhow::anyhow!(
                    "cleanup.model must be set when cleanup is enabled"
                ));
            }

            if self.cleanup.timeout == 0 {
                return Err(anyhow::anyhow!("cleanup.timeout must be greater than 0"));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_cleanup() {
        let cases = [
            ("{}", true),
            (r#"{"cleanup": {"enabled": false}}"#, true),
            (r#"{"cleanup": {"enabled": true, "model": "m"}}"#, true),
            (r#"{"cleanup": {"enabled": true}}"#, false),
            (r#"{"cleanup": {"enabled": true, "model": " "}}"#, false),
            (
                r#"{"cleanup": {"enabled": true, "model": "m", "timeout": 0}}"#,
                false,
            ),
            (
                r#"{"cleanup": {"enabled": true, "model": "m", "api_url": ""}}"#,
                false,
            ),
        ];
        for (json, expect_ok) in cases {
            let config: Config = serde_json::from_str(json).unwrap();
            assert_eq!(config.validate().is_ok(), expect_ok, "{}", json);
        }
    }

    #[test]
    fn test_default_cleanup_skips_unset_fields() {
        let json = serde_json::to_value(Config::default()).unwrap();
        assert_eq!(
            json["cleanup"],
            serde_json::json!({"enabled": false, "temperature": 0.0, "timeout": 20})
        );
    }
}

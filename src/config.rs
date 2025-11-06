use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_shortcut")]
    pub primary_shortcut: String,

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

    #[serde(default = "default_paste_mode")]
    pub paste_mode: String,

    #[serde(default)]
    pub word_overrides: HashMap<String, String>,

    #[serde(default = "default_audio_feedback")]
    pub audio_feedback: bool,

    #[serde(default = "default_start_sound")]
    pub start_sound_path: String,

    #[serde(default = "default_stop_sound")]
    pub stop_sound_path: String,

    #[serde(default = "default_timeout")]
    pub timeout: u64,

    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

fn default_shortcut() -> String {
    "SUPER+ALT+D".to_string()
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

fn default_paste_mode() -> String {
    "ctrl_shift".to_string()
}

fn default_audio_feedback() -> bool {
    true
}

fn default_start_sound() -> String {
    "ping-up.opus".to_string()
}

fn default_stop_sound() -> String {
    "ping-down.opus".to_string()
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
            primary_shortcut: default_shortcut(),
            api_url: default_api_url(),
            api_key: default_api_key(),
            model: default_model(),
            language: None,
            whisper_prompt: None,
            paste_mode: default_paste_mode(),
            word_overrides: HashMap::new(),
            audio_feedback: default_audio_feedback(),
            start_sound_path: default_start_sound(),
            stop_sound_path: default_stop_sound(),
            timeout: default_timeout(),
            max_retries: default_max_retries(),
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

        if !["super", "ctrl_shift", "ctrl"].contains(&self.paste_mode.as_str()) {
            return Err(anyhow::anyhow!(
                "paste_mode must be one of: super, ctrl_shift, ctrl"
            ));
        }

        Ok(())
    }
}

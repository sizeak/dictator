use crate::audio::{AudioFormat, WavSink};
use crate::audio_feedback;
use crate::config::Config;
use crate::messages::AppState;
use crate::services::{Recorder, RecorderHandle};
use crate::text_injection;
use crate::text_processing::TextProcessor;
use crate::transcription;
use crate::{shortcuts, transcription::TranscriptionConfig};

use anyhow::Result;
use tokio::sync::mpsc;

pub struct App {
    state: AppState,
    config: Config,
    recorder: RecorderHandle,
    transcription_client: async_openai::Client<async_openai::config::OpenAIConfig>,
    text_processor: TextProcessor,
    shortcut_rx: mpsc::Receiver<()>,
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        let (audio_tx, audio_rx) = mpsc::channel(100);
        let format = AudioFormat::default(); // 16kHz, mono
        let sink = Box::new(WavSink::new(format));

        // Create and spawn Recorder (using spawn_local because it's !Send)
        let (recorder_tx, recorder_rx) = mpsc::channel(10);
        let recorder = Recorder::new(format, recorder_rx, audio_rx, audio_tx, sink);
        let recorder_handle = RecorderHandle::new(recorder_tx);
        tokio::task::spawn_local(recorder.run());

        let transcription_client =
            transcription::create_client(config.api_url.clone(), config.api_key.clone());

        let text_processor = TextProcessor::new(config.word_overrides.clone());

        let (shortcut_tx, shortcut_rx) = mpsc::channel(10);
        let target_keys = shortcuts::parse_shortcut(&config.primary_shortcut)?;
        tokio::spawn(shortcuts::monitor_keyboards(target_keys, shortcut_tx));

        tracing::info!(
            "Ready! Press {} to start/stop recording",
            config.primary_shortcut
        );

        Ok(Self {
            state: AppState::Idle,
            config,
            recorder: recorder_handle,
            transcription_client,
            text_processor,
            shortcut_rx,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        loop {
            tracing::debug!("Main loop: waiting for event");
            tokio::select! {
                Some(_) = self.shortcut_rx.recv() => {
                    tracing::debug!("Main loop: received shortcut signal");
                    if let Err(e) = self.handle_toggle().await {
                        tracing::error!("Error handling toggle: {}", e);
                    }
                    tracing::debug!("Main loop: handle_toggle completed");
                }
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Received Ctrl+C, shutting down");
                    break;
                }
            }
        }

        tracing::info!("Dictator shutdown complete");
        Ok(())
    }

    async fn handle_toggle(&mut self) -> Result<()> {
        tracing::debug!("handle_toggle: current state = {:?}", self.state);

        match self.state {
            AppState::Idle => {
                tracing::info!("Starting recording");
                tracing::debug!("handle_toggle: changing state to Recording");
                self.state = AppState::Recording;

                if self.config.audio_feedback {
                    audio_feedback::play_sound(&self.config.start_sound_path).await;
                }

                tracing::debug!("handle_toggle: calling recorder.start()");
                self.recorder.start().await?;
                tracing::debug!("handle_toggle: recorder.start() completed");
            }

            AppState::Recording => {
                tracing::info!("Stopping recording");
                self.state = AppState::Processing;

                let temp_file = self.recorder.stop().await?;
                tracing::info!("Recording saved to: {:?}", temp_file.path());

                if self.config.audio_feedback {
                    audio_feedback::play_sound(&self.config.stop_sound_path).await;
                }

                tracing::info!("Transcribing...");
                let transcription_config = TranscriptionConfig {
                    model: self.config.model.clone(),
                    prompt: self.config.whisper_prompt.clone().unwrap_or_default(),
                    language: self.config.language.clone().unwrap_or_default(),
                };
                let text = transcription::transcribe(
                    temp_file.path(),
                    &self.transcription_client,
                    &transcription_config,
                )
                .await?;
                tracing::info!("Transcription: {}", text);

                tracing::info!("Processing text...");
                let processed_text = self.text_processor.process(&text);

                tracing::info!("Injecting text...");
                text_injection::inject_text(processed_text, &self.config.paste_mode).await?;

                tracing::info!("Complete!");
                self.state = AppState::Idle;
            }

            AppState::Processing => {
                tracing::debug!("Already processing, ignoring toggle");
            }
        }

        Ok(())
    }
}

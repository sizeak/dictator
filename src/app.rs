use crate::audio::AudioFormat;
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

enum FeedbackSoundType {
    Start,
    Stop,
    Complete,
}

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
        let recorder = Self::setup_audio_pipeline();
        let transcription_client = transcription::create_client(&config.api_url, &config.api_key);
        let text_processor = TextProcessor::new(&config.word_overrides);
        let shortcut_rx = Self::setup_keyboard_monitoring(&config.primary_shortcut)?;

        tracing::info!(
            "Ready! Press {} to start/stop recording",
            config.primary_shortcut
        );

        Ok(Self {
            state: AppState::Idle,
            config,
            recorder,
            transcription_client,
            text_processor,
            shortcut_rx,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        loop {
            tracing::debug!("Main loop: waiting for event");
            if self.shortcut_rx.recv().await.is_some() {
                tracing::debug!("Main loop: received shortcut signal");
                if let Err(e) = self.handle_toggle().await {
                    tracing::error!("Error handling toggle: {}", e);
                }
                tracing::debug!("Main loop: handle_toggle completed");
            }
        }
    }

    async fn handle_toggle(&mut self) -> Result<()> {
        tracing::debug!("handle_toggle: current state = {:?}", self.state);

        match self.state {
            AppState::Idle => self.handle_start_recording().await?,
            AppState::Recording => self.handle_stop_and_process().await?,
            AppState::Processing => tracing::debug!("Already processing, ignoring toggle"),
        }

        Ok(())
    }

    async fn play_feedback_if_enabled(&self, sound_type: FeedbackSoundType) {
        if self.config.audio_feedback {
            let sound_path = match sound_type {
                FeedbackSoundType::Start => &self.config.start_sound_path,
                FeedbackSoundType::Stop => &self.config.stop_sound_path,
                FeedbackSoundType::Complete => &self.config.complete_sound_path,
            };
            audio_feedback::play_sound(sound_path).await;
        }
    }

    fn build_transcription_config(&self) -> TranscriptionConfig {
        TranscriptionConfig {
            model: self.config.model.clone(),
            prompt: self.config.whisper_prompt.clone().unwrap_or_default(),
            language: self.config.language.clone().unwrap_or_default(),
        }
    }

    async fn transcribe_and_process(&self, audio_path: &std::path::Path) -> Result<String> {
        tracing::info!("Transcribing...");
        let transcription_config = self.build_transcription_config();
        let text = transcription::transcribe(
            audio_path,
            &self.transcription_client,
            &transcription_config,
        )
        .await?;
        tracing::info!("Transcription: {}", text);

        tracing::info!("Processing text...");
        let processed_text = self.text_processor.process(&text);
        tracing::info!("Processed text: {}", processed_text);

        Ok(processed_text.to_string())
    }

    async fn stop_recording_with_feedback(&mut self) -> Result<tempfile::NamedTempFile> {
        tracing::info!("Stopping recording");
        self.state = AppState::Processing;

        let temp_file = self.recorder.stop().await?;
        tracing::info!("Recording saved to: {:?}", temp_file.path());

        self.play_feedback_if_enabled(FeedbackSoundType::Stop).await;

        Ok(temp_file)
    }

    async fn handle_start_recording(&mut self) -> Result<()> {
        tracing::info!("Starting recording");
        tracing::debug!("handle_toggle: changing state to Recording");
        self.state = AppState::Recording;

        self.play_feedback_if_enabled(FeedbackSoundType::Start)
            .await;

        tracing::debug!("handle_toggle: calling recorder.start()");
        self.recorder.start().await?;
        tracing::debug!("handle_toggle: recorder.start() completed");

        Ok(())
    }

    async fn handle_stop_and_process(&mut self) -> Result<()> {
        let temp_file = self.stop_recording_with_feedback().await?;
        let processed_text = self.transcribe_and_process(temp_file.path()).await?;

        tracing::info!("Injecting text...");
        text_injection::inject_text(processed_text, &self.config.paste_mode).await?;

        self.play_feedback_if_enabled(FeedbackSoundType::Complete).await;

        tracing::info!("Complete!");
        self.state = AppState::Idle;

        Ok(())
    }

    fn setup_audio_pipeline() -> RecorderHandle {
        let (audio_tx, audio_rx) = mpsc::channel(100);
        let format = AudioFormat::default(); // 16kHz, mono

        // Create and spawn Recorder (using spawn_local because it's !Send)
        let (recorder_tx, recorder_rx) = mpsc::channel(10);
        let recorder = Recorder::new(format, recorder_rx, audio_rx, audio_tx);
        let recorder_handle = RecorderHandle::new(recorder_tx);
        tokio::task::spawn_local(recorder.run());

        recorder_handle
    }

    fn setup_keyboard_monitoring(shortcut: &str) -> Result<mpsc::Receiver<()>> {
        let (shortcut_tx, shortcut_rx) = mpsc::channel(10);
        let target_keys = shortcuts::parse_shortcut(shortcut)?;
        tokio::spawn(shortcuts::monitor_keyboards(target_keys, shortcut_tx));
        Ok(shortcut_rx)
    }
}

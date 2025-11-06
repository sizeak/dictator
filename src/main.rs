mod audio;
mod audio_feedback;
mod config;
mod messages;
mod services;
mod shortcuts;
mod text_injection;
mod text_processing;
mod transcription;

use audio::{AudioFormat, AudioSink, WavSink};
use config::Config;
use messages::AppState;
use services::{Recorder, RecorderHandle};
use text_processing::TextProcessor;

use anyhow::Result;
use tokio::sync::{mpsc, watch};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    tracing::info!("Starting dictator voice transcription daemon");

    // Load configuration
    let config = Config::load()?;
    config.validate()?;

    // Create LocalSet for !Send futures (needed for Recorder which holds cpal::Stream)
    let local = tokio::task::LocalSet::new();

    local.run_until(async move { run_app(config).await }).await
}

async fn run_app(config: Config) -> Result<()> {
    // Observable application state
    let (state_tx, _state_rx) = watch::channel(AppState::Idle);

    // Setup audio capture channel
    let (audio_tx, audio_rx) = mpsc::channel(100);
    let format = AudioFormat::default(); // 16kHz, mono
    let sink: Box<dyn AudioSink + Send> = Box::new(WavSink::new(format));

    // Create and spawn Recorder (using spawn_local because it's !Send)
    let (recorder_tx, recorder_rx) = mpsc::channel(10);
    let recorder = Recorder::new(format, recorder_rx, audio_rx, audio_tx, sink);
    let recorder_handle = RecorderHandle::new(recorder_tx);
    tokio::task::spawn_local(recorder.run());

    // Setup transcription client and config
    let transcription_client =
        transcription::create_client(config.api_url.clone(), config.api_key.clone());
    let transcription_config = transcription::TranscriptionConfig {
        model: config.model.clone(),
        prompt: config.whisper_prompt.clone().unwrap_or_default(),
        language: config.language.clone().unwrap_or_default(),
    };

    // Setup text processor
    let text_processor = TextProcessor::new(config.word_overrides.clone());

    // Setup keyboard monitoring
    let (shortcut_tx, mut shortcut_rx) = mpsc::channel(10);
    let target_keys = shortcuts::parse_shortcut(&config.primary_shortcut)?;
    tokio::spawn(shortcuts::monitor_keyboards(target_keys, shortcut_tx));

    tracing::info!(
        "Ready! Press {} to start/stop recording",
        config.primary_shortcut
    );

    // Main event loop
    loop {
        tracing::debug!("Main loop: waiting for event");
        tokio::select! {
            Some(_) = shortcut_rx.recv() => {
                tracing::debug!("Main loop: received shortcut signal");
                if let Err(e) = handle_toggle(
                    &state_tx,
                    &recorder_handle,
                    &transcription_client,
                    &transcription_config,
                    &text_processor,
                    &config,
                ).await {
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

async fn handle_toggle(
    state: &watch::Sender<AppState>,
    recorder: &RecorderHandle,
    transcription_client: &async_openai::Client<async_openai::config::OpenAIConfig>,
    transcription_config: &transcription::TranscriptionConfig,
    text_processor: &TextProcessor,
    config: &Config,
) -> Result<()> {
    let current_state = state.borrow().clone();
    tracing::debug!("handle_toggle: current state = {:?}", current_state);

    match current_state {
        AppState::Idle => {
            tracing::info!("Starting recording");
            tracing::debug!("handle_toggle: changing state to Recording");
            state.send(AppState::Recording)?;

            if config.audio_feedback {
                audio_feedback::play_sound(&config.start_sound_path).await;
            }

            tracing::debug!("handle_toggle: calling recorder.start()");
            recorder.start().await?;
            tracing::debug!("handle_toggle: recorder.start() completed");
        }

        AppState::Recording => {
            tracing::info!("Stopping recording");
            state.send(AppState::Processing)?;

            if config.audio_feedback {
                audio_feedback::play_sound(&config.stop_sound_path).await;
            }

            let temp_file = recorder.stop().await?;
            tracing::info!("Recording saved to: {:?}", temp_file.path());

            tracing::info!("Transcribing...");
            let text =
                transcription::transcribe(temp_file.path(), transcription_client, transcription_config)
                    .await?;
            tracing::info!("Transcription: {}", text);

            tracing::info!("Processing text...");
            let processed_text = text_processor.process(&text);

            tracing::info!("Injecting text...");
            text_injection::inject_text(processed_text, &config.paste_mode).await?;

            tracing::info!("Complete!");
            state.send(AppState::Idle)?;
        }

        AppState::Processing => {
            tracing::debug!("Already processing, ignoring toggle");
        }
    }

    Ok(())
}

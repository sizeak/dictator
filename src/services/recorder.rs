use crate::audio::{AudioCapture, AudioFormat, AudioSink, WavSink};
use crate::messages::RecorderCommand;
use anyhow::Result;
use tempfile::NamedTempFile;
use tokio::sync::mpsc;

/// Coordinates audio capture and encoding
///
/// This service:
/// - Manages AudioCapture lifecycle
/// - Receives audio chunks via channel
/// - Streams chunks to AudioSink for encoding
/// - Handles start/stop commands
///
/// Note: This service holds cpal::Stream which is !Send, so it must be spawned
/// on a LocalSet using tokio::task::spawn_local.
pub struct Recorder {
    format: AudioFormat,
    cmd_rx: mpsc::Receiver<RecorderCommand>,
    audio_rx: mpsc::Receiver<Vec<f32>>,
    audio_tx: mpsc::Sender<Vec<f32>>,
    sink: Option<Box<dyn AudioSink + Send>>,
    stream: Option<cpal::Stream>,
    temp_file: Option<NamedTempFile>,
    recording: bool,
}

impl Recorder {
    pub fn new(
        format: AudioFormat,
        cmd_rx: mpsc::Receiver<RecorderCommand>,
        audio_rx: mpsc::Receiver<Vec<f32>>,
        audio_tx: mpsc::Sender<Vec<f32>>,
    ) -> Self {
        Self {
            format,
            cmd_rx,
            audio_rx,
            audio_tx,
            sink: None,
            stream: None,
            temp_file: None,
            recording: false,
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                // Handle commands from coordinator
                Some(cmd) = self.cmd_rx.recv() => {
                    self.handle_command(cmd).await;
                }

                // Receive and process audio chunks (only when recording)
                Some(chunk) = self.audio_rx.recv(), if self.recording => {
                    // Stream chunk to sink (Vec is moved, no copy)
                    if let Some(sink) = &mut self.sink
                        && let Err(e) = sink.write_chunk(chunk) {
                            tracing::error!("Failed to write audio chunk: {}", e);
                            self.recording = false;
                        }
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: RecorderCommand) {
        match cmd {
            RecorderCommand::Start => {
                let temp_file = match tempfile::Builder::new()
                    .prefix("dictator-")
                    .suffix(".wav")
                    .tempfile()
                {
                    Ok(file) => file,
                    Err(e) => {
                        tracing::error!("Failed to create temp file: {}", e);
                        return;
                    }
                };

                let path = temp_file.path().to_path_buf();

                // Create sink with the path
                let sink = match WavSink::new(path, self.format) {
                    Ok(s) => Box::new(s) as Box<dyn AudioSink + Send>,
                    Err(e) => {
                        tracing::error!("Failed to create sink: {}", e);
                        return;
                    }
                };

                self.sink = Some(sink);
                self.temp_file = Some(temp_file);

                // Start audio capture
                match AudioCapture::start(self.format, self.audio_tx.clone()) {
                    Ok(stream) => {
                        self.stream = Some(stream);
                        self.recording = true;
                        tracing::info!("Recording started");
                    }
                    Err(e) => {
                        tracing::error!("Failed to start capture: {}", e);
                    }
                }
            }

            RecorderCommand::Stop(reply) => {
                self.recording = false;

                // Drop the stream to stop audio capture
                self.stream = None;

                let result = if let Some(mut sink) = self.sink.take() {
                    // Drain any remaining audio chunks from the channel and write them to the sink
                    while let Ok(chunk) = self.audio_rx.try_recv() {
                        if let Err(e) = sink.write_chunk(chunk) {
                            tracing::error!("Failed to write audio chunk during drain: {}", e);
                            break;
                        }
                    }

                    // Replace audio channel with a fresh one for next recording
                    // This drops the old receiver, which causes the bridge task's tx.send() to fail
                    // and signals it to exit cleanly
                    let (new_audio_tx, new_audio_rx) = mpsc::channel(100);
                    self.audio_tx = new_audio_tx;
                    self.audio_rx = new_audio_rx;

                    // Give bridge task a moment to receive the Err from its send and exit
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                    match sink.finalize().await {
                        Ok(()) => self
                            .temp_file
                            .take()
                            .ok_or_else(|| anyhow::anyhow!("Temp file was not created")),
                        Err(e) => Err(e),
                    }
                } else {
                    Err(anyhow::anyhow!("No active sink to finalize"))
                };

                let _ = reply.send(result);

                tracing::info!("Recording stopped");
            }
        }
    }
}

/// Handle for communicating with the Recorder
#[derive(Clone)]
pub struct RecorderHandle {
    tx: mpsc::Sender<RecorderCommand>,
}

impl RecorderHandle {
    pub fn new(tx: mpsc::Sender<RecorderCommand>) -> Self {
        Self { tx }
    }

    pub async fn start(&self) -> Result<()> {
        self.tx
            .send(RecorderCommand::Start)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send start command: {}", e))
    }

    pub async fn stop(&self) -> Result<NamedTempFile> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(RecorderCommand::Stop(reply))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send stop command: {}", e))?;

        rx.await
            .map_err(|e| anyhow::anyhow!("Failed to receive stop response: {}", e))?
    }
}

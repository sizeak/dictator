use crate::audio::{AudioCapture, AudioFormat, AudioSink, WavSink};
use anyhow::Result;
use tempfile::NamedTempFile;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

/// Manages audio recording lifecycle
///
/// Spawns recording tasks on-demand when start() is called.
/// Holds the cpal::Stream (which is !Send) but spawns Send tasks for actual recording work.
pub struct Recorder {
    format: AudioFormat,
    stream: Option<cpal::Stream>,
    task_handle: Option<JoinHandle<Result<NamedTempFile>>>,
    stop_tx: Option<oneshot::Sender<()>>,
}

impl Recorder {
    pub fn new(format: AudioFormat) -> Self {
        Self {
            format,
            stream: None,
            task_handle: None,
            stop_tx: None,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Err(anyhow::anyhow!("Recording already in progress"));
        }

        let (audio_tx, audio_rx) = mpsc::channel(100);
        let stream = AudioCapture::start(self.format, audio_tx)?;

        let (stop_tx, stop_rx) = oneshot::channel();
        let task_handle = tokio::spawn(recording_task(self.format, audio_rx, stop_rx));

        self.stream = Some(stream);
        self.task_handle = Some(task_handle);
        self.stop_tx = Some(stop_tx);

        tracing::info!("Recording started");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<NamedTempFile> {
        let stream = self
            .stream
            .take()
            .ok_or_else(|| anyhow::anyhow!("No recording in progress"))?;

        let stop_tx = self
            .stop_tx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No stop signal sender"))?;

        let task_handle = self
            .task_handle
            .take()
            .ok_or_else(|| anyhow::anyhow!("No task handle"))?;

        drop(stream);
        let _ = stop_tx.send(());

        let result = task_handle
            .await
            .map_err(|e| anyhow::anyhow!("Recording task panicked: {}", e))??;

        tracing::info!("Recording stopped");
        Ok(result)
    }
}

async fn recording_task(
    format: AudioFormat,
    mut audio_rx: mpsc::Receiver<Vec<f32>>,
    mut stop_rx: oneshot::Receiver<()>,
) -> Result<NamedTempFile> {
    let temp_file = tempfile::Builder::new()
        .prefix("dictator-")
        .suffix(".wav")
        .tempfile()?;

    let path = temp_file.path().to_path_buf();
    let mut sink = WavSink::new(path, format)?;

    loop {
        tokio::select! {
            Some(chunk) = audio_rx.recv() => {
                sink.write_chunk(chunk)?;
            }
            _ = &mut stop_rx => {
                break;
            }
        }
    }

    while let Ok(chunk) = audio_rx.try_recv() {
        sink.write_chunk(chunk)?;
    }

    sink.finalize().await?;
    Ok(temp_file)
}

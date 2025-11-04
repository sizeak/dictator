use super::format::AudioFormat;
use super::sink::AudioSink;
use anyhow::Result;
use async_trait::async_trait;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};

enum WavCommand {
    Start { path: PathBuf, spec: WavSpec },
    WriteChunk(Vec<f32>), // Takes ownership, no copy
    Finalize { reply: oneshot::Sender<Result<PathBuf>> },
}

/// WAV encoder using a dedicated blocking thread for I/O
///
/// This implementation uses a separate thread to handle all file I/O operations,
/// allowing the audio processing to remain non-blocking. Audio chunks are sent
/// to the thread via a channel and written sequentially to the WAV file.
pub struct WavSink {
    tx: mpsc::UnboundedSender<WavCommand>,
    format: AudioFormat,
}

impl WavSink {
    pub fn new(format: AudioFormat) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Spawn dedicated blocking thread for WAV file I/O
        std::thread::spawn(move || {
            let mut writer: Option<WavWriter<std::io::BufWriter<std::fs::File>>> = None;
            let mut current_path: Option<PathBuf> = None;

            while let Some(cmd) = rx.blocking_recv() {
                match cmd {
                    WavCommand::Start { path, spec } => {
                        match WavWriter::create(&path, spec) {
                            Ok(w) => {
                                writer = Some(w);
                                current_path = Some(path);
                            }
                            Err(e) => {
                                eprintln!("Failed to create WAV writer: {}", e);
                            }
                        }
                    }
                    WavCommand::WriteChunk(samples) => {
                        if let Some(w) = &mut writer {
                            for sample in samples {
                                // Convert f32 (-1.0 to 1.0) to i16
                                let amplitude = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                                if let Err(e) = w.write_sample(amplitude) {
                                    eprintln!("Failed to write sample: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    WavCommand::Finalize { reply } => {
                        let result = if let Some(w) = writer.take() {
                            w.finalize()
                                .map(|_| current_path.take().unwrap())
                                .map_err(|e| anyhow::anyhow!("Failed to finalize WAV: {}", e))
                        } else {
                            Err(anyhow::anyhow!("No active writer to finalize"))
                        };
                        let _ = reply.send(result);
                    }
                }
            }
        });

        Self { tx, format }
    }
}

#[async_trait]
impl AudioSink for WavSink {
    fn start(&mut self, path: PathBuf) -> Result<()> {
        let spec = WavSpec {
            channels: self.format.channels,
            sample_rate: self.format.sample_rate,
            bits_per_sample: AudioFormat::BITS_PER_SAMPLE,
            sample_format: SampleFormat::Int,
        };

        self.tx
            .send(WavCommand::Start { path, spec })
            .map_err(|e| anyhow::anyhow!("Failed to send start command: {}", e))
    }

    fn write_chunk(&mut self, samples: Vec<f32>) -> Result<()> {
        // Vec is moved into the command - no copy
        self.tx
            .send(WavCommand::WriteChunk(samples))
            .map_err(|e| anyhow::anyhow!("Failed to send write command: {}", e))
    }

    async fn finalize(&mut self) -> Result<PathBuf> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(WavCommand::Finalize { reply })
            .map_err(|e| anyhow::anyhow!("Failed to send finalize command: {}", e))?;

        // Await until the WAV file is finalized
        rx.await
            .map_err(|e| anyhow::anyhow!("Failed to receive finalize response: {}", e))?
    }
}

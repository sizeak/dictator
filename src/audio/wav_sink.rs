use super::format::AudioFormat;
use super::sink::AudioSink;
use anyhow::Result;
use async_trait::async_trait;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};

enum WavCommand {
    WriteChunk(Vec<f32>),
    Finalize { reply: oneshot::Sender<Result<()>> },
}

/// WAV encoder using a dedicated blocking thread for I/O
///
/// This implementation uses a separate thread to handle all file I/O operations,
/// allowing the audio processing to remain non-blocking. Audio chunks are sent
/// to the thread via a channel and written sequentially to the WAV file.
pub struct WavSink {
    tx: mpsc::UnboundedSender<WavCommand>,
}

impl WavSink {
    pub fn new(path: PathBuf, format: AudioFormat) -> Result<Self> {
        let spec = WavSpec {
            channels: format.channels,
            sample_rate: format.sample_rate,
            bits_per_sample: AudioFormat::BITS_PER_SAMPLE,
            sample_format: SampleFormat::Int,
        };

        let mut writer = WavWriter::create(&path, spec)
            .map_err(|e| anyhow::anyhow!("Failed to create WAV writer: {}", e))?;

        let (tx, mut rx) = mpsc::unbounded_channel();

        std::thread::spawn(move || {
            while let Some(cmd) = rx.blocking_recv() {
                match cmd {
                    WavCommand::WriteChunk(samples) => {
                        for sample in samples {
                            // Convert f32 (-1.0 to 1.0) to i16
                            let amplitude = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                            if let Err(e) = writer.write_sample(amplitude) {
                                eprintln!("Failed to write sample: {}", e);
                                break;
                            }
                        }
                    }
                    WavCommand::Finalize { reply } => {
                        let result = writer
                            .finalize()
                            .map(|_| ())
                            .map_err(|e| anyhow::anyhow!("Failed to finalize WAV: {}", e));
                        let _ = reply.send(result);
                        break;
                    }
                }
            }
        });

        Ok(Self { tx })
    }
}

#[async_trait]
impl AudioSink for WavSink {
    fn write_chunk(&mut self, samples: Vec<f32>) -> Result<()> {
        self.tx
            .send(WavCommand::WriteChunk(samples))
            .map_err(|e| anyhow::anyhow!("Failed to send write command: {}", e))
    }

    async fn finalize(&mut self) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(WavCommand::Finalize { reply })
            .map_err(|e| anyhow::anyhow!("Failed to send finalize command: {}", e))?;

        rx.await
            .map_err(|e| anyhow::anyhow!("Failed to receive finalize response: {}", e))?
    }
}

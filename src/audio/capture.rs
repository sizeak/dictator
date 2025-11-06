use super::format::AudioFormat;
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleRate, StreamConfig};
use ringbuf::{traits::*, HeapRb};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// Audio capture module that fully encapsulates ring buffer implementation
///
/// This is a zero-sized type that provides methods to start audio capture
pub struct AudioCapture;

impl AudioCapture {
    /// Start audio capture
    ///
    /// This sets up the cpal stream and spawns the bridge task.
    /// Returns the stream which must be kept alive for audio capture to continue.
    /// Audio chunks are sent via chunk_tx.
    pub fn start(
        format: AudioFormat,
        chunk_tx: mpsc::Sender<Vec<f32>>,
    ) -> Result<cpal::Stream> {
        tracing::debug!("AudioCapture::start - Creating ring buffer");
        // Create ring buffer - 60 seconds at configured sample rate
        let ring = HeapRb::<f32>::new((format.sample_rate * 60) as usize);
        let (mut producer, consumer) = ring.split();

        tracing::debug!("AudioCapture::start - Getting default host");
        // Setup cpal audio stream
        let host = cpal::default_host();

        tracing::debug!("AudioCapture::start - Getting default input device");
        let device = host
            .default_input_device()
            .context("No input audio device available")?;

        tracing::debug!("AudioCapture::start - Creating stream config");
        let config = StreamConfig {
            channels: format.channels,
            sample_rate: SampleRate(format.sample_rate),
            buffer_size: BufferSize::Default,
        };

        tracing::debug!("AudioCapture::start - Building input stream");
        // Real-time audio callback - lock-free write to ringbuf
        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    // Write samples to ring buffer (lock-free operation)
                    producer.push_slice(data);
                },
                move |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None,
            )
            .context("Failed to build input stream")?;

        tracing::debug!("AudioCapture::start - Starting stream playback");
        stream.play().context("Failed to start audio stream")?;

        tracing::debug!("AudioCapture::start - Spawning bridge task");
        // Spawn bridge task to read from ringbuf and send chunks
        // Chunk size is 0.5 seconds worth of audio samples
        let chunk_size = (format.sample_rate / 2) as usize;
        tokio::task::spawn_local(Self::bridge_task(consumer, chunk_tx, chunk_size));

        tracing::info!("Audio capture started");
        Ok(stream)
    }

    /// Internal bridge task that polls ringbuf and sends chunks via channel
    ///
    /// This task runs asynchronously and bridges the gap between the sync
    /// real-time audio callback and the async tokio world
    async fn bridge_task(
        mut consumer: impl Consumer<Item = f32> + Observer,
        tx: mpsc::Sender<Vec<f32>>,
        chunk_size: usize,
    ) {
        let mut tick = interval(Duration::from_millis(50)); // Poll every 50ms

        loop {
            tick.tick().await;

            let available = consumer.occupied_len();
            if available >= chunk_size {
                // Allocate and copy from ringbuf (unavoidable single copy)
                let mut chunk = vec![0.0f32; chunk_size];
                let n = consumer.pop_slice(&mut chunk);
                chunk.truncate(n);

                // Send chunk - Vec is moved through channel, no additional copy
                if tx.send(chunk).await.is_err() {
                    // Receiver dropped, exit bridge task
                    break;
                }
            }
        }

        tracing::debug!("Audio bridge task exiting");
    }
}

use super::format::AudioFormat;
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleRate, StreamConfig};
use ringbuf::{HeapRb, traits::*};
use std::sync::Arc;
use tokio::sync::{Notify, mpsc};

pub struct AudioCapture;

impl AudioCapture {
    /// Start audio capture
    ///
    /// Returns the stream which must be kept alive for audio capture to continue.
    /// Audio chunks are sent via chunk_tx.
    pub fn start(format: AudioFormat, chunk_tx: mpsc::Sender<Vec<f32>>) -> Result<cpal::Stream> {
        let ring = HeapRb::<f32>::new(format.samples_for_duration(60.0));
        let (mut producer, consumer) = ring.split();

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No input audio device available")?;

        let config = StreamConfig {
            channels: format.channels,
            sample_rate: SampleRate(format.sample_rate),
            buffer_size: BufferSize::Default,
        };

        let notify = Arc::new(Notify::new());
        let notify_callback = notify.clone();

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    producer.push_slice(data);
                    notify_callback.notify_one();
                },
                move |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None,
            )
            .context("Failed to build input stream")?;

        stream.play().context("Failed to start audio stream")?;

        let chunk_size = format.samples_for_duration(0.5);
        tokio::task::spawn_local(Self::bridge_task(consumer, chunk_tx, chunk_size, notify));

        tracing::info!("Audio capture started");
        Ok(stream)
    }

    async fn bridge_task(
        mut consumer: impl Consumer<Item = f32>,
        tx: mpsc::Sender<Vec<f32>>,
        chunk_size: usize,
        notify: Arc<Notify>,
    ) {
        loop {
            notify.notified().await;

            let available = consumer.occupied_len();
            if available >= chunk_size {
                let mut chunk = vec![0.0f32; chunk_size];
                let n = consumer.pop_slice(&mut chunk);
                chunk.truncate(n);

                if tx.send(chunk).await.is_err() {
                    break;
                }
            }
        }
    }
}

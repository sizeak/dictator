use anyhow::Result;
use async_trait::async_trait;

/// Trait for streaming audio encoding
///
/// Implementations handle encoding audio samples to various formats (WAV, Opus, etc.)
/// in a streaming fashion, writing data as it arrives rather than buffering everything.
#[async_trait]
pub trait AudioSink: Send {
    /// Write audio samples (streaming, called repeatedly during recording)
    /// The Vec is moved to avoid copying
    fn write_chunk(&mut self, samples: Vec<f32>) -> Result<()>;

    /// Finalize and close the sink
    async fn finalize(&mut self) -> Result<()>;
}

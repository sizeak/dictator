use std::path::PathBuf;
use anyhow::Result;
use async_trait::async_trait;

/// Trait for streaming audio encoding
///
/// Implementations handle encoding audio samples to various formats (WAV, Opus, etc.)
/// in a streaming fashion, writing data as it arrives rather than buffering everything.
#[async_trait]
pub trait AudioSink: Send {
    /// Start a new recording to the given path
    fn start(&mut self, path: PathBuf) -> Result<()>;

    /// Write audio samples (streaming, called repeatedly during recording)
    /// The Vec is moved to avoid copying
    fn write_chunk(&mut self, samples: Vec<f32>) -> Result<()>;

    /// Finalize and close file, returns the path to the completed file
    async fn finalize(&mut self) -> Result<PathBuf>;
}

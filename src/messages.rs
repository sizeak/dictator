use anyhow::Result;
use tempfile::NamedTempFile;
use tokio::sync::oneshot;

/// Commands for the Recorder service
pub enum RecorderCommand {
    Start,
    Stop(oneshot::Sender<Result<NamedTempFile>>),
}

/// Application state (observable via watch channel)
#[derive(Clone, Debug, PartialEq)]
pub enum AppState {
    Idle,
    Recording,
    Processing,
}

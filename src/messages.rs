use anyhow::Result;
use tempfile::NamedTempFile;
use tokio::sync::oneshot;

pub enum RecorderCommand {
    Start,
    Stop(oneshot::Sender<Result<NamedTempFile>>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppState {
    Idle,
    Recording,
    Processing,
}

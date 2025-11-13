#[derive(Clone, Debug, PartialEq)]
pub enum AppState {
    Idle,
    Recording,
    Processing,
}

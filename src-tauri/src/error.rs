use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Tray error: {0}")]
    Tray(String),

    #[error("Hotkey error: {0}")]
    Hotkey(String),

    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Transcription error: {0}")]
    Transcription(String),

    #[error("Output error: {0}")]
    Output(String),

    #[error("Model download error: {0}")]
    ModelDownload(String),
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

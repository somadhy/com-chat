use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Serial port error: {0}")]
    Serial(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Channel send error: {0}")]
    ChannelSend(String),

    #[error("Other error: {0}")]
    Other(String),
}

use thiserror::Error;

/// Errors that can occur during device discovery or control.
#[derive(Error, Debug)]
pub enum Error {
    #[error("No RME device found matching: {model}")]
    DeviceNotFound { model: String },

    #[error("ALSA error: {0}")]
    Alsa(#[from] alsa::Error),

    #[error("Mixer element not found: {0}")]
    MixerElementNotFound(String),

    #[error("Invalid channel: {0}")]
    InvalidChannel(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

use anyhow;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CodeSkewError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Font error: {0}")]
    #[allow(dead_code)] // Public API - font error handling for external use
    FontError(String),

    #[error("Layout error: {0}")]
    LayoutError(String),

    #[error("Rendering error: {0}")]
    RenderingError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Syntax highlighting error: {0}")]
    SyntaxError(String),

    #[error("Animation error: {0}")]
    #[allow(dead_code)] // Public API - animation error handling for external use
    AnimationError(String),

    #[error("Output error: {0}")]
    OutputError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<anyhow::Error> for CodeSkewError {
    fn from(err: anyhow::Error) -> Self {
        CodeSkewError::Unknown(err.to_string())
    }
}

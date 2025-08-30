use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScraperError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON deserialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("API error: {message}")]
    Api { message: String },

    #[error("Environment variable error: {0}")]
    Env(#[from] std::env::VarError),

    #[cfg(feature = "db")]
    #[error("Database error: {message}")]
    Database { message: String },
}

impl From<sms_core::common::error::ScraperError> for ScraperError {
    fn from(err: sms_core::common::error::ScraperError) -> Self {
        ScraperError::Database { message: err.to_string() }
    }
}

pub type Result<T> = std::result::Result<T, ScraperError>;

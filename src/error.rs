use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScraperError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("JSON deserialization failed: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("TOML deserialization failed: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("API error: {message}")]
    Api { message: String },
    
    #[error("Environment variable error: {0}")]
    Env(#[from] std::env::VarError),
}

pub type Result<T> = std::result::Result<T, ScraperError>;

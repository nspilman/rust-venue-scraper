use thiserror::Error;

/// Common error types used across the SMS system
#[derive(Error, Debug)]
pub enum SmsError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("GraphQL error: {0}")]
    GraphQL(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for SMS operations
pub type SmsResult<T> = Result<T, SmsError>;

#[cfg(feature = "database")]
impl From<libsql::Error> for SmsError {
    fn from(err: libsql::Error) -> Self {
        SmsError::Database(err.to_string())
    }
}

impl From<anyhow::Error> for SmsError {
    fn from(err: anyhow::Error) -> Self {
        SmsError::Internal(err.to_string())
    }
}
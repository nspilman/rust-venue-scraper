//! Main library crate for the SMS Scraper

// Re-export the main modules needed for integration tests
pub mod apis;
pub mod app;
pub mod common;
pub mod infra;
pub mod pipeline;
pub mod observability;

// Re-export commonly used types
pub use sms_core::domain::RawData;

// Enable metrics feature if needed
#[cfg(feature = "metrics")]
pub use observability::metrics;

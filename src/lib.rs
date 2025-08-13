pub mod apis;
pub mod carpenter;
pub mod constants;
#[cfg(feature = "db")]
pub mod db;
pub mod error;
pub mod graphql;
pub mod logging;
pub mod pipeline;
pub mod server;
pub mod storage;
pub mod types;

pub mod envelope;
pub mod gateway;
pub mod idempotency;
pub mod ingest_common;
pub mod ingest_log_reader;
pub mod ingest_meta;
pub mod metrics;
pub mod parser;
pub mod rate_limiter;
pub mod registry;
pub mod tasks;

// Non-invasive architecture scaffolding to guide future refactors.
pub mod architecture;

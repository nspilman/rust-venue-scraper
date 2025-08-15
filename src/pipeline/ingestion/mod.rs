// Pipeline ingestion: data fetching, gateway operations, rate limiting, and registry

pub mod envelope;
pub mod gateway;
pub mod idempotency;
pub mod ingest_common;
pub mod ingest_log_reader;
pub mod ingest_meta;
pub mod rate_limiter;
pub mod registry;

// Re-export key types and functions for external use

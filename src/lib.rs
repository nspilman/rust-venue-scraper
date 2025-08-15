// Core modules organized by functionality
pub mod apis;
pub mod common;
#[cfg(feature = "db")]
pub mod db;
pub mod domain;
pub mod graphql;
pub mod observability;
pub mod pipeline;
pub mod server;

// Application layer (ports and use cases)
pub mod app;

// Infrastructure layer (adapters)
pub mod infra;

// Architecture scaffolding for future refactors
pub mod architecture;

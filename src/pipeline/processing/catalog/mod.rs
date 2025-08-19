// Legacy modules - kept for backward compatibility during migration
mod mapper;
mod idempotency;
mod provenance;

// Registry-based modules
pub mod candidate;
pub mod catalogger;
pub mod handler;
pub mod handlers;
pub mod registry;

// Re-export legacy utilities that might still be used elsewhere
pub use mapper::{EntityMapper, EntityUtils};
pub use idempotency::IdempotencyChecker;
pub use provenance::ProvenanceTracker;

// Export the catalogger
pub use catalogger::Catalogger;

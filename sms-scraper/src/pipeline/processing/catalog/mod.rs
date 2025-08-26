// Legacy modules - kept for backward compatibility during migration
mod mapper;

// Registry-based modules
pub mod candidate;
pub mod catalogger;
pub mod handler;
pub mod handlers;
pub mod registry;

// Re-export legacy utilities that might still be used elsewhere

// Export the catalogger
pub use catalogger::Catalogger;

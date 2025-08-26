// Pipeline storage: data persistence to various backends

pub mod traits;
pub mod in_memory;

#[cfg(feature = "db")]
pub mod database;

// Re-export the main trait and implementations at module root
pub use traits::Storage;
pub use in_memory::InMemoryStorage;

#[cfg(feature = "db")]
pub use database::DatabaseStorage;

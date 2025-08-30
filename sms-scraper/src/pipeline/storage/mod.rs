// Pipeline storage: data persistence to database backend

pub mod traits;
pub mod database;

// Re-export the main trait and implementation at module root
pub use traits::Storage;
pub use database::DatabaseStorage;

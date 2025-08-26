// Database integration trait - concrete implementation will be provided by parent projects
#[cfg(feature = "database")]
use crate::{domain::*, error::*};
#[cfg(feature = "database")]
use uuid::Uuid;

/// Trait for database operations that can be implemented by different storage backends
#[cfg(feature = "database")]
#[async_trait::async_trait]
pub trait DatabaseOperations: Send + Sync {
    /// Get events with pagination
    async fn get_events(&self, limit: Option<i32>, offset: Option<i32>) -> SmsResult<Vec<Event>>;
    
    /// Get venues with pagination  
    async fn get_venues(&self, limit: Option<i32>, offset: Option<i32>) -> SmsResult<Vec<Venue>>;
    
    /// Get artists with pagination
    async fn get_artists(&self, limit: Option<i32>, offset: Option<i32>) -> SmsResult<Vec<Artist>>;
    
    /// Get a single venue by ID
    async fn get_venue(&self, id: Uuid) -> SmsResult<Option<Venue>>;
    
    /// Get a single artist by ID
    async fn get_artist(&self, id: Uuid) -> SmsResult<Option<Artist>>;
}

// Basic placeholder implementation that the parent projects can override
#[cfg(feature = "database")]
pub struct DatabaseConnection<T: DatabaseOperations> {
    pub operations: T,
}

#[cfg(feature = "database")]
impl<T: DatabaseOperations> DatabaseConnection<T> {
    pub fn new(operations: T) -> Self {
        Self { operations }
    }
}
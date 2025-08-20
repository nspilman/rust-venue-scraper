use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;
use tracing::{debug, info, warn};

use crate::common::error::Result;
use crate::domain::ProcessRun;
use crate::pipeline::processing::conflation::ConflatedRecord;
use crate::pipeline::storage::Storage;

use super::handlers::{ArtistHandler, EventHandler, VenueHandler};
use super::registry::EntityRegistry;
use super::mapper::MapperRegistry;

/// Registry-based catalogger that uses handlers for entity processing
pub struct Catalogger {
    storage: Arc<dyn Storage>,
    registry: EntityRegistry,
    process_run_id: Option<Uuid>,
}

impl Catalogger {
    /// Test-only: list entity types handled by registered handlers
    #[cfg(test)]
    pub fn supported_entity_types(&self) -> Vec<&'static str> {
        self.registry.registered_types()
    }
    /// Create a new catalogger with default handlers registered
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        let mut registry = EntityRegistry::new();
        let mappers = Arc::new(MapperRegistry::default());
        
        // Register default handlers wired with mappers
        registry.register(Arc::new(VenueHandler::with_mappers(mappers.clone())));
        registry.register(Arc::new(EventHandler::with_mappers(mappers.clone())));
        registry.register(Arc::new(ArtistHandler::with_mappers(mappers.clone())));
        
        info!("Initialized Catalogger with {} handlers", registry.handler_count());
        
        Self {
            storage,
            registry,
            process_run_id: None,
        }
    }

    /// Test-only: create with custom registry
    #[cfg(test)]
    pub fn with_registry(storage: Arc<dyn Storage>, registry: EntityRegistry) -> Self {
        info!("Initialized Catalogger with custom registry containing {} handlers", registry.handler_count());
        Self { storage, registry, process_run_id: None }
    }
    
    /// Start a new catalog processing run
    pub async fn start_run(&mut self, name: &str) -> Result<Uuid> {
        let mut run = ProcessRun {
            id: None,
            name: name.to_string(),
            created_at: Utc::now(),
            finished_at: None,
        };

        self.storage.create_process_run(&mut run).await?;
        let run_id = run.id.expect("ProcessRun should have ID after creation");
        self.process_run_id = Some(run_id);
        
        info!("Started catalog run: {} with ID {}", name, run_id);
        Ok(run_id)
    }

    /// Finish the current catalog processing run
    pub async fn finish_run(&mut self) -> Result<()> {
        if let Some(run_id) = self.process_run_id {
            let run = ProcessRun {
                id: Some(run_id),
                name: String::new(),
                created_at: Utc::now(),
                finished_at: Some(Utc::now()),
            };

            self.storage.update_process_run(&run).await?;
            info!("Finished catalog run with ID {}", run_id);
            self.process_run_id = None;
        }
        Ok(())
    }
    
    /// Process a conflated record using registered handlers
    pub async fn catalog(&self, conflated_record: &ConflatedRecord) -> Result<()> {
        debug!(
            "Cataloging conflated record with entity {:?}", 
            conflated_record.canonical_entity_id
        );
        
        // Get current process run for provenance
        let process_run = if let Some(run_id) = self.process_run_id {
            ProcessRun {
                id: Some(run_id),
                name: String::new(),
                created_at: Utc::now(),
                finished_at: None,
            }
        } else {
            warn!("No active process run for cataloging");
            // Create a temporary run for this operation
            ProcessRun {
                id: Some(Uuid::new_v4()),
                name: "adhoc".to_string(),
                created_at: Utc::now(),
                finished_at: None,
            }
        };
        
        // Process through all applicable handlers
        let stats = self.registry.process_record(
            conflated_record,
            self.storage.as_ref(),
            &process_run,
            Utc::now()
        ).await?;
        
        // Log summary
        if stats.entities_created > 0 || stats.entities_updated > 0 {
            info!(
                "Cataloged record: {} entities created, {} updated, {} unchanged",
                stats.entities_created, stats.entities_updated, stats.entities_unchanged
            );
        } else if stats.entities_unchanged > 0 {
            debug!("No changes detected in {} entities", stats.entities_unchanged);
        }
        
        Ok(())
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::storage::in_memory::InMemoryStorage;
    
    #[tokio::test]
    async fn test_catalogger_creation() {
        let storage = Arc::new(InMemoryStorage::new());
        let catalogger = Catalogger::new(storage);
        
        // Should have 3 default handlers
        let types = catalogger.supported_entity_types();
        assert_eq!(types.len(), 3);
        assert!(types.contains(&"Venue"));
        assert!(types.contains(&"Event"));
        assert!(types.contains(&"Artist"));
    }
    
    #[tokio::test]
    async fn test_custom_registry() {
        let storage = Arc::new(InMemoryStorage::new());
        let mut registry = EntityRegistry::new();
        
        // Register only venue handler
        registry.register(Arc::new(VenueHandler::new()));
        
        let catalogger = Catalogger::with_registry(storage, registry);
        let types = catalogger.supported_entity_types();
        
        assert_eq!(types.len(), 1);
        assert!(types.contains(&"Venue"));
    }
    
    #[tokio::test]
    async fn test_process_run_lifecycle() {
        let storage = Arc::new(InMemoryStorage::new());
        let mut catalogger = Catalogger::new(storage);
        
        // Start a run
        let run_id = catalogger.start_run("test_run").await.unwrap();
        assert!(catalogger.process_run_id.is_some());
        assert_eq!(catalogger.process_run_id.unwrap(), run_id);
        
        // Finish the run
        catalogger.finish_run().await.unwrap();
        assert!(catalogger.process_run_id.is_none());
    }
}

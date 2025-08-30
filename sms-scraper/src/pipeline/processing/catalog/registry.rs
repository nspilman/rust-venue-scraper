use std::sync::Arc;
use chrono::{DateTime, Utc};
use tracing::{debug, error, info};

use sms_core::common::error::Result;
use sms_core::domain::{ProcessRecord, ProcessRun};
use crate::pipeline::processing::conflation::ConflatedRecord;
use crate::pipeline::storage::Storage;

use super::handler::EntityHandler;

/// Statistics about processing results
#[derive(Debug, Default)]
pub struct ProcessingStats {
    pub entities_created: usize,
    pub entities_updated: usize,
    pub entities_unchanged: usize,
    pub entities_skipped: usize,
    pub errors: usize,
    pub process_records: Vec<ProcessRecord>,
}

impl ProcessingStats {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn total_processed(&self) -> usize {
        self.entities_created + self.entities_updated + self.entities_unchanged + self.entities_skipped
    }
}

/// Registry that holds and manages all entity handlers
pub struct EntityRegistry {
    handlers: Vec<Arc<dyn EntityHandler>>,
}

impl EntityRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        EntityRegistry {
            handlers: Vec::new(),
        }
    }

    /// Register a new entity handler
    pub fn register(&mut self, handler: Arc<dyn EntityHandler>) {
        info!("Registering handler for entity type: {}", handler.entity_type());
        self.handlers.push(handler);
    }

    /// Process a conflated record through all applicable handlers
    pub async fn process_record(
        &self,
        record: &ConflatedRecord,
        storage: &dyn Storage,
        process_run: &ProcessRun,
        timestamp: DateTime<Utc>,
    ) -> Result<ProcessingStats> {
        let mut stats = ProcessingStats::new();
        let mut all_process_records = Vec::new();

        for handler in &self.handlers {
            if handler.can_handle(record) {
                debug!(
                    "Handler {} processing record for entity {}",
                    handler.entity_type(),
                    record.canonical_entity_id.id
                );
                
                // Step 1: Prepare the catalog candidate
                match handler.prepare_candidate(record, storage).await {
                    Ok(Some(candidate)) => {
                        // Step 2: Generate process records for audit
                        let process_records = handler.generate_process_records(
                            &candidate,
                            process_run,
                            timestamp
                        );
                        
                        // Step 3: Persist if needed
                        if candidate.should_persist {
                            match handler.persist_candidate(&candidate, storage).await {
                                Ok(persisted) => {
                                    if persisted {
                                        if candidate.is_new() {
                                            stats.entities_created += 1;
                                            info!("Created new {}: {}", 
                                                handler.entity_type(),
                                                candidate.changes.change_summary
                                            );
                                        } else {
                                            stats.entities_updated += 1;
                                            info!("Updated {}: {}",
                                                handler.entity_type(),
                                                candidate.changes.change_summary
                                            );
                                        }
                                    }
                                    
                                    // Store process records
                                    for mut record in process_records {
                                        if let Err(e) = storage.create_process_record(&mut record).await {
                                            error!("Failed to store process record: {:?}", e);
                                        } else {
                                            all_process_records.push(record);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to persist {}: {:?}", handler.entity_type(), e);
                                    stats.errors += 1;
                                }
                            }
                        } else {
                            stats.entities_unchanged += 1;
                            debug!("No changes for {}", handler.entity_type());
                        }
                    }
                    Ok(None) => {
                        debug!("No candidate extracted by {} handler", handler.entity_type());
                        stats.entities_skipped += 1;
                    }
                    Err(e) => {
                        error!(
                            "Error preparing candidate with handler {}: {:?}",
                            handler.entity_type(),
                            e
                        );
                        stats.errors += 1;
                    }
                }
            }
        }

        if stats.total_processed() == 0 {
            debug!("No handlers matched record");
        }

        stats.process_records = all_process_records;
        Ok(stats)
    }

    /// Get the number of registered handlers
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Get the entity types of all registered handlers (tests only)
    #[cfg(test)]
    pub fn registered_types(&self) -> Vec<&'static str> {
        self.handlers.iter().map(|h| h.entity_type()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::pipeline::processing::catalog::candidate::CatalogCandidate;

    struct MockHandler {
        entity_type: &'static str,
        can_handle: bool,
    }

    #[async_trait]
    impl EntityHandler for MockHandler {
        fn entity_type(&self) -> &'static str {
            self.entity_type
        }

        fn can_handle(&self, _record: &ConflatedRecord) -> bool {
            self.can_handle
        }

        async fn prepare_candidate(
            &self,
            _record: &ConflatedRecord,
            _storage: &dyn Storage,
        ) -> Result<Option<CatalogCandidate>> {
            Ok(None)
        }

        async fn persist_candidate(
            &self,
            _candidate: &CatalogCandidate,
            _storage: &dyn Storage,
        ) -> Result<bool> {
            Ok(false)
        }

        fn generate_process_records(
            &self,
            _candidate: &CatalogCandidate,
            _process_run: &ProcessRun,
            _timestamp: DateTime<Utc>,
        ) -> Vec<ProcessRecord> {
            vec![]
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = EntityRegistry::new();
        assert_eq!(registry.handler_count(), 0);
    }

    #[test]
    fn test_handler_registration() {
        let mut registry = EntityRegistry::new();
        
        let handler1 = Arc::new(MockHandler {
            entity_type: "TestEntity1",
            can_handle: true,
        });
        
        let handler2 = Arc::new(MockHandler {
            entity_type: "TestEntity2",
            can_handle: false,
        });

        registry.register(handler1);
        registry.register(handler2);

        assert_eq!(registry.handler_count(), 2);
        
        let types = registry.registered_types();
        assert!(types.contains(&"TestEntity1"));
        assert!(types.contains(&"TestEntity2"));
    }
}

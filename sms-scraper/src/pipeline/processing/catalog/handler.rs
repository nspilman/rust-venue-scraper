use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::common::error::Result;
use crate::domain::{ProcessRecord, ProcessRun};
use crate::pipeline::processing::conflation::ConflatedRecord;
use crate::pipeline::storage::Storage;

use super::candidate::CatalogCandidate;

/// Trait that defines how each entity type should be handled in the catalog
/// Handlers own the transformation from ConflatedRecord to PersistedEntity
#[async_trait]
pub trait EntityHandler: Send + Sync {
    /// Returns the entity type name this handler processes
    fn entity_type(&self) -> &'static str;

    /// Checks if this handler can process the given conflated record
    fn can_handle(&self, record: &ConflatedRecord) -> bool;

    /// Extract and prepare a catalog candidate from the conflated record
    /// This is the main transformation: ConflatedRecord -> CatalogCandidate
    async fn prepare_candidate(
        &self,
        record: &ConflatedRecord,
        storage: &dyn Storage,
    ) -> Result<Option<CatalogCandidate>>;

    /// Persist the catalog candidate if it should be saved
    /// Returns true if something was persisted
    async fn persist_candidate(
        &self,
        candidate: &CatalogCandidate,
        storage: &dyn Storage,
    ) -> Result<bool>;

    /// Generate process records for audit trail
    fn generate_process_records(
        &self,
        candidate: &CatalogCandidate,
        process_run: &ProcessRun,
        timestamp: DateTime<Utc>,
    ) -> Vec<ProcessRecord>;
}

use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::common::error::Result;
use crate::domain::ProcessRecord;
use crate::pipeline::storage::Storage;

/// Handles provenance tracking for catalog operations
pub struct ProvenanceTracker {
    storage: Arc<dyn Storage>,
    process_run_id: Option<Uuid>,
}

impl ProvenanceTracker {
    /// Create a new ProvenanceTracker
    pub fn new(storage: Arc<dyn Storage>, process_run_id: Option<Uuid>) -> Self {
        Self {
            storage,
            process_run_id,
        }
    }

    /// Set the current process run ID
    pub fn set_process_run_id(&mut self, run_id: Option<Uuid>) {
        self.process_run_id = run_id;
    }

    /// Emit a ProcessRecord for audit trail
    pub async fn emit_record(
        &self,
        change_type: &str,
        change_log: &str,
        field_changed: &str,
        venue_id: Option<Uuid>,
        event_id: Option<Uuid>,
        artist_id: Option<Uuid>,
    ) -> Result<()> {
        if let Some(run_id) = self.process_run_id {
            let mut record = ProcessRecord {
                id: None,
                process_run_id: run_id,
                api_name: "catalog".to_string(),
                raw_data_id: None, // Could be populated if we track this
                change_type: change_type.to_string(),
                change_log: change_log.to_string(),
                field_changed: field_changed.to_string(),
                event_id,
                venue_id,
                artist_id,
                created_at: Utc::now(),
            };
            
            self.storage.create_process_record(&mut record).await?;
        }
        
        Ok(())
    }

    /// Emit a venue creation record
    pub async fn venue_created(&self, venue_name: &str, venue_id: Option<Uuid>) -> Result<()> {
        self.emit_record(
            "create_venue",
            &format!("Created new venue: {}", venue_name),
            "venue",
            venue_id,
            None,
            None,
        ).await
    }

    /// Emit a venue match record
    pub async fn venue_matched(&self, venue_name: &str, venue_id: Uuid) -> Result<()> {
        self.emit_record(
            "match_venue",
            &format!("Matched existing venue: {}", venue_name),
            "venue",
            Some(venue_id),
            None,
            None,
        ).await
    }

    /// Emit a venue duplicate record
    pub async fn venue_duplicate(&self, venue_name: &str, venue_id: Uuid) -> Result<()> {
        self.emit_record(
            "skip_duplicate_venue",
            &format!("Skipped duplicate venue: {}", venue_name),
            "venue",
            Some(venue_id),
            None,
            None,
        ).await
    }

    /// Emit a venue uncertain match record
    pub async fn venue_uncertain(&self, venue_name: &str) -> Result<()> {
        self.emit_record(
            "uncertain_venue",
            &format!("Uncertain match for venue: {}", venue_name),
            "venue",
            None,
            None,
            None,
        ).await
    }

    /// Emit an event creation record
    pub async fn event_created(&self, event_title: &str, event_id: Option<Uuid>) -> Result<()> {
        self.emit_record(
            "create_event",
            &format!("Created new event: {}", event_title),
            "event",
            None,
            event_id,
            None,
        ).await
    }

    /// Emit an event update record
    pub async fn event_updated(&self, event_title: &str, event_id: Option<Uuid>) -> Result<()> {
        self.emit_record(
            "update_event",
            &format!("Updated event: {}", event_title),
            "event",
            None,
            event_id,
            None,
        ).await
    }

    /// Emit an event duplicate record
    pub async fn event_duplicate(&self, event_title: &str, event_id: Uuid) -> Result<()> {
        self.emit_record(
            "skip_duplicate_event",
            &format!("Skipped duplicate event: {}", event_title),
            "event",
            None,
            Some(event_id),
            None,
        ).await
    }

    /// Emit an event uncertain match record
    pub async fn event_uncertain(&self, event_title: &str) -> Result<()> {
        self.emit_record(
            "uncertain_event",
            &format!("Uncertain match for event: {}", event_title),
            "event",
            None,
            None,
            None,
        ).await
    }

    /// Emit an artist creation record
    pub async fn artist_created(&self, artist_name: &str, artist_id: Option<Uuid>) -> Result<()> {
        self.emit_record(
            "create_artist",
            &format!("Created new artist: {}", artist_name),
            "artist",
            None,
            None,
            artist_id,
        ).await
    }

    /// Emit an artist match record
    pub async fn artist_matched(&self, artist_name: &str, artist_id: Uuid) -> Result<()> {
        self.emit_record(
            "match_artist",
            &format!("Matched existing artist: {}", artist_name),
            "artist",
            None,
            None,
            Some(artist_id),
        ).await
    }

    /// Emit an artist uncertain match record
    pub async fn artist_uncertain(&self, artist_name: &str) -> Result<()> {
        self.emit_record(
            "uncertain_artist",
            &format!("Uncertain match for artist: {}", artist_name),
            "artist",
            None,
            None,
            None,
        ).await
    }
}

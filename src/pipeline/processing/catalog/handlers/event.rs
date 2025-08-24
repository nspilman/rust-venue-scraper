use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::{debug, error};
use uuid::Uuid;

use crate::common::error::Result;
use crate::domain::{Event, ProcessRecord, ProcessRun};
use crate::pipeline::processing::catalog::candidate::{
    CatalogCandidate, ChangeSet, PersistedEntity, ProposedEntity
};
use crate::pipeline::processing::catalog::handler::EntityHandler;
use crate::pipeline::processing::conflation::{ConflatedRecord, EntityType};
use crate::pipeline::processing::normalize::NormalizedEntity;
use crate::pipeline::storage::Storage;

use std::sync::Arc;
use crate::pipeline::processing::catalog::mapper::MapperRegistry;

pub struct EventHandler {
    mappers: Arc<MapperRegistry>,
}

impl EventHandler {
    #[cfg(test)]
    pub fn new() -> Self {
        Self { mappers: Arc::new(MapperRegistry::default()) }
    }

    pub fn with_mappers(mappers: Arc<MapperRegistry>) -> Self {
        Self { mappers }
    }

    /// Detect changes between proposed and current event
    fn detect_event_changes(&self, proposed: &Event, current: &Event) -> ChangeSet {
        let mut changeset = ChangeSet::no_changes();
        
        // Compare each field and track changes
        if proposed.title != current.title {
            changeset.add_change(
                "title",
                Some(current.title.clone()),
                Some(proposed.title.clone())
            );
        }
        
        if proposed.event_day != current.event_day {
            changeset.add_change(
                "event_day",
                Some(current.event_day.to_string()),
                Some(proposed.event_day.to_string())
            );
        }
        
        if proposed.start_time != current.start_time {
            changeset.add_change(
                "start_time",
                current.start_time.map(|t| t.to_string()),
                proposed.start_time.map(|t| t.to_string())
            );
        }
        
        if proposed.venue_id != current.venue_id {
            changeset.add_change(
                "venue_id",
                Some(current.venue_id.to_string()),
                Some(proposed.venue_id.to_string())
            );
        }
        
        if proposed.description != current.description {
            changeset.add_change(
                "description",
                current.description.clone(),
                proposed.description.clone()
            );
        }
        
        if changeset.has_changes {
            changeset.change_summary = format!("Updated event: {}", proposed.title);
        }
        
        changeset
    }

}

#[async_trait]
impl EntityHandler for EventHandler {
    fn entity_type(&self) -> &'static str {
        "Event"
    }

    fn can_handle(&self, record: &ConflatedRecord) -> bool {
        matches!(record.canonical_entity_id.entity_type, EntityType::Event)
    }

    async fn prepare_candidate(
        &self,
        record: &ConflatedRecord,
        storage: &dyn Storage,
    ) -> Result<Option<CatalogCandidate>> {
        // Step 1: Extract event from the normalized entity in the conflated record
        // Build proposed event via mapper. Determine venue_id.
        let venue_id = if let NormalizedEntity::Event(e) = &record.enriched_record.quality_assessed_record.normalized_record.entity {
            // Trust the normalized event's venue_id; if it's nil, we'll persist the event without a hosts edge
            e.venue_id
        } else {
            uuid::Uuid::nil()
        };
        let Ok(proposed_event_base) = self.mappers.event_mapper.to_event(record, venue_id) else {
            debug!("No event found in conflated record");
            return Ok(None);
        };

        // Step 2: Prepare the event for persistence - the normalized event is already the correct domain struct
        let mut proposed_event = proposed_event_base.clone();
        proposed_event.id = Some(record.canonical_entity_id.id);
        // Ensure defaults that the mapper may not set for this persistence step
        proposed_event.show_event = true;
        proposed_event.finalized = false;
        proposed_event.created_at = Utc::now();
        let proposed_entity = ProposedEntity::Event(proposed_event.clone());

        // Step 3: Check if event already exists
        match storage.get_event_by_venue_date_title(
            proposed_event.venue_id,
            proposed_event.event_day,
            &proposed_event.title
        ).await {
            Ok(Some(existing_event)) => {
                // Event exists - check for changes
                let changes = self.detect_event_changes(&proposed_event, &existing_event);
let current_entity = PersistedEntity::Event;
                
                Ok(Some(CatalogCandidate::existing_entity(
                    EntityType::Event,
                    record.canonical_entity_id.clone(),
                    proposed_entity,
                    current_entity,
                    changes,
                )))
            }
            Ok(None) => {
                // New event
                Ok(Some(CatalogCandidate::new_entity(
                    EntityType::Event,
                    record.canonical_entity_id.clone(),
                    proposed_entity,
                )))
            }
            Err(e) => {
                error!("Error looking up existing event: {:?}", e);
                Err(e)
            }
        }
    }

    async fn persist_candidate(
        &self,
        candidate: &CatalogCandidate,
        storage: &dyn Storage,
    ) -> Result<bool> {
        if !candidate.should_persist {
            return Ok(false);
        }

        let ProposedEntity::Event(event) = &candidate.proposed_state else {
            error!("Expected event in proposed state");
            return Ok(false);
        };

        if candidate.is_new() {
            // Create new event
            let mut event = event.clone();
            storage.create_event(&mut event).await?;
            debug!("Created new event: {}", event.title);
            Ok(true)
        } else {
            // Update existing event
            storage.update_event(event).await?;
            debug!("Updated event: {}", event.title);
            Ok(true)
        }
    }

    fn generate_process_records(
        &self,
        candidate: &CatalogCandidate,
        process_run: &ProcessRun,
        timestamp: DateTime<Utc>,
    ) -> Vec<ProcessRecord> {
        let ProposedEntity::Event(event) = &candidate.proposed_state else {
            return vec![];
        };

        let process_run_id = process_run.id.unwrap_or_else(|| Uuid::new_v4());
        let event_id = event.id.unwrap_or_else(|| Uuid::new_v4());

        let (change_type, change_log, field_changed) = if candidate.is_new() {
            (
                "CREATE".to_string(),
                format!("Created new event: {}", event.title),
                "all".to_string(),
            )
        } else if candidate.has_changes() {
            let field_names: Vec<String> = candidate.changes.changed_fields
                .iter()
                .map(|f| f.field_name.clone())
                .collect();
            (
                "UPDATE".to_string(),
                candidate.changes.summarize(),
                field_names.join(", "),
            )
        } else {
            (
                "NO_CHANGE".to_string(),
                "No changes detected".to_string(),
                "none".to_string(),
            )
        };

        vec![ProcessRecord {
            id: Some(Uuid::new_v4()),
            process_run_id,
            api_name: "catalog".to_string(),
            raw_data_id: None,
            change_type,
            change_log,
            field_changed,
            event_id: Some(event_id),
            venue_id: Some(event.venue_id),
            artist_id: None,
            created_at: timestamp,
        }]
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_detect_event_changes() {
        let handler = EventHandler::new();
        let event_day = NaiveDate::from_ymd_opt(2025, 8, 15).unwrap();
        let start_time = chrono::NaiveTime::from_hms_opt(20, 0, 0);
        
        let event1 = Event {
            id: Some(uuid::Uuid::new_v4()),
            title: "Test Concert".to_string(),
            event_day,
            start_time,
            event_url: Some("https://example.com/event".to_string()),
            description: Some("A great concert".to_string()),
            event_image_url: None,
            venue_id: uuid::Uuid::new_v4(),
            artist_ids: vec![],
            show_event: true,
            finalized: false,
            created_at: Utc::now(),
        };
        
        let mut event2 = event1.clone();
        let changes = handler.detect_event_changes(&event1, &event2);
        assert!(!changes.has_changes);
        event2.title = "Updated Concert".to_string();
        let changes = handler.detect_event_changes(&event2, &event1);
        assert!(changes.has_changes);
    }
}

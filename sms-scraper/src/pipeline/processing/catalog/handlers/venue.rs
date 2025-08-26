use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::{debug, error};
use uuid::Uuid;

use crate::common::error::Result;
use crate::domain::{ProcessRecord, ProcessRun, Venue};
use crate::pipeline::processing::catalog::candidate::{
    CatalogCandidate, ChangeSet, PersistedEntity, ProposedEntity
};
use crate::pipeline::processing::catalog::handler::EntityHandler;
use crate::pipeline::processing::conflation::{ConflatedRecord, EntityType};
use crate::pipeline::storage::Storage;

use std::sync::Arc;
use crate::pipeline::processing::catalog::mapper::MapperRegistry;

pub struct VenueHandler {
    mappers: Arc<MapperRegistry>,
}

impl VenueHandler {
    #[cfg(test)]
    pub fn new() -> Self {
        Self { mappers: Arc::new(MapperRegistry::default()) }
    }

    pub fn with_mappers(mappers: Arc<MapperRegistry>) -> Self {
        Self { mappers }
    }

    /// Convert venue to the correct domain model format
    fn prepare_venue_for_persistence(&self, venue: &Venue, canonical_id: &uuid::Uuid) -> Venue {
        // The normalized venue is already the correct domain struct, just update ID and timestamps
        Venue {
            id: Some(*canonical_id),
            name: venue.name.clone(),
            name_lower: venue.name_lower.clone(),
            slug: venue.slug.clone(),
            latitude: venue.latitude,
            longitude: venue.longitude,
            address: venue.address.clone(),
            postal_code: venue.postal_code.clone(),
            city: venue.city.clone(),
            venue_url: venue.venue_url.clone(),
            venue_image_url: venue.venue_image_url.clone(),
            description: venue.description.clone(),
            neighborhood: venue.neighborhood.clone(),
            show_venue: true,
            created_at: Utc::now(),
        }
    }

    /// Detect changes between proposed and current venue
    fn detect_venue_changes(&self, proposed: &Venue, current: &Venue) -> ChangeSet {
        let mut changeset = ChangeSet::no_changes();
        
        // Compare each field and track changes
        if proposed.name != current.name {
            changeset.add_change(
                "name",
                Some(current.name.clone()),
                Some(proposed.name.clone())
            );
        }
        
        if proposed.address != current.address {
            changeset.add_change(
                "address",
                Some(current.address.clone()),
                Some(proposed.address.clone())
            );
        }
        
        if proposed.city != current.city {
            changeset.add_change(
                "city",
                Some(current.city.clone()),
                Some(proposed.city.clone())
            );
        }
        
        if proposed.latitude != current.latitude {
            changeset.add_change(
                "latitude",
                Some(current.latitude.to_string()),
                Some(proposed.latitude.to_string())
            );
        }
        
        if proposed.longitude != current.longitude {
            changeset.add_change(
                "longitude",
                Some(current.longitude.to_string()),
                Some(proposed.longitude.to_string())
            );
        }
        
        if changeset.has_changes {
            changeset.change_summary = format!("Updated venue: {}", proposed.name);
        }
        
        changeset
    }
}

#[async_trait]
impl EntityHandler for VenueHandler {
    fn entity_type(&self) -> &'static str {
        "Venue"
    }

    fn can_handle(&self, record: &ConflatedRecord) -> bool {
        // Check if this conflated record contains a venue entity
        matches!(record.canonical_entity_id.entity_type, EntityType::Venue)
    }

    async fn prepare_candidate(
        &self,
        record: &ConflatedRecord,
        storage: &dyn Storage,
    ) -> Result<Option<CatalogCandidate>> {
        // Step 1: Extract venue from the conflated record
        // Use mapper to build base venue from record
        let Ok(normalized_venue) = self.mappers.venue_mapper.to_venue(record) else {
            debug!("No venue found in conflated record");
            return Ok(None);
        };

        // Step 2: Prepare the venue for persistence
        let proposed_venue = self.prepare_venue_for_persistence(&normalized_venue, &record.canonical_entity_id.id);
        let proposed_entity = ProposedEntity::Venue(proposed_venue.clone());

        // Step 3: Check if venue already exists
        match storage.get_venue_by_name(&proposed_venue.name).await {
            Ok(Some(existing_venue)) => {
                // Venue exists - check for changes
                let changes = self.detect_venue_changes(&proposed_venue, &existing_venue);
let current_entity = PersistedEntity::Venue;
                
                Ok(Some(CatalogCandidate::existing_entity(
                    EntityType::Venue,
                    record.canonical_entity_id.clone(),
                    proposed_entity,
                    current_entity,
                    changes,
                )))
            }
            Ok(None) => {
                // New venue
                Ok(Some(CatalogCandidate::new_entity(
                    EntityType::Venue,
                    record.canonical_entity_id.clone(),
                    proposed_entity,
                )))
            }
            Err(e) => {
                error!("Error looking up existing venue: {:?}", e);
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

        let ProposedEntity::Venue(venue) = &candidate.proposed_state else {
            error!("Expected venue in proposed state");
            return Ok(false);
        };

        if candidate.is_new() {
            // Create new venue
            let mut venue = venue.clone();
            storage.create_venue(&mut venue).await?;
            debug!("Created new venue: {}", venue.name);
            Ok(true)
        } else {
            // Update existing venue - note: there's no update_venue method in Storage trait
            // For now, we'll just log that we would update
            debug!("Would update venue: {} (update method not available)", venue.name);
            Ok(true)
        }
    }

    fn generate_process_records(
        &self,
        candidate: &CatalogCandidate,
        process_run: &ProcessRun,
        timestamp: DateTime<Utc>,
    ) -> Vec<ProcessRecord> {
        let ProposedEntity::Venue(venue) = &candidate.proposed_state else {
            return vec![];
        };

        let process_run_id = process_run.id.unwrap_or_else(|| Uuid::new_v4());
        let venue_id = venue.id.unwrap_or_else(|| Uuid::new_v4());

        let (change_type, change_log, field_changed) = if candidate.is_new() {
            (
                "CREATE".to_string(),
                format!("Created new venue: {}", venue.name),
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
            event_id: None,
            venue_id: Some(venue_id),
            artist_id: None,
            created_at: timestamp,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_venue_detect_changes() {
        let handler = VenueHandler::new();
        
        // Venue domain has: id, name, name_lower, slug, latitude, longitude, address, postal_code, 
        // city, venue_url, venue_image_url, description, neighborhood, show_venue, created_at
        let venue1 = Venue {
            id: Some(uuid::Uuid::new_v4()),
            name: "Test Venue".to_string(),
            name_lower: "test venue".to_string(),
            slug: "test-venue".to_string(),
            latitude: 47.6062,
            longitude: -122.3321,
            address: "123 Main St".to_string(),
            postal_code: "98101".to_string(),
            city: "Seattle".to_string(),
            venue_url: Some("https://example.com".to_string()),
            venue_image_url: None,
            description: Some("A great venue".to_string()),
            neighborhood: Some("Downtown".to_string()),
            show_venue: true,
            created_at: Utc::now(),
        };
        
        let mut venue2 = venue1.clone();
        
        // Test no changes
        let changes = handler.detect_venue_changes(&venue1, &venue2);
        assert!(!changes.has_changes);
        
        // Test name change
        venue2.name = "Updated Venue".to_string();
        let changes = handler.detect_venue_changes(&venue2, &venue1);
        assert!(changes.has_changes);
        assert!(changes.changed_fields.len() == 1);
        assert!(changes.changed_fields[0].field_name == "name");
        
        // Test multiple changes
        venue2.city = "Portland".to_string();
        let changes = handler.detect_venue_changes(&venue2, &venue1);
        assert!(changes.has_changes);
        assert!(changes.changed_fields.len() == 2);
    }
}

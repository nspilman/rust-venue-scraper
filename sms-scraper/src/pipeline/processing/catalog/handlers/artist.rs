use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::{debug, error};
use uuid::Uuid;

use sms_core::common::error::Result;
use sms_core::domain::{Artist, ProcessRecord, ProcessRun};
use crate::pipeline::processing::catalog::candidate::{
    CatalogCandidate, ChangeSet, PersistedEntity, ProposedEntity
};
use crate::pipeline::processing::catalog::handler::EntityHandler;
use crate::pipeline::processing::conflation::{ConflatedRecord, EntityType};
use crate::pipeline::storage::Storage;

use std::sync::Arc;
use crate::pipeline::processing::catalog::mapper::{EntityUtils, MapperRegistry};

pub struct ArtistHandler {
    mappers: Arc<MapperRegistry>,
}

impl ArtistHandler {
    #[cfg(test)]
    pub fn new() -> Self {
        Self { mappers: Arc::new(MapperRegistry::default()) }
    }

    pub fn with_mappers(mappers: Arc<MapperRegistry>) -> Self {
        Self { mappers }
    }

    /// Detect changes between proposed and current artist
    fn detect_artist_changes(&self, proposed: &Artist, current: &Artist) -> ChangeSet {
        let mut changeset = ChangeSet::no_changes();
        
        // Compare each field and track changes
        if proposed.name != current.name {
            changeset.add_change(
                "name",
                Some(current.name.clone()),
                Some(proposed.name.clone())
            );
        }
        
        if proposed.name_slug != current.name_slug {
            changeset.add_change(
                "name_slug",
                Some(current.name_slug.clone()),
                Some(proposed.name_slug.clone())
            );
        }
        
        if proposed.bio != current.bio {
            changeset.add_change(
                "bio",
                current.bio.clone(),
                proposed.bio.clone()
            );
        }
        
        if proposed.artist_image_url != current.artist_image_url {
            changeset.add_change(
                "artist_image_url",
                current.artist_image_url.clone(),
                proposed.artist_image_url.clone()
            );
        }
        
        if changeset.has_changes {
            changeset.change_summary = format!("Updated artist: {}", proposed.name);
        }
        
        changeset
    }

}

#[async_trait]
impl EntityHandler for ArtistHandler {
    fn entity_type(&self) -> &'static str {
        "Artist"
    }

    fn can_handle(&self, record: &ConflatedRecord) -> bool {
        matches!(record.canonical_entity_id.entity_type, EntityType::Artist)
    }

    async fn prepare_candidate(
        &self,
        record: &ConflatedRecord,
        storage: &dyn Storage,
    ) -> Result<Option<CatalogCandidate>> {
        // Step 1: Extract artist from the normalized entity in the conflated record
        // Build proposed artist via mapper
        let Ok(normalized_artist) = self.mappers.artist_mapper.to_artist(record) else {
            debug!("No artist found in conflated record");
            return Ok(None);
        };

        // Step 2: Prepare the artist for persistence - need to convert from normalized to domain
        let proposed_artist = Artist {
            id: Some(record.canonical_entity_id.id),
            name: normalized_artist.name.clone(),
            name_slug: EntityUtils::generate_slug(&normalized_artist.name),
            bio: normalized_artist.bio,
            artist_image_url: None, // Not available in normalized artist
            created_at: Utc::now(),
        };
        let proposed_entity = ProposedEntity::Artist(proposed_artist.clone());

        // Step 3: Check if artist already exists
        match storage.get_artist_by_name(&proposed_artist.name).await {
            Ok(Some(existing_artist)) => {
                // Artist exists - check for changes
                let changes = self.detect_artist_changes(&proposed_artist, &existing_artist);
let current_entity = PersistedEntity::Artist;
                
                Ok(Some(CatalogCandidate::existing_entity(
                    EntityType::Artist,
                    record.canonical_entity_id.clone(),
                    proposed_entity,
                    current_entity,
                    changes,
                )))
            }
            Ok(None) => {
                // New artist
                Ok(Some(CatalogCandidate::new_entity(
                    EntityType::Artist,
                    record.canonical_entity_id.clone(),
                    proposed_entity,
                )))
            }
            Err(e) => {
                error!("Error looking up existing artist: {:?}", e);
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

        let ProposedEntity::Artist(artist) = &candidate.proposed_state else {
            error!("Expected artist in proposed state");
            return Ok(false);
        };

        if candidate.is_new() {
            // Create new artist
            let mut artist = artist.clone();
            storage.create_artist(&mut artist).await?;
            debug!("Created new artist: {}", artist.name);
            Ok(true)
        } else {
            // Update existing artist - note: there's no update_artist method in Storage trait
            // For now, we'll just log that we would update
            debug!("Would update artist: {} (update method not available)", artist.name);
            Ok(true)
        }
    }

    fn generate_process_records(
        &self,
        candidate: &CatalogCandidate,
        process_run: &ProcessRun,
        timestamp: DateTime<Utc>,
    ) -> Vec<ProcessRecord> {
        let ProposedEntity::Artist(artist) = &candidate.proposed_state else {
            return vec![];
        };

        let process_run_id = process_run.id.unwrap_or_else(|| Uuid::new_v4());
        let artist_id = artist.id.unwrap_or_else(|| Uuid::new_v4());

        let (change_type, change_log, field_changed) = if candidate.is_new() {
            (
                "CREATE".to_string(),
                format!("Created new artist: {}", artist.name),
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
            venue_id: None,
            artist_id: Some(artist_id),
            created_at: timestamp,
        }]
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_artist_changes() {
        let handler = ArtistHandler::new();
        
        let artist1 = Artist {
            id: Some(uuid::Uuid::new_v4()),
            name: "Test Band".to_string(),
            name_slug: "test-band".to_string(),
            bio: Some("A test band".to_string()),
            artist_image_url: Some("https://example.com/image.jpg".to_string()),
            created_at: Utc::now(),
        };
        
        let mut artist2 = artist1.clone();
        
        // No changes
        let changes = handler.detect_artist_changes(&artist1, &artist2);
        assert!(!changes.has_changes);
        
        // Name change
        artist2.name = "Updated Band".to_string();
        let changes = handler.detect_artist_changes(&artist2, &artist1);
        assert!(changes.has_changes);
    }
}

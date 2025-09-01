use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, debug, error};
use sms_core::storage::Storage;
use sms_core::domain::{Event, Venue, Artist};
use uuid::Uuid;
use chrono;
use super::{PipelineStep, StepResult};

/// Pipeline step for storing entities in graph database
pub struct CatalogStep {
    validate_graph: bool,
}

impl CatalogStep {
    pub fn new(validate_graph: bool) -> Self {
        Self { validate_graph }
    }
}

#[async_trait]
impl PipelineStep for CatalogStep {
    async fn execute(&self, source_id: &str, storage: &dyn Storage) -> Result<StepResult> {
        info!("📚 Running catalog step for source: {} (validate: {})", source_id, self.validate_graph);
        
        // 1. Get all processed raw data for this source (parsed events)
        let internal_api_name = crate::common::constants::api_name_to_internal(source_id);
        let processed_raw_data = storage.get_processed_raw_data(&internal_api_name, None).await
            .map_err(|e| anyhow::anyhow!("Failed to get processed raw data for source {}: {}", source_id, e))?;
        
        debug!("Found {} processed raw data items for source {}", processed_raw_data.len(), source_id);
        
        let mut created_events = 0;
        let mut created_venues = 0;
        let mut created_artists = 0;
        let mut errors = 0;
        
        // 2. Process each raw data item and create events if not already created
        for raw_data in processed_raw_data {
            // Skip if event already exists (check by event_id in raw_data)
            if raw_data.event_id.is_some() {
                debug!("Event for raw data {} already cataloged, skipping", raw_data.event_name);
                continue;
            }
            
            // Create or get venue
            let (venue_id, venue_created) = match self.ensure_venue_exists(&raw_data.venue_name, storage).await {
                Ok((id, created)) => {
                    if created {
                        created_venues += 1;
                    }
                    (id, created)
                },
                Err(e) => {
                    error!("Failed to create/get venue {}: {}", raw_data.venue_name, e);
                    errors += 1;
                    continue;
                }
            };
            
            // For now, assume single artist from event name parsing
            // In a full implementation, this would parse artist names from the raw data
            let artist_name = &raw_data.event_name; // Simplified - would need proper artist extraction
            let (artist_id, artist_created) = match self.ensure_artist_exists(artist_name, storage).await {
                Ok((id, created)) => {
                    if created {
                        created_artists += 1;
                    }
                    (id, created)
                },
                Err(e) => {
                    error!("Failed to create/get artist {}: {}", artist_name, e);
                    errors += 1;
                    continue;
                }
            };
            
            // Check if event already exists (by title + venue_id + date)
            let existing_event = storage.get_event_by_venue_date_title(venue_id, raw_data.event_day, &raw_data.event_name).await;
            
            match existing_event {
                Ok(Some(_)) => {
                    debug!("Event '{}' at {} on {:?} already exists, skipping", raw_data.event_name, raw_data.venue_name, raw_data.event_day);
                    continue;
                },
                Ok(None) => {
                    // Event doesn't exist, create it
                    let mut event = Event {
                        id: None,
                        title: raw_data.event_name.clone(),
                        event_day: raw_data.event_day,
                        start_time: None, // Would be parsed from raw data
                        event_url: None,
                        description: None,
                        event_image_url: None,
                        venue_id,
                        artist_ids: vec![artist_id],
                        show_event: true,
                        finalized: true,
                        created_at: chrono::Utc::now(),
                    };
                    
                    // Create the event in the graph database
                    match storage.create_event(&mut event).await {
                        Ok(_) => {
                            created_events += 1;
                            debug!("Created event: {} at {}", event.title, raw_data.venue_name);
                        }
                        Err(e) => {
                            error!("Failed to create event {}: {}", event.title, e);
                            errors += 1;
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to check if event exists {}: {}", raw_data.event_name, e);
                    errors += 1;
                    continue;
                }
            }
        }
        
        // 3. Optionally validate graph integrity
        if self.validate_graph {
            debug!("Validating graph integrity for source {}", source_id);
            // TODO: Add graph validation logic
        }
        
        let message = format!(
            "Catalog completed for {}: {} events, {} venues, {} artists created ({} errors)",
            source_id, created_events, created_venues, created_artists, errors
        );
        info!("✅ {}", message);
        
        Ok(StepResult::success(created_events, message))
    }
    
    fn step_name(&self) -> &'static str {
        "catalog"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["conflation"]
    }
}

impl CatalogStep {
    /// Ensure a venue exists in the database, creating it if necessary
    /// Returns (venue_id, was_created)
    async fn ensure_venue_exists(&self, venue_name: &str, storage: &dyn Storage) -> Result<(Uuid, bool)> {
        // Try to find existing venue by name
        if let Some(existing_venue) = storage.get_venue_by_name(venue_name).await? {
            if let Some(id) = existing_venue.id {
                return Ok((id, false)); // Found existing venue
            }
        }
        
        // Create new venue with correct fields
        let mut venue = Venue {
            id: None,
            name: venue_name.to_string(),
            name_lower: venue_name.to_lowercase(),
            slug: venue_name.to_lowercase().replace(' ', "-"),
            latitude: 47.6062, // Default Seattle coordinates
            longitude: -122.3321,
            address: "Address TBD".to_string(),
            postal_code: "98101".to_string(),
            city: "Seattle".to_string(),
            venue_url: None,
            venue_image_url: None,
            description: None,
            neighborhood: None,
            show_venue: true,
            created_at: chrono::Utc::now(),
        };
        
        storage.create_venue(&mut venue).await?;
        Ok((venue.id.unwrap(), true)) // Created new venue
    }
    
    /// Ensure an artist exists in the database, creating it if necessary
    /// Returns (artist_id, was_created)
    async fn ensure_artist_exists(&self, artist_name: &str, storage: &dyn Storage) -> Result<(Uuid, bool)> {
        // Try to find existing artist by name
        if let Some(existing_artist) = storage.get_artist_by_name(artist_name).await? {
            if let Some(id) = existing_artist.id {
                return Ok((id, false)); // Found existing artist
            }
        }
        
        // Create new artist with correct fields
        let mut artist = Artist {
            id: None,
            name: artist_name.to_string(),
            name_slug: artist_name.to_lowercase().replace(' ', "-"),
            bio: None,
            artist_image_url: None,
            created_at: chrono::Utc::now(),
        };
        
        storage.create_artist(&mut artist).await?;
        Ok((artist.id.unwrap(), true)) // Created new artist
    }
}

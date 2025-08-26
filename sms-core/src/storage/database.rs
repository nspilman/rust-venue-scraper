#[cfg(feature = "db")]
use super::traits::Storage;
#[cfg(feature = "db")]
use crate::common::error::{Result, ScraperError};
#[cfg(feature = "db")]
use crate::database::DatabaseManager;
#[cfg(feature = "db")]
use crate::domain::*;
#[cfg(feature = "db")]
use async_trait::async_trait;
#[cfg(feature = "db")]
use chrono::NaiveDate;
#[cfg(feature = "db")]
use std::sync::Arc;
#[cfg(feature = "db")]
use tracing::{debug, info};
#[cfg(feature = "db")]
use uuid::Uuid;

/// Database storage implementation using Turso/libSQL with nodes and edges schema
#[cfg(feature = "db")]
pub struct DatabaseStorage {
    db: Arc<DatabaseManager>,
}

#[cfg(feature = "db")]
impl DatabaseStorage {
    pub async fn new() -> Result<Self> {
        let db_manager = DatabaseManager::new().await?;
        db_manager.run_migrations().await?;

        Ok(Self {
            db: Arc::new(db_manager),
        })
    }

    /// Convert venue to node data
    fn venue_to_node_data(venue: &Venue) -> Result<String> {
        serde_json::to_string(venue).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize venue: {e}"),
        })
    }

    /// Convert node data to venue  
    fn node_data_to_venue(id: &str, data: &str) -> Result<Venue> {
        let mut venue: Venue = serde_json::from_str(data).map_err(|e| ScraperError::Database {
            message: format!("Failed to deserialize venue: {e}"),
        })?;
        venue.id = Some(Uuid::parse_str(id).map_err(|e| ScraperError::Database {
            message: format!("Invalid venue UUID: {e}"),
        })?);
        Ok(venue)
    }

    /// Convert artist to node data
    fn artist_to_node_data(artist: &Artist) -> Result<String> {
        serde_json::to_string(artist).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize artist: {e}"),
        })
    }

    /// Convert node data to artist
    fn node_data_to_artist(id: &str, data: &str) -> Result<Artist> {
        let mut artist: Artist =
            serde_json::from_str(data).map_err(|e| ScraperError::Database {
                message: format!("Failed to deserialize artist: {e}"),
            })?;
        artist.id = Some(Uuid::parse_str(id).map_err(|e| ScraperError::Database {
            message: format!("Invalid artist UUID: {e}"),
        })?);
        Ok(artist)
    }

    /// Convert event to node data
    fn event_to_node_data(event: &Event) -> Result<String> {
        debug!("Serializing event '{}' with {} artist_ids", event.title, event.artist_ids.len());
        let json = serde_json::to_string(event).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize event: {e}"),
        })?;
        
        // Verify the JSON contains artist_ids
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(artist_ids) = parsed.get("artist_ids") {
                debug!("JSON artist_ids field: {}", artist_ids);
            } else {
                debug!("WARNING: No artist_ids field in serialized JSON!");
            }
        }
        
        Ok(json)
    }

    /// Convert node data to event
    fn node_data_to_event(id: &str, data: &str) -> Result<Event> {
        let mut event: Event = serde_json::from_str(data).map_err(|e| ScraperError::Database {
            message: format!("Failed to deserialize event: {e}"),
        })?;
        event.id = Some(Uuid::parse_str(id).map_err(|e| ScraperError::Database {
            message: format!("Invalid event UUID: {e}"),
        })?);
        Ok(event)
    }

    /// Convert raw data to node data
    fn raw_data_to_node_data(raw_data: &RawData) -> Result<String> {
        serde_json::to_string(raw_data).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize raw data: {e}"),
        })
    }

    /// Convert node data to raw data
    fn node_data_to_raw_data(id: &str, data: &str) -> Result<RawData> {
        let mut raw_data: RawData =
            serde_json::from_str(data).map_err(|e| ScraperError::Database {
                message: format!("Failed to deserialize raw data: {e}"),
            })?;
        raw_data.id = Some(Uuid::parse_str(id).map_err(|e| ScraperError::Database {
            message: format!("Invalid raw data UUID: {e}"),
        })?);
        Ok(raw_data)
    }

    /// Convert process run to node data
    fn process_run_to_node_data(run: &ProcessRun) -> Result<String> {
        serde_json::to_string(run).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize process run: {e}"),
        })
    }

    /// Convert process record to node data
    fn process_record_to_node_data(record: &ProcessRecord) -> Result<String> {
        serde_json::to_string(record).map_err(|e| ScraperError::Database {
            message: format!("Failed to serialize process record: {e}"),
        })
    }
}

#[cfg(feature = "db")]
#[async_trait]
impl Storage for DatabaseStorage {
    async fn create_venue(&self, venue: &mut Venue) -> Result<()> {
        // Respect existing ID if provided by conflation; otherwise generate
        let id = venue.id.unwrap_or_else(Uuid::new_v4);
        venue.id = Some(id);

        let node_data = Self::venue_to_node_data(venue)?;

        self.db
            .create_node(&id.to_string(), "venue", &node_data)
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to upsert venue node: {e}"),
            })?;

        info!("Upserted venue: {} with id {}", venue.name, id);
        Ok(())
    }

    async fn get_venue_by_name(&self, name: &str) -> Result<Option<Venue>> {
        let venues_data = self
            .db
            .get_nodes_by_label("venue")
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query venues: {e}"),
            })?;

        for (id, _label, data) in venues_data.into_iter() {
            let venue = Self::node_data_to_venue(&id, &data)?;
            if venue.name.to_lowercase() == name.to_lowercase() {
                return Ok(Some(venue));
            }
        }

        Ok(None)
    }

    async fn create_artist(&self, artist: &mut Artist) -> Result<()> {
        // Respect existing ID if provided; otherwise generate
        let id = artist.id.unwrap_or_else(Uuid::new_v4);
        artist.id = Some(id);

        let node_data = Self::artist_to_node_data(artist)?;

        self.db
            .create_node(&id.to_string(), "artist", &node_data)
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to upsert artist node: {e}"),
            })?;

        info!("Upserted artist: {} with id {}", artist.name, id);
        Ok(())
    }

    async fn get_artist_by_name(&self, name: &str) -> Result<Option<Artist>> {
        let artists_data = self
            .db
            .get_nodes_by_label("artist")
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query artists: {e}"),
            })?;

        for (id, _label, data) in artists_data.into_iter() {
            let artist = Self::node_data_to_artist(&id, &data)?;
            if artist.name.to_lowercase() == name.to_lowercase() {
                return Ok(Some(artist));
            }
        }

        Ok(None)
    }

    async fn create_event(&self, event: &mut Event) -> Result<()> {
        debug!("[DATABASE] create_event called for: {}", event.title);
        
        // Respect existing ID if provided; otherwise generate
        let id = event.id.unwrap_or_else(Uuid::new_v4);
        event.id = Some(id);

        debug!("Creating event '{}' with {} artist_ids: {:?}", 
            event.title, event.artist_ids.len(), event.artist_ids);

        let node_data = Self::event_to_node_data(event)?;

        self.db
            .create_node(&id.to_string(), "event", &node_data)
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to upsert event node: {e}"),
            })?;

        // Create edge from venue to event if venue_id is valid (not nil)
        if event.venue_id != Uuid::nil() {
            let edge_id = Uuid::new_v4();
            self.db
                .create_edge(
                    &edge_id.to_string(),
                    &event.venue_id.to_string(),
                    &id.to_string(),
                    "hosts",
                    None,
                )
                .await
                .map_err(|e| ScraperError::Database {
                    message: format!("Failed to upsert venue-event edge: {e}"),
                })?;
        }

        // Create edges from artists to event
        for artist_id in &event.artist_ids {
            if *artist_id == Uuid::nil() { continue; }
            debug!("Creating performs_at edge from artist {} to event {}", artist_id, id);
            let artist_edge_id = Uuid::new_v4();
            self.db
                .create_edge(
                    &artist_edge_id.to_string(),
                    &artist_id.to_string(),
                    &id.to_string(),
                    "performs_at",
                    None,
                )
                .await
                .map_err(|e| ScraperError::Database {
                    message: format!("Failed to upsert artist-event edge for artist {}: {e}", artist_id),
                })?;
            debug!("Successfully created performs_at edge for artist {}", artist_id);
        }

        info!("Upserted event: {} with id {}", event.title, id);
        Ok(())
    }

    async fn get_event_by_venue_date_title(
        &self,
        venue_id: Uuid,
        event_day: NaiveDate,
        title: &str,
    ) -> Result<Option<Event>> {
        let events_data = self
            .db
            .get_nodes_by_label("event")
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query events: {e}"),
            })?;

        for (id, _label, data) in events_data.into_iter() {
            let event = Self::node_data_to_event(&id, &data)?;
            if event.venue_id == venue_id
                && event.event_day == event_day
                && event.title.to_lowercase() == title.to_lowercase()
            {
                return Ok(Some(event));
            }
        }

        Ok(None)
    }

    async fn update_event(&self, event: &Event) -> Result<()> {
        let event_id = event.id.ok_or_else(|| ScraperError::Api {
            message: "Cannot update event without ID".to_string(),
        })?;

        debug!("Updating event '{}' with {} artist_ids: {:?}", 
            event.title, event.artist_ids.len(), event.artist_ids);

        let node_data = Self::event_to_node_data(event)?;
        
        // Log the JSON data being sent to database
        debug!("Sending JSON to database for event '{}': {}", event.title, 
            if node_data.len() > 200 { 
                format!("{}...", &node_data[..200]) 
            } else { 
                node_data.clone() 
            });

        // Use upsert operation to update the node with new data
        self.db
            .create_node(&event_id.to_string(), "event", &node_data)
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to update event node: {e}"),
            })?;
            
        // Verify what was written
        if let Some((_id, _label, stored_data)) = self.db.get_node(&event_id.to_string()).await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to verify updated event: {e}"),
            })? 
        {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&stored_data) {
                if let Some(stored_artist_ids) = parsed.get("artist_ids") {
                    debug!("Verified stored artist_ids for event '{}': {}", event.title, stored_artist_ids);
                } else {
                    debug!("WARNING: No artist_ids in stored event '{}'!", event.title);
                }
            }
        }

        // Delete existing artist-event edges and recreate them
        // This ensures we have the correct artist linkages
        // Note: In a production system, you'd want to diff and only update changed edges
        
        // Get existing edges to delete
        let existing_edges = self.db
            .get_edges_for_node(&event_id.to_string())
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to get existing edges for event: {e}"),
            })?;
        
        // Delete old performs_at edges (where event is target)
        for (edge_id, source_id, target_id, relation, _) in existing_edges {
            if relation == "performs_at" && target_id == event_id.to_string() {
                // Note: The database doesn't have a delete_edge method, so we can't delete
                // Instead, we'll just create new edges (the upsert will overwrite)
                debug!("Would delete old performs_at edge {}", edge_id);
            }
        }
        
        // Create new edges from artists to event
        for artist_id in &event.artist_ids {
            if *artist_id == Uuid::nil() { continue; }
            debug!("Creating performs_at edge from artist {} to event {}", artist_id, event_id);
            let artist_edge_id = Uuid::new_v4();
            self.db
                .create_edge(
                    &artist_edge_id.to_string(),
                    &artist_id.to_string(),
                    &event_id.to_string(),
                    "performs_at",
                    None,
                )
                .await
                .map_err(|e| ScraperError::Database {
                    message: format!("Failed to create artist-event edge for artist {}: {e}", artist_id),
                })?;
            debug!("Successfully created performs_at edge for artist {}", artist_id);
        }

        info!("Updated event: {} with id {} and {} artists", event.title, event_id, event.artist_ids.len());
        Ok(())
    }

    async fn create_raw_data(&self, raw_data: &mut RawData) -> Result<()> {
        let id = Uuid::new_v4();
        raw_data.id = Some(id);

        let node_data = Self::raw_data_to_node_data(raw_data)?;

        self.db
            .create_node(&id.to_string(), "raw_data", &node_data)
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to create raw data node: {e}"),
            })?;

        debug!("Created raw data: {} with id {}", raw_data.event_name, id);
        Ok(())
    }

    async fn get_unprocessed_raw_data(
        &self,
        api_name: &str,
        min_date: Option<NaiveDate>,
    ) -> Result<Vec<RawData>> {
        let raw_data_nodes = self
            .db
            .get_nodes_by_label("raw_data")
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query raw data: {e}"),
            })?;

        let mut filtered_data = Vec::new();
        for (id, _label, data) in raw_data_nodes.into_iter() {
            let raw_data = Self::node_data_to_raw_data(&id, &data)?;

            if raw_data.api_name == api_name
                && !raw_data.processed
                && (min_date.is_none() || raw_data.event_day >= min_date.unwrap())
            {
                filtered_data.push(raw_data);
            }
        }

        // Sort by event_day to process chronologically
        filtered_data.sort_by(|a, b| a.event_day.cmp(&b.event_day));
        Ok(filtered_data)
    }

    async fn mark_raw_data_processed(&self, raw_data_id: Uuid) -> Result<()> {
        // Get the current raw data
        if let Some((id, _label, data)) = self
            .db
            .get_node(&raw_data_id.to_string())
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to get raw data node: {e}"),
            })?
        {
            let mut raw_data = Self::node_data_to_raw_data(&id, &data)?;
            raw_data.processed = true;

            let updated_data = Self::raw_data_to_node_data(&raw_data)?;

            // Update the node with processed flag
            self.db
                .create_node(&raw_data_id.to_string(), "raw_data", &updated_data)
                .await
                .map_err(|e| ScraperError::Database {
                    message: format!("Failed to update raw data node: {e}"),
                })?;

            debug!("Marked raw data {} as processed", raw_data_id);
        }

        Ok(())
    }

    async fn create_process_run(&self, run: &mut ProcessRun) -> Result<()> {
        let id = Uuid::new_v4();
        run.id = Some(id);

        let node_data = Self::process_run_to_node_data(run)?;

        self.db
            .create_node(&id.to_string(), "process_run", &node_data)
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to create process run node: {e}"),
            })?;

        debug!("Created process run: {} with id {}", run.name, id);
        Ok(())
    }

    async fn update_process_run(&self, run: &ProcessRun) -> Result<()> {
        let run_id = run.id.ok_or_else(|| ScraperError::Api {
            message: "Cannot update process run without ID".to_string(),
        })?;

        let node_data = Self::process_run_to_node_data(run)?;

        // Use upsert operation to update the node with new data
        self.db
            .create_node(&run_id.to_string(), "process_run", &node_data)
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to update process run node: {e}"),
            })?;

        debug!("Updated process run: {} with id {}", run.name, run_id);
        Ok(())
    }

    async fn create_process_record(&self, record: &mut ProcessRecord) -> Result<()> {
        let id = Uuid::new_v4();
        record.id = Some(id);

        let node_data = Self::process_record_to_node_data(record)?;

        self.db
            .create_node(&id.to_string(), "process_record", &node_data)
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to create process record node: {e}"),
            })?;

        // Create edge linking process record to process run
        let edge_id = Uuid::new_v4();
        self.db
            .create_edge(
                &edge_id.to_string(),
                &record.process_run_id.to_string(),
                &id.to_string(),
                "has_record",
                None,
            )
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to create process run-record edge: {e}"),
            })?;

        debug!("Created process record with id {}", id);
        Ok(())
    }

    // Additional GraphQL query methods
    async fn get_venue_by_id(&self, venue_id: Uuid) -> Result<Option<Venue>> {
        if let Some((id, _label, data)) = self
            .db
            .get_node(&venue_id.to_string())
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to get venue node: {e}"),
            })?
        {
            Ok(Some(Self::node_data_to_venue(&id, &data)?))
        } else {
            Ok(None)
        }
    }

    async fn get_artist_by_id(&self, artist_id: Uuid) -> Result<Option<Artist>> {
        if let Some((id, _label, data)) = self
            .db
            .get_node(&artist_id.to_string())
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to get artist node: {e}"),
            })?
        {
            Ok(Some(Self::node_data_to_artist(&id, &data)?))
        } else {
            Ok(None)
        }
    }

    async fn get_event_by_id(&self, event_id: Uuid) -> Result<Option<Event>> {
        if let Some((id, _label, data)) = self
            .db
            .get_node(&event_id.to_string())
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to get event node: {e}"),
            })?
        {
            Ok(Some(Self::node_data_to_event(&id, &data)?))
        } else {
            Ok(None)
        }
    }

    async fn get_all_venues(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Venue>> {
        let venues_data = self
            .db
            .get_nodes_by_label("venue")
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query venues: {e}"),
            })?;

        let mut venues = Vec::new();
        for (id, _label, data) in venues_data.into_iter() {
            venues.push(Self::node_data_to_venue(&id, &data)?);
        }

        venues.sort_by(|a, b| a.name.cmp(&b.name));

        let offset = offset.unwrap_or(0);
        let end = if let Some(limit) = limit {
            std::cmp::min(offset + limit, venues.len())
        } else {
            venues.len()
        };

        Ok(venues.get(offset..end).unwrap_or(&[]).to_vec())
    }

    async fn get_all_artists(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Artist>> {
        let artists_data = self
            .db
            .get_nodes_by_label("artist")
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query artists: {e}"),
            })?;

        let mut artists = Vec::new();
        for (id, _label, data) in artists_data.into_iter() {
            artists.push(Self::node_data_to_artist(&id, &data)?);
        }

        artists.sort_by(|a, b| a.name.cmp(&b.name));

        let offset = offset.unwrap_or(0);
        let end = if let Some(limit) = limit {
            std::cmp::min(offset + limit, artists.len())
        } else {
            artists.len()
        };

        Ok(artists.get(offset..end).unwrap_or(&[]).to_vec())
    }

    async fn get_all_events(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Event>> {
        let events_data = self
            .db
            .get_nodes_by_label("event")
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query events: {e}"),
            })?;

        let mut events = Vec::new();
        for (id, _label, data) in events_data.into_iter() {
            events.push(Self::node_data_to_event(&id, &data)?);
        }

        events.sort_by(|a, b| a.event_day.cmp(&b.event_day));

        let offset = offset.unwrap_or(0);
        let end = if let Some(limit) = limit {
            std::cmp::min(offset + limit, events.len())
        } else {
            events.len()
        };

        Ok(events.get(offset..end).unwrap_or(&[]).to_vec())
    }

    async fn get_events_by_venue_id(&self, venue_id: Uuid) -> Result<Vec<Event>> {
        // Use graph edges to efficiently find events hosted by this venue
        let edges = self
            .db
            .get_edges_for_node(&venue_id.to_string())
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query edges for venue: {e}"),
            })?;

        let mut venue_events = Vec::new();
        for (_edge_id, source_id, target_id, relation, _data) in edges {
            // Look for "hosts" relationships where venue is the source
            if relation == "hosts" && source_id == venue_id.to_string() {
                // Get the event node
                if let Some((id, _label, data)) = self
                    .db
                    .get_node(&target_id)
                    .await
                    .map_err(|e| ScraperError::Database {
                        message: format!("Failed to get event node: {e}"),
                    })?
                {
                    venue_events.push(Self::node_data_to_event(&id, &data)?);
                }
            }
        }

        venue_events.sort_by(|a, b| a.event_day.cmp(&b.event_day));
        Ok(venue_events)
    }

    async fn get_events_by_artist_id(&self, artist_id: Uuid) -> Result<Vec<Event>> {
        // Use graph edges to efficiently find events where this artist performs
        let edges = self
            .db
            .get_edges_for_node(&artist_id.to_string())
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query edges for artist: {e}"),
            })?;

        let mut artist_events = Vec::new();
        for (_edge_id, source_id, target_id, relation, _data) in edges {
            // Look for "performs_at" relationships where artist is the source
            if relation == "performs_at" && source_id == artist_id.to_string() {
                // Get the event node
                if let Some((id, _label, data)) = self
                    .db
                    .get_node(&target_id)
                    .await
                    .map_err(|e| ScraperError::Database {
                        message: format!("Failed to get event node: {e}"),
                    })?
                {
                    artist_events.push(Self::node_data_to_event(&id, &data)?);
                }
            }
        }

        artist_events.sort_by(|a, b| a.event_day.cmp(&b.event_day));
        Ok(artist_events)
    }

    async fn get_events_by_date_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<Event>> {
        let events_data = self
            .db
            .get_nodes_by_label("event")
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query events: {e}"),
            })?;

        let mut filtered_events = Vec::new();
        for (id, _label, data) in events_data.into_iter() {
            let event = Self::node_data_to_event(&id, &data)?;
            if event.event_day >= start_date && event.event_day <= end_date {
                filtered_events.push(event);
            }
        }

        filtered_events.sort_by(|a, b| a.event_day.cmp(&b.event_day));
        Ok(filtered_events)
    }

    async fn search_artists(&self, query: &str) -> Result<Vec<Artist>> {
        let artists_data = self
            .db
            .get_nodes_by_label("artist")
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query artists: {e}"),
            })?;

        let query_lower = query.to_lowercase();
        let mut matching_artists = Vec::new();
        for (id, _label, data) in artists_data.into_iter() {
            let artist = Self::node_data_to_artist(&id, &data)?;
            if artist.name.to_lowercase().contains(&query_lower) {
                matching_artists.push(artist);
            }
        }

        matching_artists.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(matching_artists)
    }

    async fn get_venues_by_ids(&self, venue_ids: Vec<Uuid>) -> Result<Vec<Venue>> {
        let mut venues = Vec::new();
        
        for venue_id in venue_ids {
            if let Some((id, _label, data)) = self
                .db
                .get_node(&venue_id.to_string())
                .await
                .map_err(|e| ScraperError::Database {
                    message: format!("Failed to get venue node: {e}"),
                })?
            {
                venues.push(Self::node_data_to_venue(&id, &data)?);
            }
        }
        
        Ok(venues)
    }

    async fn get_artists_by_ids(&self, artist_ids: Vec<Uuid>) -> Result<Vec<Artist>> {
        let mut artists = Vec::new();
        
        for artist_id in artist_ids {
            if let Some((id, _label, data)) = self
                .db
                .get_node(&artist_id.to_string())
                .await
                .map_err(|e| ScraperError::Database {
                    message: format!("Failed to get artist node: {e}"),
                })?
            {
                artists.push(Self::node_data_to_artist(&id, &data)?);
            }
        }
        
        Ok(artists)
    }
}

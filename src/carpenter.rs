use crate::error::{Result, ScraperError};
use crate::pipeline::ProcessedEvent;
use crate::storage::Storage;
use crate::types::EventArgs as TypesEventArgs;
use chrono::{DateTime, Utc, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{info, warn, error, debug, instrument};

/// Change types for carpenter operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    Created,
    Updated,
    NoChange,
    Skip,
    Error,
}

/// Field types that can be changed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldChanged {
    Event,
    Venue,
    Artist,
    None,
}

/// A venue in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venue {
    pub id: Option<Uuid>,
    pub name: String,
    pub name_lower: String,
    pub slug: String,
    pub latitude: f64,
    pub longitude: f64,
    pub address: String,
    pub postal_code: String,
    pub city: String,
    pub venue_url: Option<String>,
    pub venue_image_url: Option<String>,
    pub description: Option<String>,
    pub neighborhood: Option<String>,
    pub show_venue: bool,
    pub created_at: DateTime<Utc>,
}

/// An artist in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub id: Option<Uuid>,
    pub name: String,
    pub name_slug: String,
    pub bio: Option<String>,
    pub artist_image_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// An event in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<Uuid>,
    pub title: String,
    pub event_day: NaiveDate,
    pub start_time: Option<NaiveTime>,
    pub event_url: Option<String>,
    pub description: Option<String>,
    pub event_image_url: Option<String>,
    pub venue_id: Uuid,
    pub artist_ids: Vec<Uuid>,
    pub show_event: bool,
    pub finalized: bool,
    pub created_at: DateTime<Utc>,
}

/// Raw data record from ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawData {
    pub id: Option<Uuid>,
    pub api_name: String,
    pub event_api_id: String,
    pub event_name: String,
    pub venue_name: String,
    pub event_day: NaiveDate,
    pub data: serde_json::Value,
    pub processed: bool,
    pub event_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// A carpenter run record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarpenterRun {
    pub id: Option<Uuid>,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

/// A record of changes made during carpenter run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarpenterRecord {
    pub id: Option<Uuid>,
    pub carpenter_run_id: Uuid,
    pub api_name: String,
    pub raw_data_id: Option<Uuid>,
    pub change_type: ChangeType,
    pub change_log: String,
    pub field_changed: FieldChanged,
    pub event_id: Option<Uuid>,
    pub venue_id: Option<Uuid>,
    pub artist_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Main Carpenter struct for data processing
pub struct Carpenter {
    storage: Arc<dyn Storage>,
}

impl std::fmt::Debug for Carpenter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Carpenter")
            .field("storage", &"<Arc<dyn Storage>>")
            .finish()
    }
}

impl Carpenter {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Process a single raw data record into venues, artists, and events
    #[instrument(skip(self, raw_data))]
    async fn process_single_raw_data(
        &self,
        raw_data: &RawData,
        run_id: Uuid,
    ) -> Result<Vec<CarpenterRecord>> {
        let mut change_records = Vec::new();
        
        // Parse the raw data back into structured format for processing
        let event_args: TypesEventArgs = serde_json::from_value(raw_data.data.clone())
            .map_err(|e| ScraperError::Api { 
                message: format!("Failed to parse raw data: {}", e) 
            })?;

        // Step 1: Create or find venue
        let (venue_id, venue_changes) = self.process_venue(&raw_data, run_id).await?;
        change_records.extend(venue_changes);

        // Step 2: Create or find artists (if any)
        let (artist_ids, artist_changes) = self.process_artists(&raw_data, run_id).await?;
        change_records.extend(artist_changes);

        // Step 3: Create or update event
        let event_changes = self.process_event(&raw_data, &event_args, venue_id, artist_ids, run_id).await?;
        change_records.extend(event_changes);

        Ok(change_records)
    }

    /// Process venue creation or matching
    #[instrument(skip(self, raw_data))]
    async fn process_venue(
        &self,
        raw_data: &RawData,
        run_id: Uuid,
    ) -> Result<(Uuid, Vec<CarpenterRecord>)> {
        let mut change_records = Vec::new();
        
        // Try to find existing venue by name
        if let Some(existing_venue) = self.storage.get_venue_by_name(&raw_data.venue_name).await? {
            debug!("Found existing venue: {} ({})", existing_venue.name, existing_venue.id.unwrap());
            
            change_records.push(CarpenterRecord::new(
                run_id,
                raw_data.api_name.clone(),
                raw_data.id,
                ChangeType::NoChange,
                format!("Using existing venue: {}", existing_venue.name),
                FieldChanged::Venue,
            ).with_venue(existing_venue.id.unwrap()));
            
            return Ok((existing_venue.id.unwrap(), change_records));
        }

        // Create new venue with default coordinates for Seattle venues
        let venue_args = self.create_default_venue_args(&raw_data.venue_name);
        let mut new_venue = Venue::new(venue_args);
        
        self.storage.create_venue(&mut new_venue).await?;
        let venue_id = new_venue.id.unwrap();
        
        info!("Created new venue: {} ({})", new_venue.name, venue_id);
        
        change_records.push(CarpenterRecord::new(
            run_id,
            raw_data.api_name.clone(),
            raw_data.id,
            ChangeType::Created,
            format!("Created new venue: {}", new_venue.name),
            FieldChanged::Venue,
        ).with_venue(venue_id));
        
        Ok((venue_id, change_records))
    }

    /// Process artist creation or matching
    #[instrument(skip(self, raw_data))]
    async fn process_artists(
        &self,
        raw_data: &RawData,
        run_id: Uuid,
    ) -> Result<(Vec<Uuid>, Vec<CarpenterRecord>)> {
        let mut change_records = Vec::new();
        let mut artist_ids = Vec::new();
        
        // For now, most crawlers don't extract artist info
        // This is a placeholder for future enhancement when we add artist extraction
        
        // Try to extract artist names from event title (basic heuristics)
        if let Some(extracted_artists) = self.extract_artists_from_title(&raw_data.event_name) {
            for artist_name in extracted_artists {
                let (artist_id, artist_change_records) = self.process_single_artist(&artist_name, raw_data, run_id).await?;
                artist_ids.push(artist_id);
                change_records.extend(artist_change_records);
            }
        }
        
        Ok((artist_ids, change_records))
    }

    /// Process a single artist
    #[instrument(skip(self, raw_data))]
    async fn process_single_artist(
        &self,
        artist_name: &str,
        raw_data: &RawData,
        run_id: Uuid,
    ) -> Result<(Uuid, Vec<CarpenterRecord>)> {
        let mut change_records = Vec::new();
        
        // Try to find existing artist
        if let Some(existing_artist) = self.storage.get_artist_by_name(artist_name).await? {
            debug!("Found existing artist: {} ({})", existing_artist.name, existing_artist.id.unwrap());
            
            change_records.push(CarpenterRecord::new(
                run_id,
                raw_data.api_name.clone(),
                raw_data.id,
                ChangeType::NoChange,
                format!("Using existing artist: {}", existing_artist.name),
                FieldChanged::Artist,
            ).with_artist(existing_artist.id.unwrap()));
            
            return Ok((existing_artist.id.unwrap(), change_records));
        }

        // Create new artist
        let artist_args = ArtistArgs {
            name: artist_name.to_string(),
            bio: None,
            artist_image_url: None,
        };
        
        let mut new_artist = Artist::new(artist_args);
        self.storage.create_artist(&mut new_artist).await?;
        let artist_id = new_artist.id.unwrap();
        
        info!("Created new artist: {} ({})", new_artist.name, artist_id);
        
        change_records.push(CarpenterRecord::new(
            run_id,
            raw_data.api_name.clone(),
            raw_data.id,
            ChangeType::Created,
            format!("Created new artist: {}", new_artist.name),
            FieldChanged::Artist,
        ).with_artist(artist_id));
        
        Ok((artist_id, change_records))
    }

    /// Process event creation or updating
    #[instrument(skip(self, raw_data, event_args))]
    async fn process_event(
        &self,
        raw_data: &RawData,
        event_args: &TypesEventArgs,
        venue_id: Uuid,
        artist_ids: Vec<Uuid>,
        run_id: Uuid,
    ) -> Result<Vec<CarpenterRecord>> {
        let mut change_records = Vec::new();
        
        // Try to find existing event by venue, date, and title
        if let Some(existing_event) = self.storage.get_event_by_venue_date_title(
            venue_id, 
            event_args.event_day, 
            &event_args.title
        ).await? {
            debug!("Found existing event: {} ({})", existing_event.title, existing_event.id.unwrap());
            
            // Check if event needs updating
            let needs_update = self.event_needs_update(&existing_event, event_args, &artist_ids);
            
            if needs_update {
                let mut updated_event = existing_event.clone();
                self.update_event_from_args(&mut updated_event, event_args, artist_ids);
                
                self.storage.update_event(&updated_event).await?;
                
                info!("Updated existing event: {} ({})", updated_event.title, updated_event.id.unwrap());
                
                change_records.push(CarpenterRecord::new(
                    run_id,
                    raw_data.api_name.clone(),
                    raw_data.id,
                    ChangeType::Updated,
                    format!("Updated event: {}", updated_event.title),
                    FieldChanged::Event,
                ).with_event(updated_event.id.unwrap()));
            } else {
                change_records.push(CarpenterRecord::new(
                    run_id,
                    raw_data.api_name.clone(),
                    raw_data.id,
                    ChangeType::NoChange,
                    format!("No changes needed for event: {}", existing_event.title),
                    FieldChanged::Event,
                ).with_event(existing_event.id.unwrap()));
            }
            
            return Ok(change_records);
        }

        // Create new event
        let mut new_event = Event::from_types_args(event_args.clone(), venue_id, artist_ids);
        self.storage.create_event(&mut new_event).await?;
        let event_id = new_event.id.unwrap();
        
        info!("Created new event: {} ({}) on {}", new_event.title, event_id, new_event.event_day);
        
        change_records.push(CarpenterRecord::new(
            run_id,
            raw_data.api_name.clone(),
            raw_data.id,
            ChangeType::Created,
            format!("Created new event: {} on {}", new_event.title, new_event.event_day),
            FieldChanged::Event,
        ).with_event(event_id));
        
        Ok(change_records)
    }

    /// Create default venue args for known Seattle venues
    fn create_default_venue_args(&self, venue_name: &str) -> VenueArgs {
        match venue_name {
            "Blue Moon Tavern" => VenueArgs {
                name: venue_name.to_string(),
                latitude: 47.6689, // Approximate coordinates for Blue Moon Tavern
                longitude: -122.3151,
                address: "712 NE 45th St".to_string(),
                postal_code: "98105".to_string(),
                city: "Seattle".to_string(),
                venue_url: Some("https://www.bluemoontavern.com".to_string()),
                venue_image_url: None,
                description: Some("Neighborhood tavern in the U-District".to_string()),
                neighborhood: Some("University District".to_string()),
            },
            "Sea Monster Lounge" => VenueArgs {
                name: venue_name.to_string(),
                latitude: 47.6815, // Approximate coordinates for Sea Monster Lounge
                longitude: -122.3351, 
                address: "2202 N 45th St".to_string(),
                postal_code: "98103".to_string(),
                city: "Seattle".to_string(),
                venue_url: Some("https://www.seamonsterlounge.com".to_string()),
                venue_image_url: None,
                description: Some("Wallingford cocktail lounge and music venue".to_string()),
                neighborhood: Some("Wallingford".to_string()),
            },
            _ => {
                // Default for unknown venues - use generic Seattle coordinates
                VenueArgs {
                    name: venue_name.to_string(),
                    latitude: 47.6062, // Seattle center coordinates
                    longitude: -122.3321,
                    address: "Address TBD".to_string(),
                    postal_code: "98101".to_string(),
                    city: "Seattle".to_string(),
                    venue_url: None,
                    venue_image_url: None,
                    description: None,
                    neighborhood: None,
                }
            }
        }
    }

    /// Extract artist names from event title using basic heuristics
    fn extract_artists_from_title(&self, title: &str) -> Option<Vec<String>> {
        // Simple heuristics - look for common patterns
        // This is a basic implementation; real-world would be more sophisticated
        
        // Skip certain event types that don't typically have extractable artists
        let lower_title = title.to_lowercase();
        if lower_title.contains("open mic") || 
           lower_title.contains("karaoke") ||
           lower_title.contains("trivia") ||
           lower_title.contains("bingo") {
            return None;
        }
        
        // Look for common separators
        if title.contains(" with ") {
            let parts: Vec<&str> = title.split(" with ").collect();
            if parts.len() >= 2 {
                return Some(parts.into_iter().map(|s| s.trim().to_string()).collect());
            }
        }
        
        if title.contains(" & ") {
            let parts: Vec<&str> = title.split(" & ").collect();
            if parts.len() >= 2 {
                return Some(parts.into_iter().map(|s| s.trim().to_string()).collect());
            }
        }
        
        // If no separators found, treat the whole title as a single artist
        // But only if it looks like an artist name (not an event description)
        if !lower_title.contains("night") && !lower_title.contains("show") && !lower_title.contains("party") {
            Some(vec![title.trim().to_string()])
        } else {
            None
        }
    }

    /// Check if an event needs updating based on new data
    fn event_needs_update(&self, existing_event: &Event, new_args: &TypesEventArgs, new_artist_ids: &[Uuid]) -> bool {
        // Check if any key fields have changed
        if existing_event.start_time != new_args.start_time ||
           existing_event.event_url != new_args.event_url ||
           existing_event.description != new_args.description ||
           existing_event.event_image_url != new_args.event_image_url ||
           existing_event.artist_ids != new_artist_ids {
            return true;
        }
        
        false
    }

    /// Update an event with new args
    fn update_event_from_args(&self, event: &mut Event, new_args: &TypesEventArgs, new_artist_ids: Vec<Uuid>) {
        event.start_time = new_args.start_time;
        event.event_url = new_args.event_url.clone();
        event.description = new_args.description.clone();
        event.event_image_url = new_args.event_image_url.clone();
        event.artist_ids = new_artist_ids;
    }

    #[instrument(skip(self))]
    pub async fn run(
        &self, 
        apis: Option<Vec<String>>, 
        min_date: Option<NaiveDate>, 
        process_all: bool
    ) -> Result<()> {
        let run_name = self.create_run_name(&apis, &min_date, process_all);
        let mut carpenter_run = CarpenterRun::new(run_name);
        self.storage.create_carpenter_run(&mut carpenter_run).await?;
        let run_id = carpenter_run.id.unwrap();

        info!(run_id = ?run_id, "Starting Carpenter run");

        let api_list = apis.unwrap_or_else(|| crate::types::API_PRIORITY_ORDER.iter().map(|s| s.to_string()).collect());

        for api_name in api_list {
            self.process_api(&api_name, run_id, min_date, process_all).await?;
        }

        carpenter_run.finish();
        self.storage.update_carpenter_run(&carpenter_run).await?;
        info!(run_id = ?run_id, "Finished Carpenter run");
        
        Ok(())
    }

    #[instrument(skip(self, api_name, run_id))]
    async fn process_api(
        &self, 
        api_name: &str, 
        run_id: Uuid,
        min_date: Option<NaiveDate>,
        process_all: bool
    ) -> Result<()> {
        info!("Processing API: {}", api_name);
        let raw_data_list = self.storage.get_unprocessed_raw_data(api_name, min_date).await?;
        info!("Found {} unprocessed raw data records for {}", raw_data_list.len(), api_name);

        let raw_data_count = raw_data_list.len();
        for raw_data in raw_data_list {
            info!("Processing raw data: {} - {}", raw_data.event_name, raw_data.event_day);
            
            match self.process_single_raw_data(&raw_data, run_id).await {
                Ok(change_records) => {
                    // Save all change records for this raw data
                    for mut record in change_records {
                        if let Err(e) = self.storage.create_carpenter_record(&mut record).await {
                            warn!("Failed to save carpenter record: {}", e);
                        }
                    }
                    
                    // Mark raw data as processed
                    if let Err(e) = self.storage.mark_raw_data_processed(raw_data.id.unwrap()).await {
                        error!("Failed to mark raw data {} as processed: {}", raw_data.id.unwrap(), e);
                    } else {
                        debug!("Successfully processed raw data: {}", raw_data.event_name);
                    }
                }
                Err(e) => {
                    error!("Failed to process raw data {}: {}", raw_data.event_name, e);
                    
                    // Create error record
                    let mut error_record = CarpenterRecord::new(
                        run_id,
                        api_name.to_string(),
                        raw_data.id,
                        ChangeType::Error,
                        format!("Processing failed: {}", e),
                        FieldChanged::None,
                    );
                    
                    if let Err(record_err) = self.storage.create_carpenter_record(&mut error_record).await {
                        error!("Failed to save error record: {}", record_err);
                    }
                }
            }
        }
        
        info!("Completed processing {} raw data records for {}", raw_data_count, api_name);
        Ok(())
    }

    fn create_run_name(
        &self, 
        apis: &Option<Vec<String>>, 
        min_date: &Option<NaiveDate>, 
        process_all: bool
    ) -> String {
        let api_str = apis.as_ref().map_or("All Apis".to_string(), |a| a.join(", "));
        let date_str = min_date.map_or("None".to_string(), |d| d.to_string());
        format!("Carpenter Run - {} - Min Date: {} - Process All: {}", api_str, date_str, process_all)
    }
}

/// Arguments for creating/updating a venue
#[derive(Debug, Clone)]
pub struct VenueArgs {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub address: String,
    pub postal_code: String,
    pub city: String,
    pub venue_url: Option<String>,
    pub venue_image_url: Option<String>,
    pub description: Option<String>,
    pub neighborhood: Option<String>,
}

/// Arguments for creating/updating an artist
#[derive(Debug, Clone)]
pub struct ArtistArgs {
    pub name: String,
    pub bio: Option<String>,
    pub artist_image_url: Option<String>,
}

/// Arguments for creating/updating an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventArgs {
    pub title: String,
    pub event_day: NaiveDate,
    pub start_time: Option<NaiveTime>,
    pub event_url: Option<String>,
    pub description: Option<String>,
    pub event_image_url: Option<String>,
}

impl Venue {
    /// Create a new venue with generated derived fields
    pub fn new(args: VenueArgs) -> Self {
        let name_lower = args.name.to_lowercase();
        let slug = name_lower
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .replace(' ', "-");

        Self {
            id: None,
            name: args.name,
            name_lower,
            slug,
            latitude: (args.latitude * 100000.0).round() / 100000.0, // Round to 5 decimal places
            longitude: (args.longitude * 100000.0).round() / 100000.0,
            address: args.address,
            postal_code: args.postal_code,
            city: args.city,
            venue_url: args.venue_url,
            venue_image_url: args.venue_image_url,
            description: args.description,
            neighborhood: args.neighborhood,
            show_venue: true,
            created_at: Utc::now(),
        }
    }
}

impl Artist {
    /// Create a new artist with generated derived fields
    pub fn new(args: ArtistArgs) -> Self {
        let name_slug = args.name.to_lowercase().replace(' ', "-");

        Self {
            id: None,
            name: args.name,
            name_slug,
            bio: args.bio,
            artist_image_url: args.artist_image_url,
            created_at: Utc::now(),
        }
    }
}

impl Event {
    /// Create a new event
    pub fn new(args: EventArgs, venue_id: Uuid, artist_ids: Vec<Uuid>) -> Self {
        Self {
            id: None,
            title: args.title,
            event_day: args.event_day,
            start_time: args.start_time,
            event_url: args.event_url,
            description: args.description,
            event_image_url: args.event_image_url,
            venue_id,
            artist_ids,
            show_event: true,
            finalized: false,
            created_at: Utc::now(),
        }
    }
    
    /// Create a new event from TypesEventArgs (for compatibility)
    pub fn from_types_args(args: TypesEventArgs, venue_id: Uuid, artist_ids: Vec<Uuid>) -> Self {
        Self::new(EventArgs {
            title: args.title,
            event_day: args.event_day,
            start_time: args.start_time,
            event_url: args.event_url,
            description: args.description,
            event_image_url: args.event_image_url,
        }, venue_id, artist_ids)
    }
}

impl RawData {
    /// Create a new raw data record from processed event
    pub fn from_processed_event(processed_event: &ProcessedEvent) -> Self {
        Self {
            id: None,
            api_name: processed_event.api_name.clone(),
            event_api_id: processed_event.raw_data_info.event_api_id.clone(),
            event_name: processed_event.raw_data_info.event_name.clone(),
            venue_name: processed_event.raw_data_info.venue_name.clone(),
            event_day: processed_event.raw_data_info.event_day,
            data: serde_json::to_value(&processed_event.event_args).unwrap_or_default(),
            processed: false,
            event_id: None,
            created_at: processed_event.processed_at,
        }
    }
}

impl CarpenterRun {
    /// Create a new carpenter run
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            created_at: Utc::now(),
            finished_at: None,
        }
    }

    /// Mark the run as finished
    pub fn finish(&mut self) {
        self.finished_at = Some(Utc::now());
    }
}

impl CarpenterRecord {
    /// Create a new carpenter record
    pub fn new(
        carpenter_run_id: Uuid,
        api_name: String,
        raw_data_id: Option<Uuid>,
        change_type: ChangeType,
        change_log: String,
        field_changed: FieldChanged,
    ) -> Self {
        Self {
            id: None,
            carpenter_run_id,
            api_name,
            raw_data_id,
            change_type,
            change_log,
            field_changed,
            event_id: None,
            venue_id: None,
            artist_id: None,
            created_at: Utc::now(),
        }
    }

    /// Set the event that was affected
    pub fn with_event(mut self, event_id: Uuid) -> Self {
        self.event_id = Some(event_id);
        self
    }

    /// Set the venue that was affected
    pub fn with_venue(mut self, venue_id: Uuid) -> Self {
        self.venue_id = Some(venue_id);
        self
    }

    /// Set the artist that was affected
    pub fn with_artist(mut self, artist_id: Uuid) -> Self {
        self.artist_id = Some(artist_id);
        self
    }
}

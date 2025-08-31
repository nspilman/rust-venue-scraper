use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error, debug};
use uuid::Uuid;
use sms_core::domain::{RawData, Venue, Artist, Event};
use sms_core::storage::{Storage, DatabaseStorage};
use sms_core::common::types::{RawDataInfo, EventArgs};

/// Orchestrator for running the complete data processing pipeline
/// 
/// This bridges the gap between raw data ingestion and final entity cataloging.
/// Currently handles the transition from raw ingested data to processed entities.
pub struct FullPipelineOrchestrator {
    storage: Arc<dyn Storage>,
}

impl FullPipelineOrchestrator {
    /// Create a new pipeline orchestrator
    pub async fn new() -> Result<Self> {
        let storage = Arc::new(DatabaseStorage::new().await?);
        Ok(Self { storage })
    }

    /// Process all unprocessed raw data for a given source through the complete pipeline
    pub async fn process_source(&self, source_id: &str) -> Result<ProcessingResult> {
        info!("🔄 Starting full pipeline processing for source: {}", source_id);

        // Check if bypass-cadence is set via environment variable to force fresh ingestion
        let force_fresh_ingestion = std::env::var("BYPASS_CADENCE").is_ok() || 
                                   std::env::var("FORCE_FRESH_INGESTION").is_ok();
        
        info!("🔍 Force fresh ingestion: {}", force_fresh_ingestion);
        info!("🔍 BYPASS_CADENCE env var: {:?}", std::env::var("BYPASS_CADENCE"));
        info!("🔍 FORCE_FRESH_INGESTION env var: {:?}", std::env::var("FORCE_FRESH_INGESTION"));

        // Get all unprocessed raw data for this source
        // Convert user-friendly source_id to internal API name for database lookup
        let internal_api_name = crate::common::constants::api_name_to_internal(source_id);
        let mut raw_data_items = self.storage.get_unprocessed_raw_data(&internal_api_name, None).await?;
        let mut just_ingested_fresh_data = false;
        
        if raw_data_items.is_empty() || force_fresh_ingestion {
            if force_fresh_ingestion {
                info!("🔄 Force fresh ingestion enabled - running ingestion to fetch latest data...");
            } else {
                info!("ℹ️  No unprocessed raw data found for source: {}", source_id);
                info!("🔄 Running ingestion to fetch fresh data...");
            }
            
            // Run ingestion to fetch fresh data
            match self.run_ingestion_for_source(source_id).await {
                Ok(_) => {
                    info!("✅ Ingestion completed, checking for new raw data...");
                    // Get the newly ingested raw data
                    raw_data_items = self.storage.get_unprocessed_raw_data(&internal_api_name, None).await?;
                    just_ingested_fresh_data = true;
                    
                    if raw_data_items.is_empty() {
                        info!("⚠️  No raw data found even after ingestion - source may be empty or have issues");
                        return Ok(ProcessingResult {
                            source_id: source_id.to_string(),
                            total_items: 0,
                            processed_items: 0,
                            failed_items: 0,
                            errors: vec!["No data available after ingestion".to_string()],
                        });
                    }
                }
                Err(e) => {
                    error!("❌ Failed to run ingestion: {}", e);
                    return Ok(ProcessingResult {
                        source_id: source_id.to_string(),
                        total_items: 0,
                        processed_items: 0,
                        failed_items: 1,
                        errors: vec![format!("Ingestion failed: {}", e)],
                    });
                }
            }
        }

        // If we just ingested fresh data, only process the most recent item (the fresh HTML)
        // to avoid processing old cached items without wix-warmup-data
        if just_ingested_fresh_data && raw_data_items.len() > 1 {
            info!("🔄 Just ingested fresh data - processing only the most recent item to avoid old cached data");
            // Sort by created_at descending and take only the first (most recent) item
            raw_data_items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            raw_data_items.truncate(1);
        }

        info!("📊 Found {} unprocessed raw data items for {}", raw_data_items.len(), source_id);

        let mut result = ProcessingResult {
            source_id: source_id.to_string(),
            total_items: raw_data_items.len(),
            processed_items: 0,
            failed_items: 0,
            errors: Vec::new(),
        };

        // For now, we'll process by creating entities directly from the structured data
        // The ingester already did the parsing work by extracting meaningful data from raw sources
        for raw_data in &raw_data_items {
            match self.process_raw_data_item(raw_data).await {
                Ok(()) => {
                    // Mark as processed
                    if let Some(id) = raw_data.id {
                        if let Err(e) = self.storage.mark_raw_data_processed(id).await {
                            error!("Failed to mark raw data {} as processed: {}", id, e);
                            result.errors.push(format!("Failed to mark processed: {}", e));
                        } else {
                            result.processed_items += 1;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to process raw data item {}: {}", 
                        raw_data.id.map(|id| id.to_string()).unwrap_or("unknown".to_string()), e);
                    result.failed_items += 1;
                    result.errors.push(format!("Processing failed: {}", e));
                }
            }
        }

        info!("✅ Pipeline processing completed for {}: {} processed, {} failed", 
              source_id, result.processed_items, result.failed_items);

        Ok(result)
    }

    /// Process a single raw data item through the complete pipeline stages
    async fn process_raw_data_item(&self, raw_data: &RawData) -> Result<()> {
        debug!("Processing raw data item: {} ({})", raw_data.event_name, raw_data.api_name);
        
        // Step 1: Parse - Convert raw HTML/JSON to structured events
        info!("📄 Step 1: Parse");
        let parsed_events = self.parse_raw_data(raw_data).await?;
        
        info!("✅ Parsed {} events from raw data", parsed_events.len());
        
        // Process each parsed event through the pipeline
        for parsed_data in parsed_events {
            info!("🔄 Processing event: {}", parsed_data.event_args.title);
            
            // Step 2: Normalize - Standardize data format
            info!("📝 Step 2: Normalize");
            let normalized_data = self.normalize_parsed_data(&parsed_data).await?;
            
            // Step 3: Quality Gate - Check data quality and completeness
            info!("✅ Step 3: Quality Gate");
            let quality_result = self.quality_gate_check(&normalized_data).await?;
            if !quality_result.passed {
                info!("❌ Quality gate failed for {}: {}", normalized_data.title, quality_result.reason);
                continue; // Skip this event, continue with next
            }
            
            // Step 4: Enrich - Add additional data and context
            info!("🔍 Step 4: Enrich");
            let enriched_data = self.enrich_data(&normalized_data).await?;
            
            // Step 5: Conflation - Resolve entity relationships
            info!("🔗 Step 5: Conflation");
            let conflated_data = self.conflate_entities(&enriched_data).await?;
            
            // Step 6: Catalog - Store final entities in database
            info!("📚 Step 6: Catalog");
            self.catalog_entities(&conflated_data).await?;
            info!("✅ Event cataloged: {}", normalized_data.title);
        }

        Ok(())
    }

    /// Parse raw HTML/JSON data into structured format
    async fn parse_raw_data(&self, raw_data: &RawData) -> Result<Vec<ParsedEventData>> {
        // Create appropriate parser based on source
        // Map internal storage name back to user-friendly name for factory
        let api_name = match raw_data.api_name.as_str() {
            "crawler_neumos" => "neumos",
            "crawler_barboza" => "barboza", 
            "crawler_blue_moon" => "blue_moon",
            "crawler_sea_monster_lounge" => "sea_monster",
            "crawler_darrells_tavern" => "darrells_tavern",
            "crawler_kexp" => "kexp",
            "crawler_conor_byrne" => "conor_byrne",
            other => other,
        };
        
        // Create parser directly
        let parser = super::super::apis::factory::create_parser(api_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown parser: {}", api_name))?;
        
        let mut parsed_data_list = Vec::new();
        
        // Check if the data is already structured (parsed during ingestion) or raw
        if raw_data.data.is_object() || raw_data.data.is_array() {
            // Data is already parsed - treat it as a single event or array of events
            let events = if let Some(array) = raw_data.data.as_array() {
                array.clone()
            } else {
                vec![raw_data.data.clone()]
            };
            
            for event_json in events {
                let raw_data_info = parser.extract_raw_data_info(&event_json)?;
                let event_args = parser.extract_event_args(&event_json)?;
                
                parsed_data_list.push(ParsedEventData {
                    raw_data_info,
                    event_args,
                    source_api: raw_data.api_name.clone(),
                });
            }
        } else {
            // Data is raw (string/bytes) - needs parsing
            let json_string;
            let bytes_slice = if let Some(bytes_str) = raw_data.data.as_str() {
                bytes_str.as_bytes()
            } else {
                json_string = serde_json::to_string(&raw_data.data)?;
                json_string.as_bytes()
            };
            
            let parsed_events = parser.parse_events(bytes_slice).await?;
            
            for event_json in parsed_events {
                let raw_data_info = parser.extract_raw_data_info(&event_json)?;
                let event_args = parser.extract_event_args(&event_json)?;
                
                parsed_data_list.push(ParsedEventData {
                    raw_data_info,
                    event_args,
                    source_api: raw_data.api_name.clone(),
                });
            }
        }
        
        Ok(parsed_data_list)
    }
    
    /// Normalize parsed data to consistent format
    async fn normalize_parsed_data(&self, parsed: &ParsedEventData) -> Result<NormalizedEventData> {
        // Convert to normalized format with consistent field names and types
        Ok(NormalizedEventData {
            title: parsed.event_args.title.clone(),
            venue_name: parsed.raw_data_info.venue_name.clone(),
            event_day: parsed.event_args.event_day,
            start_time: parsed.event_args.start_time,
            description: parsed.event_args.description.clone(),
            event_url: parsed.event_args.event_url.clone(),
            image_url: parsed.event_args.event_image_url.clone(),
            source_api: parsed.source_api.clone(),
        })
    }
    
    /// Quality gate validation
    async fn quality_gate_check(&self, normalized: &NormalizedEventData) -> Result<QualityResult> {
        // Basic quality checks
        if normalized.title.trim().is_empty() {
            return Ok(QualityResult { passed: false, reason: "Empty title".to_string() });
        }
        
        if normalized.venue_name.trim().is_empty() {
            return Ok(QualityResult { passed: false, reason: "Empty venue name".to_string() });
        }
        
        // Could add more sophisticated quality checks here
        Ok(QualityResult { passed: true, reason: "Passed".to_string() })
    }
    
    /// Enrich data with additional context
    async fn enrich_data(&self, normalized: &NormalizedEventData) -> Result<EnrichedEventData> {
        // Add enrichment like geocoding, categorization, etc.
        Ok(EnrichedEventData {
            normalized_data: normalized.clone(),
            city: "Seattle".to_string(), // Could be derived from venue
            neighborhood: None, // Could be geocoded
            categories: Vec::new(), // Could be inferred from title/description
        })
    }
    
    /// Conflate entities to resolve duplicates and relationships
    async fn conflate_entities(&self, enriched: &EnrichedEventData) -> Result<ConflatedEventData> {
        // Entity resolution and relationship mapping
        Ok(ConflatedEventData {
            enriched_data: enriched.clone(),
            resolved_venue_id: None, // Will be resolved in catalog step
            resolved_artist_ids: Vec::new(), // Will be resolved in catalog step
        })
    }
    
    /// Catalog final entities in database
    async fn catalog_entities(&self, conflated: &ConflatedEventData) -> Result<()> {
        let normalized = &conflated.enriched_data.normalized_data;
        
        // Create or find the venue
        self.ensure_venue(&normalized.venue_name).await?;
        
        // Create or find artists from the event title
        self.ensure_artists_from_title(&normalized.title).await?;
        
        // Create the event entity
        self.create_event_entity_from_normalized(normalized).await?;
        
        Ok(())
    }
    
    /// Create event entity from normalized data
    async fn create_event_entity_from_normalized(&self, normalized: &NormalizedEventData) -> Result<()> {
        // Get the venue
        let venue = self.storage.get_venue_by_name(&normalized.venue_name).await?
            .ok_or_else(|| anyhow::anyhow!("Venue not found: {}", normalized.venue_name))?;

        let venue_id = venue.id.ok_or_else(|| anyhow::anyhow!("Venue ID missing"))?;

        // Check if event already exists
        if let Ok(Some(_)) = self.storage.get_event_by_venue_date_title(
            venue_id, 
            normalized.event_day, 
            &normalized.title
        ).await {
            debug!("Event already exists: {} on {}", normalized.title, normalized.event_day);
            return Ok(());
        }

        // Extract and link artists from the event title
        let artist_ids = self.get_artist_ids_from_title(&normalized.title).await?;
        debug!("Found {} artist IDs for event: {}", artist_ids.len(), normalized.title);

        // Create new event
        let mut event = Event {
            id: None,
            title: normalized.title.clone(),
            event_day: normalized.event_day,
            start_time: normalized.start_time,
            event_url: normalized.event_url.clone(),
            description: normalized.description.clone(),
            event_image_url: normalized.image_url.clone(),
            venue_id,
            artist_ids,
            show_event: true,
            finalized: false,
            created_at: chrono::Utc::now(),
        };

        self.storage.create_event(&mut event).await?;
        debug!("Created event: {} on {} with {} artists", normalized.title, normalized.event_day, event.artist_ids.len());
        Ok(())
    }

    /// Ensure a venue exists in the database
    async fn ensure_venue(&self, venue_name: &str) -> Result<()> {
        // Check if venue already exists
        if let Ok(Some(_)) = self.storage.get_venue_by_name(venue_name).await {
            return Ok(());
        }

        // Create new venue with default Seattle coordinates and required fields
        let mut venue = Venue {
            id: None,
            name: venue_name.to_string(),
            name_lower: venue_name.to_lowercase(),
            slug: venue_name.to_lowercase().replace(" ", "-"),
            latitude: 47.6062, // Default Seattle latitude
            longitude: -122.3321, // Default Seattle longitude  
            address: "Seattle, WA".to_string(), // Default address
            postal_code: "98101".to_string(), // Default Seattle postal code
            city: "Seattle".to_string(),
            venue_url: None,
            venue_image_url: None,
            description: None,
            neighborhood: None,
            show_venue: true,
            created_at: chrono::Utc::now(),
        };

        self.storage.create_venue(&mut venue).await?;
        debug!("Created venue: {}", venue_name);
        Ok(())
    }

    /// Extract and ensure artists exist from event title
    async fn ensure_artists_from_title(&self, title: &str) -> Result<()> {
        let mut potential_artists = Vec::new();
        
        // Handle KEXP-specific format: "Artist Name LIVE on KEXP (OPEN TO THE PUBLIC)"
        if title.contains("LIVE on KEXP") {
            if let Some(artist_part) = title.split(" LIVE on KEXP").next() {
                let artist_name = artist_part.trim();
                if !artist_name.is_empty() && artist_name.len() > 2 {
                    potential_artists.push(artist_name);
                }
            }
        } else {
            // Fallback to general parsing for other venues
            potential_artists = title
                .split(&[',', '&', '+', '/', '|'][..])
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
        }

        for artist_name in potential_artists {
            // Skip common venue/event words (but not "live" for KEXP since we handle it above)
            let lower = artist_name.to_lowercase();
            if lower.contains("presents") || lower.contains("show") || 
               lower.contains("concert") || lower.len() < 2 {
                continue;
            }

            // Generate consistent slug
            let artist_slug = self.generate_artist_slug(artist_name);
            
            // Check if artist already exists by name OR by slug to avoid duplicates
            if let Ok(Some(_)) = self.storage.get_artist_by_name(artist_name).await {
                continue;
            }
            
            // Also check by slug to prevent UNIQUE constraint errors
            if let Ok(Some(_)) = self.get_artist_by_slug(&artist_slug).await {
                debug!("Artist with slug '{}' already exists, skipping: {}", artist_slug, artist_name);
                continue;
            }

            // Create new artist
            let mut artist = Artist {
                id: None,
                name: artist_name.to_string(),
                name_slug: artist_slug,
                bio: None,
                artist_image_url: None,
                created_at: chrono::Utc::now(),
            };

            match self.storage.create_artist(&mut artist).await {
                Ok(_) => {
                    debug!("Created artist: {} (slug: {})", artist_name, artist.name_slug);
                },
                Err(e) => {
                    // If we still get a constraint error, log it but don't fail the entire event
                    error!("Failed to create artist '{}' (slug: '{}'): {}", artist_name, artist.name_slug, e);
                    // Continue processing instead of failing
                }
            }
        }

        Ok(())
    }
    
    /// Generate a consistent, URL-safe slug from artist name
    fn generate_artist_slug(&self, name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("-")
    }
    
    /// Get artist by slug (helper method)
    async fn get_artist_by_slug(&self, slug: &str) -> Result<Option<Artist>> {
        self.storage.get_artist_by_slug(slug).await.map_err(|e| anyhow::anyhow!("Database error: {}", e))
    }
    
    /// Extract artist IDs from event title (assumes artists have already been created)
    async fn get_artist_ids_from_title(&self, title: &str) -> Result<Vec<Uuid>> {
        let mut potential_artists = Vec::new();
        
        // Handle KEXP-specific format: "Artist Name LIVE on KEXP (OPEN TO THE PUBLIC)"
        if title.contains("LIVE on KEXP") {
            if let Some(artist_part) = title.split(" LIVE on KEXP").next() {
                let artist_name = artist_part.trim();
                if !artist_name.is_empty() && artist_name.len() > 2 {
                    potential_artists.push(artist_name);
                }
            }
        } else {
            // Fallback to general parsing for other venues
            potential_artists = title
                .split(&[',', '&', '+', '/', '|'][..])
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
        }

        let mut artist_ids = Vec::new();
        
        for artist_name in potential_artists {
            // Skip common venue/event words (but not "live" for KEXP since we handle it above)
            let lower = artist_name.to_lowercase();
            if lower.contains("presents") || lower.contains("show") || 
               lower.contains("concert") || lower.len() < 2 {
                continue;
            }

            // Try to find artist by name first
            if let Ok(Some(artist)) = self.storage.get_artist_by_name(artist_name).await {
                if let Some(id) = artist.id {
                    artist_ids.push(id);
                    continue;
                }
            }
            
            // Try to find artist by slug as backup
            let artist_slug = self.generate_artist_slug(artist_name);
            if let Ok(Some(artist)) = self.get_artist_by_slug(&artist_slug).await {
                if let Some(id) = artist.id {
                    artist_ids.push(id);
                }
            }
        }
        
        Ok(artist_ids)
    }

    /// Create an event entity from raw data
    async fn create_event_entity(&self, raw_data: &RawData, event_data: &serde_json::Value) -> Result<()> {
        // Get the venue
        let venue = self.storage.get_venue_by_name(&raw_data.venue_name).await?
            .ok_or_else(|| anyhow::anyhow!("Venue not found: {}", raw_data.venue_name))?;

        let venue_id = venue.id.ok_or_else(|| anyhow::anyhow!("Venue ID missing"))?;

        // Extract event details
        let title = event_data.get("title")
            .and_then(|t| t.as_str())
            .unwrap_or(&raw_data.event_name);

        let description = event_data.get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        let start_time = event_data.get("start_time")
            .and_then(|t| t.as_str())
            .and_then(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M:%S").ok());

        let event_url = event_data.get("event_url")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        let event_image_url = event_data.get("event_image_url")
            .or_else(|| event_data.get("image_url"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        // Check if event already exists
        if let Ok(Some(_)) = self.storage.get_event_by_venue_date_title(
            venue_id, 
            raw_data.event_day, 
            title
        ).await {
            debug!("Event already exists: {} on {}", title, raw_data.event_day);
            return Ok(());
        }

        // Extract and link artists from the event title
        let artist_ids = self.get_artist_ids_from_title(title).await?;

        // Create new event
        let mut event = Event {
            id: None,
            title: title.to_string(),
            event_day: raw_data.event_day,
            start_time,
            event_url,
            description,
            event_image_url,
            venue_id,
            artist_ids,
            show_event: true,
            finalized: false,
            created_at: chrono::Utc::now(),
        };

        self.storage.create_event(&mut event).await?;
        debug!("Created event: {} on {} with {} artists", title, raw_data.event_day, event.artist_ids.len());
        Ok(())
    }

    /// Run ingestion for a specific source to fetch fresh raw data
    async fn run_ingestion_for_source(&self, source_id: &str) -> Result<()> {
        info!("🔄 Running ingestion for source: {}", source_id);
        
        // Create the appropriate crawler for this source
        let crawler = crate::apis::factory::create_crawler(source_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown crawler for source: {}", source_id))?;
        
        // Fetch raw event data from the source
        let raw_event_data = crawler.get_event_list().await?;
        
        info!("📥 Fetched {} raw event items from {}", raw_event_data.len(), source_id);
        
        // Convert raw event data to RawData entities and store them
        let internal_api_name = crate::common::constants::api_name_to_internal(source_id);
        
        for event_data in raw_event_data {
            // Extract raw data info for storage
            let raw_data_info = crawler.get_raw_data_info(&event_data)?;
            
            // Create RawData entity
            let mut raw_data = sms_core::domain::RawData {
                id: None,
                event_api_id: raw_data_info.event_api_id,
                event_name: raw_data_info.event_name,
                venue_name: raw_data_info.venue_name,
                event_day: raw_data_info.event_day,
                api_name: internal_api_name.clone(),
                data: event_data,
                processed: false,
                event_id: None, // Will be set when event is created during processing
                created_at: chrono::Utc::now(),
            };
            
            // Store the raw data
            self.storage.create_raw_data(&mut raw_data).await?;
        }
        
        info!("✅ Ingestion completed for {}", source_id);
        Ok(())
    }
}

/// Data structures for pipeline stages
#[derive(Debug, Clone)]
pub struct ParsedEventData {
    pub raw_data_info: RawDataInfo,
    pub event_args: EventArgs,
    pub source_api: String,
}

#[derive(Debug, Clone)]
pub struct NormalizedEventData {
    pub title: String,
    pub venue_name: String,
    pub event_day: chrono::NaiveDate,
    pub start_time: Option<chrono::NaiveTime>,
    pub description: Option<String>,
    pub event_url: Option<String>,
    pub image_url: Option<String>,
    pub source_api: String,
}

#[derive(Debug, Clone)]
pub struct EnrichedEventData {
    pub normalized_data: NormalizedEventData,
    pub city: String,
    pub neighborhood: Option<String>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConflatedEventData {
    pub enriched_data: EnrichedEventData,
    pub resolved_venue_id: Option<Uuid>,
    pub resolved_artist_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct QualityResult {
    pub passed: bool,
    pub reason: String,
}

/// Result of processing a source through the full pipeline
#[derive(Debug)]
pub struct ProcessingResult {
    pub source_id: String,
    pub total_items: usize,
    pub processed_items: usize,
    pub failed_items: usize,
    pub errors: Vec<String>,
}

impl ProcessingResult {
    /// Check if processing was successful (no failures)
    pub fn is_success(&self) -> bool {
        self.failed_items == 0
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_items == 0 {
            100.0
        } else {
            (self.processed_items as f64 / self.total_items as f64) * 100.0
        }
    }
}
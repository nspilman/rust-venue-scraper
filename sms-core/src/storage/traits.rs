use crate::domain::*;
use crate::common::error::Result;
use async_trait::async_trait;
use chrono::NaiveDate;
use uuid::Uuid;

/// Storage trait for persisting domain data (venues, artists, events, raw data, and process runs/records)
#[async_trait]
#[allow(dead_code)]
pub trait Storage: Send + Sync {
    // Venue operations
    async fn create_venue(&self, venue: &mut Venue) -> Result<()>;
    async fn get_venue_by_name(&self, name: &str) -> Result<Option<Venue>>;
    
    // Artist operations
    async fn create_artist(&self, artist: &mut Artist) -> Result<()>;
    async fn get_artist_by_name(&self, name: &str) -> Result<Option<Artist>>;
    async fn get_artist_by_slug(&self, slug: &str) -> Result<Option<Artist>>;
    
    // Event operations
    async fn create_event(&self, event: &mut Event) -> Result<()>;
    async fn get_event_by_venue_date_title(
        &self,
        venue_id: uuid::Uuid,
        date: chrono::NaiveDate,
        title: &str
    ) -> Result<Option<Event>>;
    async fn update_event(&self, event: &Event) -> Result<()>;
    async fn delete_event(&self, event_id: Uuid) -> Result<()>;
    
    // Raw data operations
    async fn create_raw_data(&self, raw_data: &mut RawData) -> Result<()>;
    async fn get_unprocessed_raw_data(
        &self,
        api_name: &str,
        min_date: Option<NaiveDate>
    ) -> Result<Vec<RawData>>;
    async fn mark_raw_data_processed(&self, raw_data_id: Uuid) -> Result<()>;
    
    // Processing operations
    async fn create_process_run(&self, run: &mut ProcessRun) -> Result<()>;
    async fn update_process_run(&self, run: &ProcessRun) -> Result<()>;
    
    async fn create_process_record(&self, record: &mut ProcessRecord) -> Result<()>;

    // Additional query methods for GraphQL
    async fn get_venue_by_id(&self, venue_id: Uuid) -> Result<Option<Venue>>;
    async fn get_artist_by_id(&self, artist_id: Uuid) -> Result<Option<Artist>>;
    async fn get_event_by_id(&self, event_id: Uuid) -> Result<Option<Event>>;
    async fn get_all_venues(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Venue>>;
    async fn get_all_artists(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Artist>>;
    async fn get_all_events(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Event>>;
    async fn get_events_by_venue_id(&self, venue_id: Uuid) -> Result<Vec<Event>>;
    async fn get_events_by_artist_id(&self, artist_id: Uuid) -> Result<Vec<Event>>;
    async fn get_events_by_date_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<Event>>;
    async fn search_artists(&self, query: &str) -> Result<Vec<Artist>>;

    // Batch loading methods for GraphQL DataLoader optimization
    async fn get_venues_by_ids(&self, venue_ids: Vec<Uuid>) -> Result<Vec<Venue>>;
    async fn get_artists_by_ids(&self, artist_ids: Vec<Uuid>) -> Result<Vec<Artist>>;
}

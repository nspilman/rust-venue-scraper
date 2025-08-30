use sms_core::common::error::{Result, ScraperError};
use sms_core::common::types::{EventApi, EventArgs, RawDataInfo, RawEventData};
use crate::pipeline::ingestion::ingest_common::fetch_payload_and_log;
use tracing::{info, instrument};

/// Trait for venue-specific parsing logic
#[async_trait::async_trait]
pub trait VenueParser: Send + Sync {
    /// Parse raw HTTP payload into structured event data
    async fn parse_events(&self, payload: &[u8]) -> Result<Vec<RawEventData>>;
    
    /// Extract metadata for raw data storage
    fn extract_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo>;
    
    /// Extract event arguments for processing
    fn extract_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs>;
    
    /// Get the venue name for this parser
    fn venue_name(&self) -> &'static str;
}

/// Base crawler that implements EventApi using venue-specific parsers
pub struct BaseCrawler {
    client: reqwest::Client,
    api_name: &'static str,
    parser: Box<dyn VenueParser>,
}

impl BaseCrawler {
    pub fn new(api_name: &'static str, parser: Box<dyn VenueParser>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_name,
            parser,
        }
    }
}

#[async_trait::async_trait]
impl EventApi for BaseCrawler {
    fn api_name(&self) -> &'static str {
        self.api_name
    }

    #[instrument(skip(self))]
    async fn get_event_list(&self) -> Result<Vec<RawEventData>> {
        let payload = fetch_payload_and_log(self.api_name).await?;
        let events = self.parser.parse_events(&payload).await?;
        
        info!(
            "Successfully fetched {} events from {}",
            events.len(),
            self.parser.venue_name()
        );
        
        Ok(events)
    }

    fn get_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo> {
        self.parser.extract_raw_data_info(raw_data)
    }

    fn get_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs> {
        self.parser.extract_event_args(raw_data)
    }
}
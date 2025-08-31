use sms_core::common::error::{Result, ScraperError};
use sms_core::common::types::{EventApi, EventArgs, RawDataInfo, RawEventData};
use crate::pipeline::ingestion::ingest_common::fetch_payload_and_log;
use crate::infra::http_client::ReqwestHttp;
use crate::app::ports::HttpClientPort;
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
    http_client: ReqwestHttp,
    api_name: &'static str,
    parser: Box<dyn VenueParser>,
}

impl BaseCrawler {
    pub fn new(api_name: &'static str, parser: Box<dyn VenueParser>) -> Self {
        Self {
            http_client: ReqwestHttp,
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
        // Use our HTTP client with User-Agent header to fetch the payload
        let url = match self.api_name {
            "sea_monster" => "https://www.seamonsterlounge.com/buy-tickets-in-advance",
            "kexp" => "https://www.kexp.org/events/kexp-events/",
            "blue_moon" => "https://www.bluemoontavern.com/calendar/",
            "darrells_tavern" => "https://www.darrellstavern.com/events/",
            "barboza" => "https://www.barbozaseattle.com/events/",
            "neumos" => "https://www.neumos.com/events/",
            "conor_byrne" => "https://www.conorbyrnepub.com/events/",
            _ => return Err(ScraperError::Api {
                message: format!("Unknown API name: {}", self.api_name),
            }),
        };
        
        let http_result = self.http_client.get(url).await.map_err(|e| ScraperError::Api {
            message: format!("HTTP request failed: {}", e),
        })?;
        
        let events = self.parser.parse_events(&http_result.bytes).await?;
        
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
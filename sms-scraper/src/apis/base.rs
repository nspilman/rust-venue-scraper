use sms_core::common::error::{Result, ScraperError};
use sms_core::common::types::{EventApi, EventArgs, RawDataInfo, RawEventData};
use crate::infra::http_client::ReqwestHttp;
use crate::app::ports::HttpClientPort;
use crate::registry::source_loader::SourceRegistry;
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
    source_registry: SourceRegistry,
}

impl BaseCrawler {
    pub fn new(api_name: &'static str, parser: Box<dyn VenueParser>, source_registry: SourceRegistry) -> Self {
        Self {
            http_client: ReqwestHttp,
            api_name,
            parser,
            source_registry,
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
        // Per Platonic Ideal: ingester should only fetch raw HTML/JSON bytes
        // Load URL from source registry instead of hardcoding
        let url = self.source_registry.get_source_url(self.api_name)?;
        
        let http_result = self.http_client.get(&url).await.map_err(|e| ScraperError::Api {
            message: format!("HTTP request failed: {}", e),
        })?;
        
        info!(
            "Successfully fetched {} bytes of raw data from {}",
            http_result.bytes.len(),
            self.parser.venue_name()
        );
        
        // Store the raw payload as a single raw data item with processed=false
        // The parser will handle extracting individual events from this data in a separate step
        let raw_data_value = serde_json::Value::String(
            String::from_utf8_lossy(&http_result.bytes).to_string()
        );
        Ok(vec![raw_data_value])
    }

    fn get_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo> {
        self.parser.extract_raw_data_info(raw_data)
    }

    fn get_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs> {
        self.parser.extract_event_args(raw_data)
    }
}
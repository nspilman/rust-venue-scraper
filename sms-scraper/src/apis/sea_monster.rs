use crate::common::constants::{SEA_MONSTER_API, SEA_MONSTER_VENUE_NAME};
use sms_core::common::error::{Result, ScraperError};
use crate::pipeline::ingestion::ingest_common::fetch_payload_and_log;
use sms_core::common::types::{EventApi, EventArgs, RawDataInfo, RawEventData};
use scraper::{Html, Selector};
use serde_json::Value;
use tracing::{debug, error, info, instrument};

pub struct SeaMonsterCrawler {
    _client: reqwest::Client, // Prefixed with _ to suppress warning
}

impl Default for SeaMonsterCrawler {
    fn default() -> Self {
        Self::new()
    }
}

impl SeaMonsterCrawler {
    pub fn new() -> Self {
        Self {
            _client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl EventApi for SeaMonsterCrawler {
    fn api_name(&self) -> &'static str {
        SEA_MONSTER_API
    }

    #[instrument(skip(self))]
    async fn get_event_list(&self) -> Result<Vec<RawEventData>> {
        // Per Platonic Ideal: ingester should only fetch raw HTML/JSON bytes
        // The parsing should happen in the parser, not here
        let payload = fetch_payload_and_log(SEA_MONSTER_API).await?;
        
        info!("Successfully fetched {} bytes of raw HTML from Sea Monster Lounge", payload.len());
        
        // Store the raw HTML payload as a single raw data item
        // The parser will handle extracting individual events from this HTML
        let raw_html_value = Value::String(String::from_utf8_lossy(&payload).to_string());
        Ok(vec![raw_html_value])
    }

    fn get_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo> {
        // For raw HTML data, we provide generic info since parsing happens later
        Ok(RawDataInfo {
            event_api_id: "raw_html_page".to_string(),
            event_name: "Sea Monster Raw HTML Page".to_string(),
            venue_name: SEA_MONSTER_VENUE_NAME.to_string(),
            event_day: chrono::Utc::now().date_naive(),
        })
    }

    fn get_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs> {
        // This method is not used in the Platonic Ideal architecture
        // Event args are extracted during the parsing phase, not ingestion
        Err(ScraperError::Api {
            message: "get_event_args should not be called for raw HTML data".to_string(),
        })
    }

    fn should_skip(&self, raw_data: &RawEventData) -> (bool, String) {
        // For raw HTML data, we don't skip at ingestion time
        // Skipping logic will be handled during parsing
        (false, String::new())
    }
}

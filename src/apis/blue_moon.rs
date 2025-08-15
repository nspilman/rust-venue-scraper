use crate::common::constants::{BLUE_MOON_API, BLUE_MOON_VENUE_NAME};
use crate::common::error::{Result, ScraperError};
use crate::pipeline::ingestion::ingest_common::fetch_payload_and_log;
use crate::common::types::{EventApi, EventArgs, RawDataInfo, RawEventData};
use serde_json::Value;
use tracing::{debug, info, instrument};

pub struct BlueMoonCrawler {
    _client: reqwest::Client, // Prefixed with _ to suppress warning
}

impl Default for BlueMoonCrawler {
    fn default() -> Self {
        Self::new()
    }
}

impl BlueMoonCrawler {
    pub fn new() -> Self {
        Self {
            _client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl EventApi for BlueMoonCrawler {
    fn api_name(&self) -> &'static str {
        BLUE_MOON_API
    }

    #[instrument(skip(self))]
    async fn get_event_list(&self) -> Result<Vec<RawEventData>> {
        // New path: use shared ingestion helper, then parse
        let payload = fetch_payload_and_log(BLUE_MOON_API).await?;

        let data: Value = serde_json::from_slice(&payload)?;
        let events_by_date = data["eventsByDates"].as_object().ok_or_else(|| {
            ScraperError::MissingField("eventsByDates not found".into())
        })?;

        let mut all_events = Vec::new();
        for (day, events) in events_by_date {
            if let Some(event_array) = events.as_array() {
                debug!("Processing {} events for date {}", event_array.len(), day);
                for event in event_array {
                    let mut event_clone = event.clone();
                    event_clone["event_day"] = day.clone().into();
                    all_events.push(event_clone);
                }
            }
        }
        info!(
            "Successfully fetched {} events from Blue Moon Tavern",
            all_events.len()
        );
        Ok(all_events)
    }

    fn get_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo> {
        let title = raw_data["title"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("title not found".into()))?;
        let id = raw_data["id"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("id not found".into()))?;
        let event_day_str = raw_data["event_day"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("event_day not found".into()))?;
        let event_day =
            chrono::NaiveDate::parse_from_str(event_day_str, "%Y-%m-%d").map_err(|e| {
                ScraperError::Api {
                    message: format!("Failed to parse event_day: {e}"),
                }
            })?;

        Ok(RawDataInfo {
            event_api_id: id.to_string(),
            event_name: title.to_string(),
            venue_name: BLUE_MOON_VENUE_NAME.to_string(), // Fixed venue
            event_day,
        })
    }

    fn get_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs> {
        let title = raw_data["title"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("title not found".into()))?;
        let event_day_str = raw_data["event_day"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("event_day not found".into()))?;
        let event_day =
            chrono::NaiveDate::parse_from_str(event_day_str, "%Y-%m-%d").map_err(|e| {
                ScraperError::Api {
                    message: format!("Failed to parse event_day: {e}"),
                }
            })?;

        let start_time = if let Some(start_date) = raw_data["startDate"].as_str() {
            if let Some(time_str) = start_date.split('T').nth(1) {
                chrono::NaiveTime::parse_from_str(
                    time_str.split('-').next().unwrap_or(""),
                    "%H:%M:%S",
                )
                .ok()
            } else {
                None
            }
        } else {
            None
        };

        Ok(EventArgs {
            title: title.to_string(),
            event_day,
            start_time,
            event_url: None,
            description: None,
            event_image_url: None,
        })
    }
}

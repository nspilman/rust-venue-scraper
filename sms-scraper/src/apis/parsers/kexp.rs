use super::super::base::VenueParser;
use crate::common::constants::KEXP_VENUE_NAME;
use sms_core::common::error::{Result, ScraperError};
use sms_core::common::types::{EventArgs, RawDataInfo, RawEventData};

pub struct KexpParser;

impl KexpParser {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl VenueParser for KexpParser {
    fn venue_name(&self) -> &'static str {
        KEXP_VENUE_NAME
    }

    async fn parse_events(&self, _payload: &[u8]) -> Result<Vec<RawEventData>> {
        // Placeholder - would contain actual KEXP parsing logic
        Ok(vec![])
    }

    fn extract_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo> {
        let title = raw_data["title"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("title not found".into()))?;
        let id = raw_data["id"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("id not found".into()))?;
        let event_day_str = raw_data["event_day"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("event_day not found".into()))?;
        let event_day = chrono::NaiveDate::parse_from_str(event_day_str, "%Y-%m-%d")
            .map_err(|e| ScraperError::Api {
                message: format!("Failed to parse event_day: {e}"),
            })?;

        Ok(RawDataInfo {
            event_api_id: id.to_string(),
            event_name: title.to_string(),
            venue_name: KEXP_VENUE_NAME.to_string(),
            event_day,
        })
    }

    fn extract_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs> {
        let title = raw_data["title"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("title not found".into()))?;
        let event_day_str = raw_data["event_day"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("event_day not found".into()))?;
        let event_day = chrono::NaiveDate::parse_from_str(event_day_str, "%Y-%m-%d")
            .map_err(|e| ScraperError::Api {
                message: format!("Failed to parse event_day: {e}"),
            })?;

        Ok(EventArgs {
            title: title.to_string(),
            event_day,
            start_time: None,
            event_url: None,
            description: None,
            event_image_url: None,
        })
    }
}
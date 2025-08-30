use super::super::base::VenueParser;
use crate::common::constants::SEA_MONSTER_VENUE_NAME;
use sms_core::common::error::{Result, ScraperError};
use sms_core::common::types::{EventArgs, RawDataInfo, RawEventData};
use scraper::{Html, Selector};
use serde_json::Value;
use tracing::{debug, error};

pub struct SeaMonsterParser;

impl SeaMonsterParser {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl VenueParser for SeaMonsterParser {
    fn venue_name(&self) -> &'static str {
        SEA_MONSTER_VENUE_NAME
    }

    async fn parse_events(&self, payload: &[u8]) -> Result<Vec<RawEventData>> {
        let body = String::from_utf8_lossy(payload).to_string();
        let document = Html::parse_document(&body);
        let selector =
            Selector::parse("script[type=\"application/json\"]#wix-warmup-data").unwrap();

        if let Some(element) = document.select(&selector).next() {
            debug!("Found wix-warmup-data script tag, parsing JSON");
            let json_text = element.inner_html();
            let data: Value = serde_json::from_str(&json_text)?;

            let mut all_events = Vec::new();
            if let Some(apps_data) = data["appsWarmupData"].as_object() {
                for (_, app_data) in apps_data {
                    if let Some(widgets) = app_data.as_object() {
                        for (widget_key, widget_data) in widgets {
                            if widget_key.starts_with("widget") {
                                if let Some(events_data) =
                                    widget_data.get("events").and_then(|e| e.get("events"))
                                {
                                    if let Some(events_array) = events_data.as_array() {
                                        debug!(
                                            "Found {} events in widget {}",
                                            events_array.len(),
                                            widget_key
                                        );
                                        for event in events_array {
                                            all_events.push(event.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(all_events)
        } else {
            error!("Could not find wix-warmup-data script tag on Sea Monster page");
            Err(ScraperError::Api {
                message: "Could not find wix-warmup-data script tag".to_string(),
            })
        }
    }

    fn extract_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo> {
        let title = raw_data["title"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("title not found".into()))?;
        let slug = raw_data["slug"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("slug not found".into()))?;
        let start_date_str = raw_data["scheduling"]["startDateFormatted"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("startDateFormatted not found".into()))?;
        
        let event_day = chrono::NaiveDate::parse_from_str(start_date_str, "%B %d, %Y")
            .map_err(|e| ScraperError::Api {
                message: format!("Failed to parse event_day: {e}"),
            })?;

        Ok(RawDataInfo {
            event_api_id: slug.to_string(),
            event_name: title.to_string(),
            venue_name: SEA_MONSTER_VENUE_NAME.to_string(),
            event_day,
        })
    }

    fn extract_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs> {
        let title = raw_data["title"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("title not found".into()))?;
        let start_date_str = raw_data["scheduling"]["startDateFormatted"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("startDateFormatted not found".into()))?;
        
        let event_day = chrono::NaiveDate::parse_from_str(start_date_str, "%B %d, %Y")
            .map_err(|e| ScraperError::Api {
                message: format!("Failed to parse event_day: {e}"),
            })?;

        let start_time_str = raw_data["scheduling"]["startTimeFormatted"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("startTimeFormatted not found".into()))?;
        let start_time = chrono::NaiveTime::parse_from_str(start_time_str, "%I:%M %p").ok();

        let slug = raw_data["slug"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("slug not found".into()))?;

        let image_url = raw_data
            .get("mainImage")
            .and_then(|i| i.get("url"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        let event_url = Some(format!(
            "https://www.seamonsterlounge.com/event-info/{slug}"
        ));

        Ok(EventArgs {
            title: title.trim().to_string(),
            event_day,
            start_time,
            event_url,
            description: None,
            event_image_url: image_url,
        })
    }
}
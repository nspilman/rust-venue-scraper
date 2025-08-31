use super::super::base::VenueParser;
use crate::common::constants::SEA_MONSTER_VENUE_NAME;
use sms_core::common::error::{Result, ScraperError};
use sms_core::common::types::{EventArgs, RawDataInfo, RawEventData};
use scraper::{Html, Selector};
use serde_json::Value;
use tracing::{debug, error, info};

pub struct SeaMonsterParser;

impl SeaMonsterParser {
    pub fn new() -> Self {
        Self
    }

    fn search_for_events_recursively(&self, value: &Value, events: &mut Vec<Value>) {
        match value {
            Value::Object(obj) => {
                for (key, val) in obj {
                    // Look for keys that might contain events
                    if key.to_lowercase().contains("event") || key.to_lowercase().contains("item") {
                        if let Value::Array(arr) = val {
                            // Check if this array contains event-like objects
                            for item in arr {
                                if let Value::Object(event_obj) = item {
                                    // Check if this looks like an event (has title, date, etc.)
                                    if event_obj.contains_key("title") || 
                                       event_obj.contains_key("name") ||
                                       event_obj.contains_key("scheduling") ||
                                       event_obj.contains_key("startDate") {
                                        events.push(item.clone());
                                    }
                                }
                            }
                        }
                    }
                    // Recursively search deeper
                    self.search_for_events_recursively(val, events);
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    self.search_for_events_recursively(item, events);
                }
            }
            _ => {}
        }
    }
}

#[async_trait::async_trait]
impl VenueParser for SeaMonsterParser {
    fn venue_name(&self) -> &'static str {
        SEA_MONSTER_VENUE_NAME
    }

    async fn parse_events(&self, payload: &[u8]) -> Result<Vec<RawEventData>> {
        let body = String::from_utf8_lossy(payload).to_string();
        debug!("Processing raw HTML content of {} bytes", body.len());
        let document = Html::parse_document(&body);
        let warmup_selector = Selector::parse("script[type=\"application/json\"]#wix-warmup-data").unwrap();

        if let Some(element) = document.select(&warmup_selector).next() {
            debug!("Found wix-warmup-data script tag, parsing JSON");
            let json_text = element.inner_html();
            let data: Value = serde_json::from_str(&json_text)?;

            let mut all_events = Vec::new();
            // Follow Python logic exactly: iterate through appsWarmupData nested structure
            if let Some(apps_data) = data.get("appsWarmupData").and_then(|d| d.as_object()) {
                for (_, app_data) in apps_data {
                    if let Some(app_obj) = app_data.as_object() {
                        for (_, widget_data) in app_obj {
                            if let Some(events_container) = widget_data.get("events") {
                                if let Some(events_array) = events_container.get("events").and_then(|e| e.as_array()) {
                                    debug!("Found events array with {} events", events_array.len());
                                    
                                    for event in events_array {
                                        if let Some(event_obj) = event.as_object() {
                                            // Parse the start date and add enriched fields like Python
                                            if let Some(start_date_str) = event_obj.get("scheduling")
                                                .and_then(|s| s.get("startDateFormatted"))
                                                .and_then(|d| d.as_str()) {
                                                
                                                let event_day = chrono::NaiveDate::parse_from_str(start_date_str, "%B %d, %Y")
                                                    .map_err(|e| ScraperError::Api {
                                                        message: format!("Failed to parse event_day: {e}"),
                                                    })?;
                                                
                                                // Clone the event and add enriched fields exactly like Python
                                                let mut enhanced_event = event.clone();
                                                if let Some(enhanced_obj) = enhanced_event.as_object_mut() {
                                                    // event_data["event_day"] = datetime.strptime(...).strftime("%Y-%m-%d")
                                                    enhanced_obj.insert("event_day".to_string(), Value::String(event_day.format("%Y-%m-%d").to_string()));
                                                    
                                                    // event_data["event_api_id"] = event_data["slug"]
                                                    if let Some(slug) = event_obj.get("slug").and_then(|s| s.as_str()) {
                                                        enhanced_obj.insert("event_api_id".to_string(), Value::String(slug.to_string()));
                                                    }
                                                    
                                                    // event_data["event_name"] = event_data["title"].strip()
                                                    if let Some(title) = event_obj.get("title").and_then(|t| t.as_str()) {
                                                        enhanced_obj.insert("event_name".to_string(), Value::String(title.trim().to_string()));
                                                    }
                                                }
                                                
                                                all_events.push(enhanced_event);
                                                debug!("Added event: {}", event_obj.get("title").and_then(|t| t.as_str()).unwrap_or("Unknown"));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            info!("Successfully parsed {} events from Sea Monster Lounge", all_events.len());
            Ok(all_events)
        } else {
            error!("Could not find wix-warmup-data script tag in HTML");
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
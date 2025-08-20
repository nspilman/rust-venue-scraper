use crate::common::constants::{KEXP_API, KEXP_VENUE_NAME};
use crate::common::error::{Result, ScraperError};
use crate::pipeline::ingestion::ingest_common::fetch_payload_and_log;
use crate::common::types::{EventApi, EventArgs, RawDataInfo, RawEventData};
use scraper::{Html, Selector};
use serde_json::{json, Value};
use tracing::{debug, info, instrument, warn};

pub struct KexpCrawler {
    _client: reqwest::Client, // Prefixed with _ to suppress warning
}

impl Default for KexpCrawler {
    fn default() -> Self {
        Self::new()
    }
}

impl KexpCrawler {
    pub fn new() -> Self {
        Self {
            _client: reqwest::Client::new(),
        }
    }

    fn parse_time_string(&self, time_str: &str) -> Option<chrono::NaiveTime> {
        // Handle various time formats: "1 p.m.", "noon", "midnight", "11 a.m."
        let time_clean = time_str.trim().to_lowercase();
        
        if time_clean == "noon" {
            return Some(chrono::NaiveTime::from_hms_opt(12, 0, 0)?);
        }
        
        if time_clean == "midnight" {
            return Some(chrono::NaiveTime::from_hms_opt(0, 0, 0)?);
        }
        
        // Parse "1 p.m." or "11 a.m." format
        if let Some(captures) = regex::Regex::new(r"(\d{1,2})(?::(\d{2}))?\s*(a\.m\.|p\.m\.)")
            .ok()?
            .captures(&time_clean) 
        {
            let hour: u32 = captures.get(1)?.as_str().parse().ok()?;
            let minute: u32 = captures.get(2)
                .map(|m| m.as_str().parse().unwrap_or(0))
                .unwrap_or(0);
            let is_pm = captures.get(3)?.as_str().contains("p.m.");
            
            let hour_24 = if is_pm && hour != 12 {
                hour + 12
            } else if !is_pm && hour == 12 {
                0
            } else {
                hour
            };
            
            return chrono::NaiveTime::from_hms_opt(hour_24, minute, 0);
        }
        
        warn!("Could not parse time string: {}", time_str);
        None
    }

    fn parse_date_header(&self, date_str: &str) -> Result<chrono::NaiveDate> {
        // Parse "Wednesday, 20 August 2025" format
        let date_clean = date_str.trim();
        
        // Try multiple date formats KEXP might use
        let formats = [
            "%A, %d %B %Y",      // "Wednesday, 20 August 2025"
            "%A, %B %d, %Y",     // "Wednesday, August 20, 2025"
            "%d %B %Y",          // "20 August 2025"
            "%B %d, %Y",         // "August 20, 2025"
        ];
        
        for format in &formats {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(date_clean, format) {
                return Ok(date);
            }
        }
        
        Err(ScraperError::Api {
            message: format!("Could not parse date: {}", date_str),
        })
    }

    fn extract_events_from_html(&self, html: &str) -> Result<Vec<Value>> {
        let document = Html::parse_document(html);
        
        // Select date headers
        let _date_selector = Selector::parse("h2").unwrap();
        let _event_selector = Selector::parse("article.EventItem").unwrap();
        
        let mut events = Vec::new();
        let mut current_date: Option<chrono::NaiveDate> = None;
        
        // Process the document sequentially to match dates with events
        for element in document.select(&Selector::parse("h2, article.EventItem").unwrap()) {
            if element.value().name() == "h2" {
                // This is a date header
                let date_text = element.text().collect::<Vec<_>>().join(" ");
                if let Ok(date) = self.parse_date_header(&date_text) {
                    current_date = Some(date);
                    debug!("Found date header: {} -> {}", date_text, date);
                }
            } else if element.value().name() == "article" {
                // This is an event item
                if let Some(date) = current_date {
                    if let Some(event) = self.parse_event_item(&element, date) {
                        events.push(event);
                    }
                }
            }
        }
        
        info!("Extracted {} events from KEXP page", events.len());
        Ok(events)
    }

    fn parse_event_item(&self, element: &scraper::ElementRef, event_date: chrono::NaiveDate) -> Option<Value> {
        // Extract time
        let time_selector = Selector::parse(".EventItem-DateTime h5").unwrap();
        let time_text = element
            .select(&time_selector)
            .next()?
            .text()
            .collect::<Vec<_>>()
            .join(" ");
        
        let start_time = self.parse_time_string(&time_text);
        
        // Extract title and URL
        let title_selector = Selector::parse(".EventItem-body h3 a").unwrap();
        let title_link = element.select(&title_selector).next()?;
        let title = title_link.text().collect::<Vec<_>>().join(" ");
        let url_path = title_link.value().attr("href")?;
        let full_url = format!("https://www.kexp.org{}", url_path);
        
        // Extract venue/location
        let location_selector = Selector::parse(".EventItem-body .u-h3 a").unwrap();
        let location = element
            .select(&location_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| KEXP_VENUE_NAME.to_string());
        
        // Extract description/photo credit
        let description_selector = Selector::parse(".EventItem-description").unwrap();
        let description = element
            .select(&description_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" "));
        
        // Create a unique ID from the URL path
        let event_id = url_path
            .trim_start_matches("/events/kexp-events/")
            .trim_end_matches("/")
            .to_string();
        
        Some(json!({
            "id": event_id,
            "title": title,
            "date": event_date.format("%Y-%m-%d").to_string(),
            "time": time_text,
            "start_time": start_time.map(|t| t.format("%H:%M:%S").to_string()),
            "location": location,
            "url": full_url,
            "description": description
        }))
    }
}

#[async_trait::async_trait]
impl EventApi for KexpCrawler {
    fn api_name(&self) -> &'static str {
        KEXP_API
    }

    #[instrument(skip(self))]
    async fn get_event_list(&self) -> Result<Vec<RawEventData>> {
        debug!("Starting KEXP event fetch");
        
        // Use the shared ingestion helper to fetch raw bytes
        let payload = fetch_payload_and_log(KEXP_API).await?;
        
        let html = String::from_utf8_lossy(&payload).to_string();
        let events = self.extract_events_from_html(&html)?;
        
        info!("Successfully fetched {} events from KEXP", events.len());
        Ok(events)
    }

    fn get_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo> {
        let title = raw_data["title"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("title not found".into()))?;
        let id = raw_data["id"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("id not found".into()))?;
        let date_str = raw_data["date"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("date not found".into()))?;
        
        let event_day = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| ScraperError::Api {
                message: format!("Failed to parse date '{}': {}", date_str, e),
            })?;

        Ok(RawDataInfo {
            event_api_id: id.to_string(),
            event_name: title.to_string(),
            venue_name: KEXP_VENUE_NAME.to_string(),
            event_day,
        })
    }

    fn get_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs> {
        let title = raw_data["title"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("title not found".into()))?;
        let date_str = raw_data["date"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("date not found".into()))?;
        
        let event_day = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| ScraperError::Api {
                message: format!("Failed to parse date '{}': {}", date_str, e),
            })?;

        let start_time = raw_data["start_time"]
            .as_str()
            .and_then(|t| chrono::NaiveTime::parse_from_str(t, "%H:%M:%S").ok());

        let event_url = raw_data["url"].as_str().map(|s| s.to_string());
        let description = raw_data["description"].as_str().map(|s| s.to_string());

        Ok(EventArgs {
            title: title.to_string(),
            event_day,
            start_time,
            event_url,
            description,
            event_image_url: None, // KEXP doesn't seem to have event-specific images
        })
    }

    fn should_skip(&self, raw_data: &RawEventData) -> (bool, String) {
        // Skip events that are not open to the public
        if let Some(title) = raw_data["title"].as_str() {
            let title_lower = title.to_lowercase();
            
            // Skip broadcast-only events (not open to public)
            if !title_lower.contains("open to the public") && title_lower.contains("live on kexp") {
                return (true, "Skipping broadcast-only event (not open to public)".to_string());
            }
            
            // Skip if it's just a reading or non-musical event (optional filter)
            if title_lower.contains("book reading") {
                return (true, "Skipping book reading event".to_string());
            }
        }
        
        (false, String::new())
    }
}

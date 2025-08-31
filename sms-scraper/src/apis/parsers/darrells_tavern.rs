use super::super::base::VenueParser;
use crate::common::constants::DARRELLS_TAVERN_VENUE_NAME;
use sms_core::common::error::{Result, ScraperError};
use sms_core::common::types::{EventArgs, RawDataInfo, RawEventData};
use serde_json::json;
use chrono::Datelike;

pub struct DarrellsTavernParser;

impl DarrellsTavernParser {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl VenueParser for DarrellsTavernParser {
    fn venue_name(&self) -> &'static str {
        DARRELLS_TAVERN_VENUE_NAME
    }

    async fn parse_events(&self, payload: &[u8]) -> Result<Vec<RawEventData>> {
        use tracing::info;
        
        let body = String::from_utf8_lossy(payload);
        info!("Parsing body of length: {}", body.len());
        
        // Check if this is already JSON (from previous processing)
        if body.trim().starts_with('{') {
            info!("Input appears to be JSON, attempting to parse as single event");
            match serde_json::from_str::<RawEventData>(&body) {
                Ok(event) => {
                    info!("Successfully parsed JSON event: {:?}", event);
                    return Ok(vec![event]);
                }
                Err(e) => {
                    info!("Failed to parse as JSON: {}", e);
                    // Fall through to HTML parsing
                }
            }
        }
        
        // Original HTML parsing logic
        use scraper::{Html, Selector, ElementRef};
        
        let preview = if body.len() > 500 {
            format!("{}...", &body[..500])
        } else {
            body.to_string()
        };
        info!("HTML content preview: {}", preview);
        
        let document = Html::parse_document(&body);
        
        // Find the entry-content div that contains all the events
        let content_selector = Selector::parse(".entry-content").unwrap();
        
        let mut events = Vec::new();
        
        if let Some(content_div) = document.select(&content_selector).next() {
            info!("Found .entry-content div");
            
            // Get all child elements in order to properly associate dates with bands
            let mut current_date = None;
            let mut current_date_text = String::new();
            
            for child in content_div.children() {
                if let Some(element) = child.value().as_element() {
                    let element_ref = ElementRef::wrap(child).unwrap();
                    
                    match element.name() {
                        "h1" => {
                            // Save previous event if we have one
                            if let Some(date) = current_date.take() {
                                // This h1 marks the end of the previous event, so we don't have bands for it
                                info!("Found h1 but no bands for previous date: {}", current_date_text);
                            }
                            
                            // Parse new date
                            let date_text = element_ref.text().collect::<String>();
                            info!("Found h1 with date: '{}'", date_text);
                            
                            if let Some(parsed_date) = self.parse_date(&date_text) {
                                current_date = Some(parsed_date);
                                current_date_text = date_text;
                                info!("Parsed date: {}", parsed_date);
                            } else {
                                info!("Could not parse date from: '{}'", date_text);
                            }
                        }
                        "p" => {
                            // Check if this paragraph contains band names and we have a current date
                            if let Some(date) = current_date {
                                let p_text = element_ref.text().collect::<String>();
                                
                                if self.is_band_paragraph(&p_text) {
                                    let bands = self.extract_bands_from_paragraph(&p_text);
                                    if !bands.is_empty() {
                                        info!("Found {} bands for date {}: {:?}", bands.len(), date, bands);
                                        
                                        let event = self.create_event_json(&date, &bands, &current_date_text)?;
                                        info!("Created event: {:?}", event);
                                        events.push(event);
                                        
                                        // Clear current date since we've processed this event
                                        current_date = None;
                                        current_date_text.clear();
                                    }
                                }
                            }
                        }
                        _ => {
                            // Skip other elements
                        }
                    }
                }
            }
            
            // Handle the last event if we have a date but no bands were found
            if let Some(date) = current_date {
                info!("Last date {} had no bands found", current_date_text);
            }
            
        } else {
            info!("Could not find .entry-content div - this may not be HTML");
        }
        
        info!("Total events parsed: {}", events.len());
        Ok(events)
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
            venue_name: DARRELLS_TAVERN_VENUE_NAME.to_string(),
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

impl DarrellsTavernParser {
    fn find_next_band_paragraph<'a>(&self, _h1_elem: &scraper::ElementRef, p_elements: &[scraper::ElementRef<'a>]) -> Option<scraper::ElementRef<'a>> {
        // Simple approach: find the first p element that contains band-like content
        for p_elem in p_elements {
            let p_text = p_elem.text().collect::<String>();
            if !p_text.trim().is_empty() && 
               !p_text.contains("OPEN 4PM") && 
               !p_text.contains("FOR BOOKING") &&
               !p_text.contains("SEVEN DAYS") &&
               !p_text.contains("SUGGESTED DONATION") &&
               !p_text.contains("HOSTED BY") {
                return Some(*p_elem);
            }
        }
        None
    }

    fn parse_date(&self, date_text: &str) -> Option<chrono::NaiveDate> {
        use regex::Regex;
        
        // Parse dates like "THU 08.28", "FRI 09.05", "SAT 10.18"
        let date_regex = Regex::new(r"(?i)(?:mon|tue|wed|thu|fri|sat|sun)\s+(\d{2})\.(\d{2})").unwrap();
        
        if let Some(captures) = date_regex.captures(date_text) {
            let month: u32 = captures[1].parse().ok()?;
            let day: u32 = captures[2].parse().ok()?;
            
            // Assume current year for now - could be enhanced to handle year transitions
            let current_year = chrono::Utc::now().year();
            
            chrono::NaiveDate::from_ymd_opt(current_year, month, day)
        } else {
            None
        }
    }
    
    fn is_band_paragraph(&self, text: &str) -> bool {
        let text = text.trim();
        !text.is_empty() && 
        !text.contains("OPEN 4PM") && 
        !text.contains("FOR BOOKING") &&
        !text.contains("SEVEN DAYS") &&
        !text.contains("SUGGESTED DONATION") &&
        !text.contains("HOSTED BY") &&
        !text.contains("(unless otherwise noted)") &&
        !text.contains("THURSDAY SHOWS") &&
        !text.contains("FRI/SAT SHOWS") &&
        // Check if it contains band-like content (has links or uppercase names)
        (text.contains("http") || text.chars().any(|c| c.is_uppercase()))
    }

    fn extract_bands_from_paragraph(&self, text: &str) -> Vec<String> {
        // Split by line breaks and filter out empty lines
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .filter(|line| !line.contains("$") && !line.contains("DOORS") && !line.contains("SHOW"))
            .map(|line| {
                // Remove HTML tags and clean up band names
                let clean_line = line
                    .replace("&amp;", "&")
                    .replace("&#8211;", "-")
                    .replace("&#039;", "'");
                
                // If it looks like a band name, extract it
                if !clean_line.is_empty() {
                    clean_line
                } else {
                    line.to_string()
                }
            })
            .filter(|band| !band.is_empty())
            .collect()
    }
    
    fn create_event_json(
        &self,
        date: &chrono::NaiveDate,
        bands: &[String],
        details: &str,
    ) -> Result<RawEventData> {
        let title = if bands.len() == 1 {
            bands[0].clone()
        } else {
            bands.join(" / ")
        };
        
        let event_id = format!("darrells_{}", date.format("%Y_%m_%d"));
        
        Ok(json!({
            "id": event_id,
            "title": title,
            "event_day": date.format("%Y-%m-%d").to_string(),
            "bands": bands,
            "details": details,
            "venue": DARRELLS_TAVERN_VENUE_NAME
        }))
    }
}
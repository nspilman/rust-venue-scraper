use crate::common::constants::{NEUMOS_API, NEUMOS_VENUE_NAME};
use crate::common::error::{Result, ScraperError};
use crate::pipeline::ingestion::ingest_common::fetch_payload_and_log;
use crate::common::types::{EventApi, EventArgs, RawDataInfo, RawEventData};
use chrono::{Datelike, NaiveDate, NaiveTime};
use scraper::{Html, Selector};
use serde_json::json;
use tracing::{debug, info, instrument};

pub struct NeumosCrawler {
    _client: reqwest::Client,
}

impl Default for NeumosCrawler {
    fn default() -> Self {
        Self::new()
    }
}

impl NeumosCrawler {
    pub fn new() -> Self {
        Self {
            _client: reqwest::Client::new(),
        }
    }

    /// Parse time from format like "Doors: 7:00 PM" or "8:00 PM"
    fn parse_time(time_str: &str) -> Option<NaiveTime> {
        // Remove "Doors: " prefix if present
        let cleaned = time_str.replace("Doors: ", "").replace("doors: ", "");
        
        // Try parsing formats like "7:00 PM" or "8:00 PM"
        NaiveTime::parse_from_str(&cleaned, "%I:%M %p").ok()
            .or_else(|| NaiveTime::parse_from_str(&cleaned, "%l:%M %p").ok())
    }

    /// Parse date from format like "Aug 29" or "Sep 5"
    fn parse_date(date_str: &str, year: i32) -> Option<NaiveDate> {
        // Split into month and day
        let parts: Vec<&str> = date_str.split_whitespace().collect();
        if parts.len() != 2 {
            return None;
        }

        let month_str = parts[0];
        let day_str = parts[1];

        // Parse month abbreviation
        let month = match month_str.to_lowercase().as_str() {
            "jan" => 1,
            "feb" => 2,
            "mar" => 3,
            "apr" => 4,
            "may" => 5,
            "jun" => 6,
            "jul" => 7,
            "aug" => 8,
            "sep" => 9,
            "oct" => 10,
            "nov" => 11,
            "dec" => 12,
            _ => return None,
        };

        // Parse day
        let day: u32 = day_str.parse().ok()?;

        NaiveDate::from_ymd_opt(year, month, day)
    }
}

#[async_trait::async_trait]
impl EventApi for NeumosCrawler {
    fn api_name(&self) -> &'static str {
        NEUMOS_API
    }

    #[instrument(skip(self))]
    async fn get_event_list(&self) -> Result<Vec<RawEventData>> {
        let payload = fetch_payload_and_log(NEUMOS_API).await?;
        let body = String::from_utf8_lossy(&payload).to_string();
        
        // Debug: Check if we have the expected content
        if body.contains("eventItem") {
            debug!("HTML contains 'eventItem' class");
        } else {
            debug!("Warning: HTML does not contain 'eventItem' class");
            debug!("First 500 chars of HTML: {}", &body[..body.len().min(500)]);
        }
        
        let document = Html::parse_document(&body);

        let mut events = Vec::new();
        
        // Parse events using the actual HTML structure: div.eventItem
        let event_selector = Selector::parse("div.eventItem").unwrap();
        let title_selector = Selector::parse("h3.title a").unwrap();
        let tagline_selector = Selector::parse("h4.tagline").unwrap();
        let tour_selector = Selector::parse("div.promotion-text.tour").unwrap();
        let promotion_selector = Selector::parse("div.promotion-text:not(.tour)").unwrap();
        let month_selector = Selector::parse(".m-date__month").unwrap();
        let day_selector = Selector::parse(".m-date__day").unwrap();
        let time_selector = Selector::parse(".meta .time").unwrap();
        let age_selector = Selector::parse(".meta .age").unwrap();
        let ticket_link_selector = Selector::parse("a.tickets").unwrap();
        let image_selector = Selector::parse(".thumb img").unwrap();

        // Get current year for date parsing
        let _current_year = chrono::Local::now().year();
        
        let event_elements: Vec<_> = document.select(&event_selector).collect();
        info!("Found {} event elements with selector 'div.eventItem'", event_elements.len());

        for event_element in event_elements {
            let mut event_data = json!({});
            
            // Extract main title (headliner)
            if let Some(title_elem) = event_element.select(&title_selector).next() {
                let title = title_elem.text().collect::<String>().trim().to_string();
                event_data["title"] = json!(title);
                
                // Also get the event detail URL
                if let Some(href) = title_elem.value().attr("href") {
                    event_data["detail_url"] = json!(href);
                    // Extract event ID from URL if possible
                    if let Some(id_match) = href.split('/').last() {
                        event_data["id"] = json!(id_match.to_string());
                    }
                }
            }

            // Extract tagline (supporting acts)
            if let Some(tagline_elem) = event_element.select(&tagline_selector).next() {
                let tagline = tagline_elem.text().collect::<String>().trim().to_string();
                if !tagline.is_empty() {
                    event_data["supporting_acts"] = json!(tagline);
                }
            }

            // Extract tour name if present
            if let Some(tour_elem) = event_element.select(&tour_selector).next() {
                let tour = tour_elem.text().collect::<String>().trim().to_string();
                if !tour.is_empty() {
                    event_data["tour_name"] = json!(tour);
                }
            }

            // Extract promotion text (e.g., "Neumos Presents")
            if let Some(promo_elem) = event_element.select(&promotion_selector).next() {
                let promo = promo_elem.text().collect::<String>().trim().to_string();
                // Only save if it's not a tour name
                if !promo.is_empty() && !event_element.select(&tour_selector).any(|t| t.text().collect::<String>().trim() == promo) {
                    event_data["promoter"] = json!(promo);
                }
            }

            // Extract date (month and day)
            let mut month_str = String::new();
            let mut day_str = String::new();
            
            if let Some(month_elem) = event_element.select(&month_selector).next() {
                month_str = month_elem.text().collect::<String>().trim().to_string();
            }
            
            if let Some(day_elem) = event_element.select(&day_selector).next() {
                day_str = day_elem.text().collect::<String>().trim().to_string();
            }
            
            if !month_str.is_empty() && !day_str.is_empty() {
                let date_text = format!("{} {}", month_str, day_str);
                event_data["date_text"] = json!(date_text);
            }

            // Extract time (e.g., "Doors: 7:00 PM")
            if let Some(time_elem) = event_element.select(&time_selector).next() {
                let time_text = time_elem.text().collect::<String>().trim().to_string();
                event_data["time_text"] = json!(time_text);
            }

            // Extract age restriction
            if let Some(age_elem) = event_element.select(&age_selector).next() {
                let age_text = age_elem.text().collect::<String>().trim().to_string();
                event_data["age_restriction"] = json!(age_text);
            }

            // Extract ticket purchase link
            if let Some(ticket_elem) = event_element.select(&ticket_link_selector).next() {
                if let Some(href) = ticket_elem.value().attr("href") {
                    event_data["ticket_url"] = json!(href);
                }
                // Check if tickets are on sale
                let class_attr = ticket_elem.value().attr("class").unwrap_or("");
                if class_attr.contains("onsalenow") {
                    event_data["tickets_on_sale"] = json!(true);
                } else {
                    event_data["tickets_on_sale"] = json!(false);
                }
            }

            // Extract event image
            if let Some(img_elem) = event_element.select(&image_selector).next() {
                if let Some(src) = img_elem.value().attr("src") {
                    event_data["image_url"] = json!(src);
                }
            }

            // Only add if we have at least a title or date
            if event_data.get("title").is_some() || event_data.get("date_text").is_some() {
                // If no ID was extracted, generate one
                if event_data.get("id").is_none() {
                    let title = event_data.get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown");
                    let date = event_data.get("date_text")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    event_data["id"] = json!(format!("neumos_{}_{}", 
                        title.to_lowercase().replace(" ", "_").replace("/", "_"),
                        date.to_lowercase().replace(" ", "_")
                    ));
                }
                events.push(event_data);
            }
        }

        info!(
            "Successfully fetched {} events from Neumos",
            events.len()
        );
        
        Ok(events)
    }

    fn get_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo> {
        // Extract date from the raw data
        let date_text = raw_data["date_text"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("date_text not found".into()))?;
        
        // Parse the date (assuming current year, but handle year boundary)
        let current_date = chrono::Local::now().naive_local().date();
        let current_year = current_date.year();
        let current_month = current_date.month();
        
        // First try with current year
        let mut event_day = Self::parse_date(date_text, current_year)
            .ok_or_else(|| ScraperError::Api {
                message: format!("Failed to parse date: {}", date_text),
            })?;
        
        // If the parsed month is earlier than current month and we're in the last quarter,
        // it's probably next year
        if event_day.month() < current_month && current_month >= 10 {
            event_day = Self::parse_date(date_text, current_year + 1)
                .ok_or_else(|| ScraperError::Api {
                    message: format!("Failed to parse date: {}", date_text),
                })?;
        }

        // Extract title or generate from date
        let title = raw_data["title"]
            .as_str()
            .unwrap_or_else(|| raw_data["id"].as_str().unwrap_or("Unknown Event"));

        // Generate event ID
        let event_api_id = raw_data["id"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("neumos_{}", event_day.format("%Y%m%d")));

        Ok(RawDataInfo {
            event_api_id,
            event_name: title.trim().to_string(),
            venue_name: NEUMOS_VENUE_NAME.to_string(),
            event_day,
        })
    }

    fn get_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs> {
        // Extract date
        let date_text = raw_data["date_text"]
            .as_str()
            .ok_or_else(|| ScraperError::MissingField("date_text not found".into()))?;
        
        // Parse the date with year boundary handling
        let current_date = chrono::Local::now().naive_local().date();
        let current_year = current_date.year();
        let current_month = current_date.month();
        
        let mut event_day = Self::parse_date(date_text, current_year)
            .ok_or_else(|| ScraperError::Api {
                message: format!("Failed to parse date: {}", date_text),
            })?;
        
        // Handle year boundary
        if event_day.month() < current_month && current_month >= 10 {
            event_day = Self::parse_date(date_text, current_year + 1)
                .ok_or_else(|| ScraperError::Api {
                    message: format!("Failed to parse date: {}", date_text),
                })?;
        }

        // Extract title (headliner)
        let title = raw_data["title"]
            .as_str()
            .unwrap_or_else(|| raw_data["id"].as_str().unwrap_or("TBA"))
            .trim()
            .to_string();

        // Build a full title that includes supporting acts if available
        let full_title = if let Some(supporting) = raw_data["supporting_acts"].as_str() {
            format!("{} with {}", title, supporting)
        } else {
            title.clone()
        };

        // Extract and parse time
        let start_time = raw_data["time_text"]
            .as_str()
            .and_then(Self::parse_time);

        // Extract ticket URL - prefer the ticket purchase link, fall back to detail URL
        let event_url = raw_data["ticket_url"]
            .as_str()
            .or_else(|| raw_data["detail_url"].as_str())
            .map(|s| {
                if s.starts_with("http") {
                    s.to_string()
                } else {
                    format!("https://www.neumos.com{}", s)
                }
            })
            .or_else(|| Some("https://www.neumos.com/events".to_string()));

        // Build description from available metadata
        let mut description_parts = Vec::new();
        
        if let Some(tour) = raw_data["tour_name"].as_str() {
            description_parts.push(format!("Tour: {}", tour));
        }
        
        if let Some(promoter) = raw_data["promoter"].as_str() {
            description_parts.push(promoter.to_string());
        }
        
        if let Some(age) = raw_data["age_restriction"].as_str() {
            description_parts.push(format!("Age: {}", age));
        }
        
        let description = if !description_parts.is_empty() {
            Some(description_parts.join(" | "))
        } else {
            raw_data["description"].as_str().map(|s| s.to_string())
        };

        // Extract image URL if available
        let event_image_url = raw_data["image_url"]
            .as_str()
            .map(|s| s.to_string());

        Ok(EventArgs {
            title: full_title,
            event_day,
            start_time,
            event_url,
            description,
            event_image_url,
        })
    }

    fn should_skip(&self, _raw_data: &RawEventData) -> (bool, String) {
        // For now, don't skip any events
        // Could add logic here to skip private events or special cases
        (false, String::new())
    }
}

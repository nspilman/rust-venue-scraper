use super::super::base::VenueParser;
use crate::common::constants::KEXP_VENUE_NAME;
use sms_core::common::error::{Result, ScraperError};
use sms_core::common::types::{EventArgs, RawDataInfo, RawEventData};
use scraper::{Html, Selector};
use serde_json::json;
use chrono::{NaiveDate, NaiveTime, Datelike};

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

    async fn parse_events(&self, payload: &[u8]) -> Result<Vec<RawEventData>> {
        let content = std::str::from_utf8(payload)
            .map_err(|e| ScraperError::Api {
                message: format!("Failed to parse content as UTF-8: {}", e),
            })?;

        // Debug: Check what format we're receiving
        tracing::info!("KEXP Parser content preview: {}", &content[..content.len().min(200)]);

        // Try to parse as JSON first (individual event objects)
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(content) {
            tracing::info!("Content is JSON, using as-is (already in correct format)");
            // The JSON is already in the correct format, just return it
            return Ok(vec![json_value]);
        }

        // If not JSON, treat as HTML
        tracing::info!("Content is HTML, parsing with scraper");
        let document = Html::parse_document(content);
        let event_selector = Selector::parse("article.aldryn-events-article")
            .map_err(|e| ScraperError::Api {
                message: format!("Failed to parse event selector: {:?}", e),
            })?;

        let mut events = Vec::new();

        for event_element in document.select(&event_selector) {
            // Extract event title and URL
            let title_selector = Selector::parse("h3 a")
                .map_err(|e| ScraperError::Api {
                    message: format!("Failed to parse title selector: {:?}", e),
                })?;
            
            let title_element = event_element.select(&title_selector).next();
            let (title, event_url) = if let Some(title_el) = title_element {
                let title = title_el.text().collect::<Vec<_>>().join(" ").trim().to_string();
                let url = title_el.value().attr("href").map(|href| {
                    if href.starts_with("/") {
                        format!("https://www.kexp.org{}", href)
                    } else {
                        href.to_string()
                    }
                });
                (title, url)
            } else {
                continue; // Skip events without titles
            };

            // Extract date from EventItem-DateTime
            let date_selector = Selector::parse(".EventItem-DateTime h3")
                .map_err(|e| ScraperError::Api {
                    message: format!("Failed to parse date selector: {:?}", e),
                })?;
            
            let date_text = event_element
                .select(&date_selector)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .unwrap_or_default();

            // Extract time
            let time_selector = Selector::parse(".EventItem-DateTime h5")
                .map_err(|e| ScraperError::Api {
                    message: format!("Failed to parse time selector: {:?}", e),
                })?;
            
            let time_text = event_element
                .select(&time_selector)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string());

            // Extract venue/location
            let venue_selector = Selector::parse(".u-h3.u-mb1.u-lightWeight a")
                .map_err(|e| ScraperError::Api {
                    message: format!("Failed to parse venue selector: {:?}", e),
                })?;
            
            let venue_text = event_element
                .select(&venue_selector)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .unwrap_or_else(|| "KEXP Studio".to_string());

            // Extract description
            let desc_selector = Selector::parse(".EventItem-description")
                .map_err(|e| ScraperError::Api {
                    message: format!("Failed to parse description selector: {:?}", e),
                })?;
            
            let description = event_element
                .select(&desc_selector)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string());

            // Extract image URL
            let img_selector = Selector::parse(".SquareImage-image")
                .map_err(|e| ScraperError::Api {
                    message: format!("Failed to parse image selector: {:?}", e),
                })?;
            
            let image_url = event_element
                .select(&img_selector)
                .next()
                .and_then(|img| img.value().attr("src"))
                .map(|src| {
                    if src.starts_with("/") {
                        format!("https://www.kexp.org{}", src)
                    } else {
                        src.to_string()
                    }
                });

            // Parse date - handle formats like "Aug 31st"
            let event_day = self.parse_date(&date_text)?;
            
            // Generate a unique ID from the event URL or title
            let event_id = event_url
                .as_ref()
                .and_then(|url| url.split('/').last())
                .unwrap_or(&title)
                .to_string();

            let event_data = json!({
                "id": event_id,
                "title": title,
                "event_day": event_day.format("%Y-%m-%d").to_string(),
                "start_time": time_text,
                "event_url": event_url,
                "venue": venue_text,
                "description": description,
                "event_image_url": image_url
            });

            events.push(event_data);
        }

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

        let start_time = raw_data["start_time"].as_str().and_then(|s| {
            // Parse time strings like "noon", "12:00", etc.
            if s.to_lowercase() == "noon" {
                Some(NaiveTime::from_hms_opt(12, 0, 0).unwrap())
            } else if let Ok(time) = NaiveTime::parse_from_str(s, "%H:%M") {
                Some(time)
            } else if let Ok(time) = NaiveTime::parse_from_str(s, "%I:%M %p") {
                Some(time)
            } else {
                None
            }
        });
        let event_url = raw_data["event_url"].as_str().map(|s| s.to_string());
        let description = raw_data["description"].as_str().map(|s| s.to_string());
        let event_image_url = raw_data["event_image_url"].as_str().map(|s| s.to_string());

        Ok(EventArgs {
            title: title.to_string(),
            event_day,
            start_time,
            event_url,
            description,
            event_image_url,
        })
    }
}

impl KexpParser {
    fn parse_date(&self, date_text: &str) -> Result<NaiveDate> {
        // Handle formats like "Aug 31st", "Sep 1st", etc.
        let current_year = chrono::Utc::now().year();
        
        // Remove ordinal suffixes (st, nd, rd, th)
        let cleaned_date = date_text
            .replace("st", "")
            .replace("nd", "")
            .replace("rd", "")
            .replace("th", "");
        
        // Try to parse with current year
        let date_with_year = format!("{} {}", cleaned_date, current_year);
        
        match NaiveDate::parse_from_str(&date_with_year, "%b %d %Y") {
            Ok(date) => {
                // If the parsed date is in the past, assume it's next year
                let today = chrono::Utc::now().date_naive();
                if date < today {
                    NaiveDate::parse_from_str(&format!("{} {}", cleaned_date, current_year + 1), "%b %d %Y")
                        .map_err(|e| ScraperError::Api {
                            message: format!("Failed to parse date '{}': {}", date_text, e),
                        })
                } else {
                    Ok(date)
                }
            }
            Err(e) => Err(ScraperError::Api {
                message: format!("Failed to parse date '{}': {}", date_text, e),
            })
        }
    }
}
use crate::constants::{DARRELLS_TAVERN_API, DARRELLS_TAVERN_VENUE_NAME};
use crate::error::{Result, ScraperError};
use crate::types::{EventApi, RawEventData, RawDataInfo, EventArgs};
use chrono::{Datelike, NaiveDate, NaiveTime};
use scraper::{Html, Selector};
use serde_json::Value;
use tracing::{info, warn};

pub struct DarrellsTavernCrawler {
    client: reqwest::Client,
}

impl DarrellsTavernCrawler {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    fn parse_date(&self, date_str: &str) -> Option<NaiveDate> {
        let parts: Vec<&str> = date_str.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let date_part = parts[1];
        let date_components: Vec<&str> = date_part.split('.').collect();
        if date_components.len() != 2 {
            return None;
        }

        let month: u32 = date_components[0].parse().ok()?;
        let day: u32 = date_components[1].parse().ok()?;
        let current_year = chrono::Utc::now().year();

        NaiveDate::from_ymd_opt(current_year, month, day)
    }

    fn extract_performers(&self, element: &scraper::ElementRef) -> Vec<String> {
        let link_selector = Selector::parse("a").unwrap();
        let mut performers = Vec::new();

        for link in element.select(&link_selector) {
            let performer_name = link.text().collect::<String>().trim().to_string();
            if !performer_name.is_empty() {
                performers.push(performer_name);
            }
        }

        let text_content = element.text().collect::<String>();
        let lines: Vec<&str> = text_content.split('\n').collect();
        for line in lines {
            let line = line.trim();
            if !line.is_empty() && !performers.iter().any(|p| line.contains(p)) {
                if !line.contains("DOORS") && !line.contains("SHOW") && !line.contains("$") {
                    performers.push(line.to_string());
                }
            }
        }

        performers
    }
}

#[async_trait::async_trait]
impl EventApi for DarrellsTavernCrawler {
    fn api_name(&self) -> &'static str {
        DARRELLS_TAVERN_API
    }


    async fn get_event_list(&self) -> Result<Vec<RawEventData>> {
        info!("Fetching events from Darrell's Tavern");

        let url = "https://darrellstavern.com/show-calendar/";
        let response = self.client.get(url).send().await?.text().await?;
        let document = Html::parse_document(&response);
        let entry_content_selector = Selector::parse("div.entry-content").unwrap();
        let mut events = Vec::new();

        if let Some(entry_content) = document.select(&entry_content_selector).next() {
            let mut current_date: Option<NaiveDate> = None;
            let nodes = entry_content.children().collect::<Vec<_>>();
            let mut i = 0;

            while i < nodes.len() {
                let node = nodes[i];
                
                if let Some(element) = node.value().as_element() {
                    if element.name() == "h1" {
                        let element_ref = scraper::ElementRef::wrap(node).unwrap();
                        let date_text = element_ref.text().collect::<String>();
                        current_date = self.parse_date(&date_text);
                    } else if element.name() == "p" && current_date.is_some() {
                        let element_ref = scraper::ElementRef::wrap(node).unwrap();
                        let performers = self.extract_performers(&element_ref);
                        
                        if !performers.is_empty() {
                            for performer in performers {
                                let mut event_data = serde_json::Map::new();
                                event_data.insert("title".to_string(), Value::String(performer.clone()));
                                event_data.insert("event_day".to_string(), Value::String(current_date.unwrap().to_string()));
                                events.push(Value::Object(event_data));
                            }
                        }
                    }
                }
                i += 1;
            }
        }

        info!("Parsed {} events from Darrell's Tavern", events.len());
        if events.is_empty() {
            warn!("No events found - the page structure may have changed");
        }

        Ok(events)
    }

    fn get_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo> {
        let title = raw_data["title"].as_str().ok_or_else(|| ScraperError::MissingField("title not found".into()))?;
        let event_day_str = raw_data["event_day"].as_str().ok_or_else(|| ScraperError::MissingField("event_day not found".into()))?;
        let event_day = chrono::NaiveDate::parse_from_str(event_day_str, "%Y-%m-%d")
            .map_err(|e| ScraperError::Api { message: format!("Failed to parse event_day: {}", e) })?;

        Ok(RawDataInfo {
            event_api_id: format!("{}_{}", title, event_day_str),
            event_name: title.to_string(),
            venue_name: DARRELLS_TAVERN_VENUE_NAME.to_string(),
            event_day,
        })
    }

    fn get_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs> {
        let title = raw_data["title"].as_str().ok_or_else(|| ScraperError::MissingField("title not found".into()))?;
        let event_day_str = raw_data["event_day"].as_str().ok_or_else(|| ScraperError::MissingField("event_day not found".into()))?;
        let event_day = chrono::NaiveDate::parse_from_str(event_day_str, "%Y-%m-%d")
            .map_err(|e| ScraperError::Api { message: format!("Failed to parse event_day: {}", e) })?;

        Ok(EventArgs {
            title: title.to_string(),
            event_day,
            start_time: Some(NaiveTime::from_hms_opt(19, 0, 0).unwrap()),
            event_url: None,
            description: None,
            event_image_url: None,
        })
    }
}


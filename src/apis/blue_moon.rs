use crate::constants::{BLUE_MOON_API, BLUE_MOON_VENUE_NAME};
use crate::error::{Result, ScraperError};
use crate::types::{EventApi, EventArgs, RawDataInfo, RawEventData};
use serde_json::Value;
use tracing::{debug, info, instrument};

const CALENDAR_EVENTS_URL: &str = "https://google-calendar.galilcloud.wixapps.net/_api/getEvents?compId=comp-kurk0gts&instance=Cvx_9zA6zBPvynb5y3Ufq9ti1OwqSvCBaRhAgM9XwtA.eyJpbnN0YW5jZUlkIjoiMDg2NDdkMmEtMmViMC00MzgwLWJmZGItNzA2ZGUzMTQ0ZjE0IiwiYXBwRGVmSWQiOiIxMjlhY2I0NC0yYzhhLTgzMTQtZmJjOC03M2Q1Yjk3M2E4OGYiLCJtZXRhU2l0ZUlkIjoiYjcwNTNhNmYtZDNiZC00Y2Y3LTk1MjUtMDRhYTdhZGZjNDc1Iiwic2lnbkRhdGUiOiIyMDIzLTExLTA3VDA3OjAyOjIzLjgxOFoiLCJkZW1vTW9kZSI6ZmFsc2UsImFpZCI6ImI1ZDQ1NDEwLTQ3NjktNGMwYS04MGE0LTdjYTNiNjBjMmI3NSIsImJpVG9rZW4iOiJiZjYxNDc0NS1mZDBkLTBmNzctMmFmZS03NGM3OTljYjhiNjEiLCJzaXRlT3duZXJJZCI6ImM0Mzc5Nzk2LWE5YzUtNDVkYi05MGIxLTE2OGZhZTQ0MTQ2NiJ9";

pub struct BlueMoonCrawler {
    client: reqwest::Client,
}

impl Default for BlueMoonCrawler {
    fn default() -> Self {
        Self::new()
    }
}

impl BlueMoonCrawler {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
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
        debug!("Fetching events from Blue Moon Tavern calendar API");
        let response = self.client.get(CALENDAR_EVENTS_URL).send().await?;
        let data: Value = response.json().await?;
        let events_by_date = data["eventsByDates"]
            .as_object()
            .ok_or_else(|| ScraperError::MissingField("eventsByDates not found".into()))?;

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

use sms_core::common::constants::{SUNSET_TAVERN_API, SUNSET_TAVERN_VENUE_NAME};
use sms_core::common::error::{Result, ScraperError};
use sms_core::common::types::{EventApi, EventArgs, RawDataInfo, RawEventData};
use chrono::{NaiveDate, NaiveTime};
use reqwest::Client;
use tracing::{info, instrument};

pub struct SunsetTavernCrawler {
    client: Client,
}

impl Default for SunsetTavernCrawler {
    fn default() -> Self {
        Self::new()
    }
}

impl SunsetTavernCrawler {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    const DICE_API_URL: &'static str = "https://partners-endpoint.dice.fm/api/v2/events?page%5Bsize%5D=&types=linkout%2Cevent&filter%5Bpromoters%5D%5B%5D=Bars+We+Like%2C+Inc+dba+Sunset+Tavern";
}

impl SunsetTavernCrawler {
    /// Extract authentication headers by visiting the shows page first
    async fn get_auth_headers(&self) -> Result<reqwest::header::HeaderMap> {
        info!("Visiting Sunset Tavern shows page to extract auth headers");
        
        let response = self
            .client
            .get("https://sunsettavern.com/shows/")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36")
            .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
            .header("accept-language", "en-US,en;q=0.9")
            .header("cache-control", "no-cache")
            .header("pragma", "no-cache")
            .header("sec-ch-ua", "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\"")
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", "\"macOS\"")
            .header("sec-fetch-dest", "document")
            .header("sec-fetch-mode", "navigate")
            .header("sec-fetch-site", "none")
            .header("sec-fetch-user", "?1")
            .header("upgrade-insecure-requests", "1")
            .send()
            .await
            .map_err(|e| ScraperError::Api { message: format!("Failed to fetch shows page: {}", e) })?;

        info!("Shows page response status: {}", response.status());
        
        if !response.status().is_success() {
            return Err(ScraperError::Api {
                message: format!("Shows page request failed with status: {}", response.status())
            });
        }

        // Extract cookies from the response
        let mut auth_headers = reqwest::header::HeaderMap::new();
        
        // Get cookies from the response headers
        for cookie_header in response.headers().get_all(reqwest::header::SET_COOKIE) {
            if let Ok(cookie_str) = cookie_header.to_str() {
                info!("Found cookie: {}", cookie_str);
                // Parse cookie and add to our headers for subsequent requests
                if let Some(cookie_value) = cookie_str.split(';').next() {
                    auth_headers.insert(
                        reqwest::header::COOKIE,
                        reqwest::header::HeaderValue::from_str(cookie_value)
                            .map_err(|e| ScraperError::Api { message: format!("Invalid cookie value: {}", e) })?
                    );
                }
            }
        }

        // Also parse the HTML to look for any embedded tokens or keys
        let html_content = response
            .text()
            .await
            .map_err(|e| ScraperError::Api { message: format!("Failed to read shows page content: {}", e) })?;

        info!("Shows page HTML length: {}", html_content.len());

        // Look for common authentication patterns in the HTML
        use regex::Regex;
        
        let patterns = vec![
            (r#"apiKey["']?\s*[:=]\s*["']([^"']+)["']"#, "X-API-Key"),
            (r#"authToken["']?\s*[:=]\s*["']([^"']+)["']"#, "Authorization"),
            (r#"csrfToken["']?\s*[:=]\s*["']([^"']+)["']"#, "X-CSRF-Token"),
            (r#"sessionId["']?\s*[:=]\s*["']([^"']+)["']"#, "X-Session-ID"),
        ];

        for (pattern, header_name) in patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(captures) = re.captures(&html_content) {
                    if let Some(token) = captures.get(1) {
                        info!("Found {} token: {}", header_name, token.as_str());
                        let header_name_lower = header_name.to_lowercase().replace('-', "_");
                        if let Ok(header_name_parsed) = reqwest::header::HeaderName::from_bytes(header_name_lower.as_bytes()) {
                            auth_headers.insert(
                                header_name_parsed,
                                reqwest::header::HeaderValue::from_str(token.as_str())
                                    .map_err(|e| ScraperError::Api { message: format!("Invalid token value: {}", e) })?
                            );
                        }
                    }
                }
            }
        }

        Ok(auth_headers)
    }
}

#[async_trait::async_trait]
impl EventApi for SunsetTavernCrawler {
    fn api_name(&self) -> &'static str {
        SUNSET_TAVERN_API
    }

    #[instrument(skip(self))]
    async fn get_event_list(&self) -> Result<Vec<RawEventData>> {
        // First, get authentication headers from the shows page
        let auth_headers = self.get_auth_headers().await?;
        
        info!("Fetching Sunset Tavern events from Dice API with auth headers");
        
        // Build the request with headers
        let mut request = self
            .client
            .get(Self::DICE_API_URL)
            .header("accept", "application/json")
            .header("accept-language", "en-US,en;q=0.5")
            .header("cache-control", "no-cache")
            .header("origin", "https://sunsettavern.com")
            .header("pragma", "no-cache")
            .header("referer", "https://sunsettavern.com/")
            .header("sec-ch-ua", r#""Not;A=Brand";v="99", "Brave";v="139", "Chromium";v="139""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("sec-ch-ua-platform", r#""macOS""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "cross-site")
            .header("sec-gpc", "1")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36");

        // Add extracted authentication headers
        for (header_name, header_value) in auth_headers.iter() {
            info!("Adding auth header: {:?} = {:?}", header_name, header_value);
            request = request.header(header_name, header_value);
        }

        // Make the API request
        let response = request
            .send()
            .await
            .map_err(|e| ScraperError::Api { message: format!("Failed to fetch events: {}", e) })?;

        let status = response.status();
        let headers = response.headers().clone();
        info!("Response status: {}", status);
        info!("Response headers: {:#?}", headers);

        if !status.is_success() {
            return Err(ScraperError::Api {
                message: format!("Dice API request failed with status: {}", status)
            });
        }

        let text = response
            .text()
            .await
            .map_err(|e| ScraperError::Api { message: format!("Failed to read response: {}", e) })?;

        info!("Response body:\n{}", text);

        // Parse the JSON response to extract events
        let json_value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| {
                info!("Raw response: {}", text);
                ScraperError::Json(e)
            })?;

        // Navigate to the events data
        let events = json_value
            .get("data")
            .ok_or_else(|| ScraperError::Api { message: "Invalid Dice API response structure - missing 'data'".to_string() })?;

        // Count events
        if let Some(events_array) = events.as_array() {
            info!("Found {} events from Sunset Tavern Dice API", events_array.len());
        }

        // Return the full API response as a single RawEventData
        // The parser will handle extracting individual events
        Ok(vec![json_value])
    }

    fn get_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo> {
        // Extract first event from the data array for basic info
        let first_event = raw_data
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| ScraperError::MissingField("No events in response".to_string()))?;

        let event_name = first_event
            .get("attributes")
            .and_then(|a| a.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown Event")
            .to_string();

        let date_str = first_event
            .get("attributes")
            .and_then(|a| a.get("date"))
            .and_then(|d| d.as_str())
            .ok_or_else(|| ScraperError::MissingField("date".to_string()))?;

        // Parse date - Dice API usually returns ISO format
        let event_day = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .or_else(|_| {
                // Try parsing as ISO datetime and extract date
                chrono::DateTime::parse_from_rfc3339(date_str)
                    .map(|dt| dt.date_naive())
            })
            .map_err(|e| ScraperError::Api { message: format!("Failed to parse date: {}", e) })?;

        Ok(RawDataInfo {
            event_api_id: SUNSET_TAVERN_API.to_string(),
            event_name,
            venue_name: SUNSET_TAVERN_VENUE_NAME.to_string(),
            event_day,
        })
    }

    fn get_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs> {
        // Extract first event for args
        let first_event = raw_data
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| ScraperError::MissingField("No events in response".to_string()))?;

        let title = first_event
            .get("attributes")
            .and_then(|a| a.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown Event")
            .to_string();

        let date_str = first_event
            .get("attributes")
            .and_then(|a| a.get("date"))
            .and_then(|d| d.as_str())
            .ok_or_else(|| ScraperError::MissingField("date".to_string()))?;

        let event_day = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .or_else(|_| {
                chrono::DateTime::parse_from_rfc3339(date_str)
                    .map(|dt| dt.date_naive())
            })
            .map_err(|e| ScraperError::Api { message: format!("Failed to parse date: {}", e) })?;

        let start_time = first_event
            .get("attributes")
            .and_then(|a| a.get("date"))
            .and_then(|d| d.as_str())
            .and_then(|d| {
                chrono::DateTime::parse_from_rfc3339(d)
                    .ok()
                    .map(|dt| dt.time())
            });

        let event_url = first_event
            .get("attributes")
            .and_then(|a| a.get("url"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        let description = first_event
            .get("attributes")
            .and_then(|a| a.get("description"))
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        let event_image_url = first_event
            .get("attributes")
            .and_then(|a| a.get("images"))
            .and_then(|i| i.as_array())
            .and_then(|arr| arr.first())
            .and_then(|img| img.get("url"))
            .and_then(|url| url.as_str())
            .map(|s| s.to_string());

        Ok(EventArgs {
            title,
            event_day,
            start_time,
            event_url,
            description,
            event_image_url,
        })
    }
}
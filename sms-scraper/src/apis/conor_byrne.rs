use crate::common::constants::{CONOR_BYRNE_API, CONOR_BYRNE_VENUE_NAME};
use crate::common::error::{Result, ScraperError};
use crate::common::types::{EventApi, EventArgs, RawDataInfo, RawEventData};
use chrono::{Duration, Local, NaiveDate, NaiveTime};
use reqwest::Client;
use serde_json::json;
use tracing::{info, instrument};

pub struct ConorByrneCrawler {
    client: Client,
}

impl Default for ConorByrneCrawler {
    fn default() -> Self {
        Self::new()
    }
}

impl ConorByrneCrawler {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    const GRAPHQL_URL: &'static str = "https://www.venuepilot.co/graphql";
    const ACCOUNT_ID: i32 = 194;

    const GRAPHQL_QUERY: &'static str = r#"query ($accountIds: [Int!]!, $startDate: String!, $endDate: String, $search: String, $searchScope: String, $limit: Int, $page: Int) {
  paginatedEvents(arguments: {accountIds: $accountIds, startDate: $startDate, endDate: $endDate, search: $search, searchScope: $searchScope, limit: $limit, page: $page}) {
    collection {
      id
      name
      date
      doorTime
      startTime
      endTime
      minimumAge
      promoter
      support
      description
      websiteUrl
      twitterUrl
      instagramUrl
      ...AnnounceImages
      status
      announceArtists {
        applemusic
        bandcamp
        facebook
        instagram
        lastfm
        name
        songkick
        spotify
        twitter
        website
        wikipedia
        youtube
        __typename
      }
      artists {
        bio
        createdAt
        id
        name
        updatedAt
        __typename
      }
      venue {
        name
        __typename
      }
      footerContent
      ticketsUrl
      __typename
    }
    metadata {
      currentPage
      limitValue
      totalCount
      totalPages
      __typename
    }
    __typename
  }
}

fragment AnnounceImages on PublicEvent {
  announceImages {
    name
    highlighted
    versions {
      thumb {
        src
        __typename
      }
      cover {
        src
        __typename
      }
      __typename
    }
    __typename
  }
  __typename
}"#;
}

impl ConorByrneCrawler {
    /// Extract authentication headers by visiting the events page first
    async fn get_auth_headers(&self) -> Result<reqwest::header::HeaderMap> {
        info!("Visiting Conor Byrne events page to extract auth headers");
        
        let response = self
            .client
            .get("https://www.conorbyrnepub.com/events#/events")
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
            .map_err(|e| ScraperError::Api { message: format!("Failed to fetch events page: {}", e) })?;

        info!("Events page response status: {}", response.status());
        
        if !response.status().is_success() {
            return Err(ScraperError::Api {
                message: format!("Events page request failed with status: {}", response.status())
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
            .map_err(|e| ScraperError::Api { message: format!("Failed to read events page content: {}", e) })?;

        info!("Events page HTML length: {}", html_content.len());

        // Look for common authentication patterns in the HTML
        // This might include embedded JavaScript variables, meta tags, or script tags
        use regex::Regex;
        
        // Look for common token patterns (you may need to adjust these based on what you find)
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
impl EventApi for ConorByrneCrawler {
    fn api_name(&self) -> &'static str {
        CONOR_BYRNE_API
    }

    #[instrument(skip(self))]
    async fn get_event_list(&self) -> Result<Vec<RawEventData>> {
        // First, get authentication headers from the events page
        let auth_headers = self.get_auth_headers().await?;
        
        // Get current date and date 3 months from now
        let today = Local::now().date_naive();
        let end_date = today + Duration::days(90);

        // Format dates as YYYY-MM-DD
        let start_date_str = today.format("%Y-%m-%d").to_string();
        let end_date_str = end_date.format("%Y-%m-%d").to_string();

        info!("Fetching Conor Byrne events from {} to {} with auth headers", start_date_str, end_date_str);
        
        // Build GraphQL request body - match the successful call exactly
        let request_body = json!({
            "operationName": null,
            "variables": {
                "accountIds": [Self::ACCOUNT_ID],
                "startDate": start_date_str,
                "endDate": null,  // Set to null like the successful call
                "search": "",
                "searchScope": "",
                "page": 1
                // Note: No limit parameter in the successful call
            },
            "query": Self::GRAPHQL_QUERY
        });

        info!("Request body: {}", serde_json::to_string_pretty(&request_body).unwrap());

        // Build the request with headers that match the successful call exactly
        let mut request = self
            .client
            .post(Self::GRAPHQL_URL)
            .header("accept", "*/*")
            .header("accept-language", "en-US,en;q=0.5")
            .header("cache-control", "no-cache")
            .header("content-type", "application/json")
            .header("origin", "https://www.conorbyrnepub.com")
            .header("pragma", "no-cache")
            .header("priority", "u=1, i")
            .header("referer", "https://www.conorbyrnepub.com/")
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

        // Make the GraphQL request
        let response = request
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ScraperError::Api { message: format!("Failed to fetch events: {}", e) })?;

        let status = response.status();
        let headers = response.headers().clone();
        info!("Response status: {}", status);
        info!("Response headers: {:#?}", headers);

        if !status.is_success() {
            return Err(ScraperError::Api {
                message: format!("GraphQL request failed with status: {}", status)
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

        // Navigate to the events collection
        let events = json_value
            .get("data")
            .and_then(|d| d.get("paginatedEvents"))
            .and_then(|p| p.get("collection"))
            .ok_or_else(|| ScraperError::Api { message: "Invalid GraphQL response structure".to_string() })?;

        // Count events
        if let Some(events_array) = events.as_array() {
            info!("Found {} events from Conor Byrne GraphQL API", events_array.len());
        }

        // Return the full GraphQL response as a single RawEventData
        // The parser will handle extracting individual events
        Ok(vec![json_value])
    }

    fn get_raw_data_info(&self, raw_data: &RawEventData) -> Result<RawDataInfo> {
        // Extract first event from the collection for basic info
        let first_event = raw_data
            .get("data")
            .and_then(|d| d.get("paginatedEvents"))
            .and_then(|p| p.get("collection"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| ScraperError::MissingField("No events in response".to_string()))?;

        let event_name = first_event
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown Event")
            .to_string();

        let date_str = first_event
            .get("date")
            .and_then(|d| d.as_str())
            .ok_or_else(|| ScraperError::MissingField("date".to_string()))?;

        let event_day = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| ScraperError::Api { message: format!("Failed to parse date: {}", e) })?;

        Ok(RawDataInfo {
            event_api_id: CONOR_BYRNE_API.to_string(),
            event_name,
            venue_name: CONOR_BYRNE_VENUE_NAME.to_string(),
            event_day,
        })
    }

    fn get_event_args(&self, raw_data: &RawEventData) -> Result<EventArgs> {
        // Extract first event for args
        let first_event = raw_data
            .get("data")
            .and_then(|d| d.get("paginatedEvents"))
            .and_then(|p| p.get("collection"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| ScraperError::MissingField("No events in response".to_string()))?;

        let title = first_event
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown Event")
            .to_string();

        let date_str = first_event
            .get("date")
            .and_then(|d| d.as_str())
            .ok_or_else(|| ScraperError::MissingField("date".to_string()))?;

        let event_day = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| ScraperError::Api { message: format!("Failed to parse date: {}", e) })?;

        let start_time = first_event
            .get("startTime")
            .and_then(|t| t.as_str())
            .and_then(|t| NaiveTime::parse_from_str(t, "%H:%M:%S").ok());

        let event_url = first_event
            .get("ticketsUrl")
            .and_then(|u| u.as_str())
            .map(|s| s.to_string());

        let description = first_event
            .get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        Ok(EventArgs {
            title,
            event_day,
            start_time,
            event_url,
            description,
            event_image_url: None,
        })
    }
}

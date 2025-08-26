use async_trait::async_trait;
use chrono::{Datelike, Duration, Local};
use reqwest::Client;
use serde_json::json;

use crate::{
    errors::ScraperError,
    scrapers::traits::{VenueCrawler, VenueInfo},
};

pub struct ConorByrneCrawler {
    client: Client,
}

impl ConorByrneCrawler {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    const GRAPHQL_URL: &'static str = "https://www.venuepilot.co/graphql";
    const ACCOUNT_ID: i32 = 194;

    const GRAPHQL_QUERY: &'static str = r#"
        query ($accountIds: [Int!]!, $startDate: String!, $endDate: String, $search: String, $searchScope: String, $limit: Int, $page: Int) {
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
                    }
                    artists {
                        bio
                        createdAt
                        id
                        name
                        updatedAt
                    }
                    venue {
                        name
                    }
                    footerContent
                    ticketsUrl
                }
                metadata {
                    currentPage
                    limitValue
                    totalCount
                    totalPages
                }
            }
        }
    "#;
}

#[async_trait]
impl VenueCrawler for ConorByrneCrawler {
    fn venue_info(&self) -> VenueInfo {
        VenueInfo {
            name: "Conor Byrne Pub".to_string(),
            slug: "conor-byrne".to_string(),
            url: "https://www.conorbyrnepub.com".to_string(),
        }
    }

    async fn fetch_events(&self) -> Result<String, ScraperError> {
        // Get current date and date 3 months from now
        let today = Local::now().date_naive();
        let end_date = today + Duration::days(90);

        // Format dates as YYYY-MM-DD
        let start_date_str = today.format("%Y-%m-%d").to_string();
        let end_date_str = end_date.format("%Y-%m-%d").to_string();

        // Build GraphQL request body
        let request_body = json!({
            "operationName": null,
            "variables": {
                "accountIds": [Self::ACCOUNT_ID],
                "startDate": start_date_str,
                "endDate": end_date_str,
                "search": "",
                "searchScope": "",
                "page": 1,
                "limit": 100  // Fetch up to 100 events
            },
            "query": Self::GRAPHQL_QUERY
        });

        // Make the GraphQL request
        let response = self
            .client
            .post(Self::GRAPHQL_URL)
            .header("accept", "*/*")
            .header("content-type", "application/json")
            .header("origin", "https://www.conorbyrnepub.com")
            .header("referer", "https://www.conorbyrnepub.com/")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ScraperError::FetchError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ScraperError::FetchError(format!(
                "GraphQL request failed with status: {}",
                response.status()
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| ScraperError::FetchError(e.to_string()))?;

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_events() {
        let crawler = ConorByrneCrawler::new();
        let result = crawler.fetch_events().await;
        assert!(result.is_ok());
        let json_data = result.unwrap();
        assert!(json_data.contains("paginatedEvents"));
    }
}

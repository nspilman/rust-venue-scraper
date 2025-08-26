use crate::models::{WebArtist, WebEvent, EventFilter, GraphQLRequest, WebVenue};
use crate::state::AppState;
use serde_json::json;
use serde::Deserialize;

#[derive(Deserialize)]
struct GqlResponse<T> {
    data: Option<T>,
    #[allow(dead_code)]
    errors: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct SearchEventsData {
    #[serde(rename = "searchEvents")]
    events: Vec<WebEvent>,
}

#[derive(Deserialize)]
struct UpcomingEventsData {
    #[serde(rename = "upcomingEvents")]
    events: Vec<WebEvent>,
}

pub async fn fetch_events(state: &AppState, filter: &EventFilter, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<WebEvent>, String> {
    let query_parts = vec![
        "id".to_string(),
        "title".to_string(),
        "eventDay".to_string(),
        "startTime".to_string(),
        "eventUrl".to_string(),
        "description".to_string(),
        "eventImageUrl".to_string(),
        "venue { id name address city }".to_string(),
        "artists { id name nameSlug bio artistImageUrl }".to_string(),
    ];

    let fields = query_parts.join("\n                ");

    let has_non_empty_filter = |opt: &Option<String>| opt.as_ref().map_or(false, |s| !s.trim().is_empty());

    let has_search = has_non_empty_filter(&filter.search);
    let has_venue = has_non_empty_filter(&filter.venue);

    // Use pagination-aware queries
    let default_limit = limit.unwrap_or(20); // Default to 20 events per page
    let default_offset = offset.unwrap_or(0);
    
    let query = if has_search || has_venue {
        format!(
            r#"
            query($search: String, $venue: String, $limit: Int, $offset: Int) {{
                searchEvents(search: $search, venue: $venue, limit: $limit, offset: $offset) {{
                    {}
                }}
            }}
            "#,
            fields
        )
    } else {
        // Use upcomingEvents for the main query as it has proper date logic
        format!(
            r#"
            query($days: Int) {{
                upcomingEvents(days: $days) {{
                    {}
                }}
            }}
            "#,
            fields
        )
    };

    let variables = if has_search || has_venue {
        let mut vars = serde_json::Map::new();
        if let Some(search) = &filter.search {
            if !search.trim().is_empty() {
                vars.insert("search".to_string(), serde_json::Value::String(search.clone()));
            }
        }
        if let Some(venue) = &filter.venue {
            if !venue.trim().is_empty() {
                vars.insert("venue".to_string(), serde_json::Value::String(venue.clone()));
            }
        }
        vars.insert("limit".to_string(), json!(default_limit));
        vars.insert("offset".to_string(), json!(default_offset));
        Some(serde_json::Value::Object(vars))
    } else {
        // days is optional in the schema; omit to use server default
        Some(json!({}))
    };

    let request = GraphQLRequest { query, variables };

    let response = state
        .graphql_client
        .post(&state.graphql_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to get response text: {}", e))?;

    // Deserialize into the expected typed shape based on the query we sent
    let mut events: Vec<WebEvent> = if has_search || has_venue {
        let parsed: GqlResponse<SearchEventsData> = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse GraphQL response: {} - Response: {}", e, response_text))?;
        if let Some(errs) = &parsed.errors {
            return Err(format!("GraphQL errors: {}", errs));
        }
        let data = parsed
            .data
            .ok_or_else(|| format!("No data in response - Response: {}", response_text))?;
        data.events
    } else {
        let parsed: GqlResponse<UpcomingEventsData> = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse GraphQL response: {} - Response: {}", e, response_text))?;
        if let Some(errs) = &parsed.errors {
            return Err(format!("GraphQL errors: {}", errs));
        }
        let data = parsed
            .data
            .ok_or_else(|| format!("No data in response - Response: {}", response_text))?;
        data.events
    };

    // Note: GraphQL API already filters for future events, so no additional filtering needed

    events.sort_by(|a, b| a.event_day.cmp(&b.event_day));

    Ok(events)
}

pub async fn fetch_artist(state: &AppState, artist_id: &str) -> Result<Option<WebArtist>, String> {
    let artist_query = r#"
        query($id: ID!) {
            artist(id: $id) {
                id
                name
                nameSlug
                bio
                artistImageUrl
            }
        }
    "#
    .to_string();

    let artist_request = GraphQLRequest {
        query: artist_query,
        variables: Some(json!({ "id": artist_id })),
    };

    let response = state
        .graphql_client
        .post(&state.graphql_url)
        .json(&artist_request)
        .send()
        .await
        .map_err(|e| format!("Error fetching artist: {}", e))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Error reading artist response: {}", e))?;

    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Error parsing artist JSON: {} - Response: {}", e, response_text))?;

    if let Some(errors) = response_json.get("errors") {
        return Err(format!("GraphQL errors: {}", errors));
    }

    let maybe_artist = response_json
        .get("data")
        .and_then(|data| data.get("artist"))
        .cloned();

    if let Some(artist_json) = maybe_artist {
        if artist_json.is_null() {
            Ok(None)
        } else {
            let artist: WebArtist = serde_json::from_value(artist_json)
                .map_err(|e| format!("Error deserializing artist: {}", e))?;
            Ok(Some(artist))
        }
    } else {
        Ok(None)
    }
}

pub async fn fetch_venue(state: &AppState, venue_id: &str) -> Result<Option<WebVenue>, String> {
    let venue_query = r#"
        query($id: ID!) {
            venue(id: $id) {
                id
                name
                address
                city
            }
        }
    "#
    .to_string();

    let venue_request = GraphQLRequest {
        query: venue_query,
        variables: Some(json!({ "id": venue_id })),
    };

    let response = state
        .graphql_client
        .post(&state.graphql_url)
        .json(&venue_request)
        .send()
        .await
        .map_err(|e| format!("Error fetching venue: {}", e))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Error reading venue response: {}", e))?;

    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Error parsing venue JSON: {} - Response: {}", e, response_text))?;

    if let Some(errors) = response_json.get("errors") {
        return Err(format!("GraphQL errors: {}", errors));
    }

    let maybe_venue = response_json
        .get("data")
        .and_then(|data| data.get("venue"))
        .cloned();

    if let Some(venue_json) = maybe_venue {
        if venue_json.is_null() {
            Ok(None)
        } else {
            let venue: WebVenue = serde_json::from_value(venue_json)
                .map_err(|e| format!("Error deserializing venue: {}", e))?;
            Ok(Some(venue))
        }
    } else {
        Ok(None)
    }
}

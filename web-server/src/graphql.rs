use crate::models::{Artist, Event, EventFilter, GraphQLRequest, Venue};
use crate::state::AppState;
use serde_json::json;
use chrono::Local;

pub async fn fetch_events(state: &AppState, filter: &EventFilter) -> Result<Vec<Event>, String> {
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

    // Use searchEvents when filters are provided; otherwise, ask the API for upcomingEvents
    let default_days = 365; // horizon for "future" events on the homepage
    let query = if has_search || has_venue {
        format!(
            r#"
            query($search: String, $venue: String) {{
                searchEvents(search: $search, venue: $venue) {{
                    {}
                }}
            }}
            "#,
            fields
        )
    } else {
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
        Some(serde_json::Value::Object(vars))
    } else {
        Some(json!({ "days": default_days }))
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

    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse JSON: {} - Response: {}", e, response_text))?;

    if let Some(errors) = response_json.get("errors") {
        return Err(format!("GraphQL errors: {}", errors));
    }

    let events_json = if let Some(data) = response_json.get("data") {
        if let Some(search_events) = data.get("searchEvents") {
            search_events
        } else if let Some(upcoming) = data.get("upcomingEvents") {
            upcoming
        } else if let Some(events) = data.get("events") {
            events
        } else {
            return Err("No events field found in response".to_string());
        }
    } else {
        return Err("No data field found in response".to_string());
    };

    let mut events: Vec<Event> = serde_json::from_value(events_json.clone())
        .map_err(|e| format!("Failed to deserialize events: {} - JSON: {}", e, events_json))?;

    // Keep only events from today onward
    let today = Local::now().date_naive();
    events.retain(|e| e.event_day >= today);

    events.sort_by(|a, b| a.event_day.cmp(&b.event_day));

    Ok(events)
}

pub async fn fetch_artist(state: &AppState, artist_id: &str) -> Result<Option<Artist>, String> {
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
            let artist: Artist = serde_json::from_value(artist_json)
                .map_err(|e| format!("Error deserializing artist: {}", e))?;
            Ok(Some(artist))
        }
    } else {
        Ok(None)
    }
}

pub async fn fetch_venue(state: &AppState, venue_id: &str) -> Result<Option<Venue>, String> {
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
            let venue: Venue = serde_json::from_value(venue_json)
                .map_err(|e| format!("Error deserializing venue: {}", e))?;
            Ok(Some(venue))
        }
    } else {
        Ok(None)
    }
}

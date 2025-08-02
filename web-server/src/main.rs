use axum::{
    extract::{Path, Query},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use askama::Template;
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;
use chrono::{NaiveDate, NaiveTime};

// GraphQL client for querying our API
use reqwest::Client;

// Event structure that matches the GraphQL response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Event {
    pub id: String,
    pub title: String,
    #[serde(rename = "eventDay")]
    pub event_day: NaiveDate,
    #[serde(rename = "startTime")]
    pub start_time: Option<NaiveTime>,
    #[serde(rename = "eventUrl")]
    pub event_url: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "eventImageUrl")]
    pub event_image_url: Option<String>,
    pub venue: Option<Venue>,
    pub artists: Vec<Artist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Venue {
    pub id: String,
    pub name: String,
    pub address: String,
    pub city: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Artist {
    pub id: String,
    pub name: String,
    #[serde(rename = "nameSlug")]
    pub name_slug: String,
    pub bio: Option<String>,
    #[serde(rename = "artistImageUrl")]
    pub artist_image_url: Option<String>,
}

// GraphQL request/response structures
#[derive(Serialize)]
struct GraphQLRequest {
    query: String,
    variables: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

#[derive(Deserialize)]
struct EventsResponse {
    events: Vec<Event>,
}

#[derive(Deserialize)]
struct ArtistResponse {
    artist: Option<Artist>,
}

#[derive(Deserialize)]
struct ArtistEventsResponse {
    #[serde(rename = "eventsByArtist")]
    events_by_artist: Vec<Event>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    events: Vec<Event>,
}

#[derive(Template)]
#[template(path = "events_list.html")]
struct EventsListTemplate {
    events: Vec<Event>,
}

#[derive(Template)]
#[template(path = "artist.html")]
struct ArtistTemplate {
    artist: Artist,
    events: Vec<Event>,
}

#[derive(Deserialize)]
struct EventFilter {
    venue: Option<String>,
    search: Option<String>,
}

#[derive(Clone)]
struct AppState {
    graphql_client: Client,
    graphql_url: String,
}

#[tokio::main]
async fn main() {
    // Create HTTP client for GraphQL requests
    let graphql_client = Client::new();
    let graphql_url = "http://127.0.0.1:8080/graphql".to_string();
    
    let app_state = AppState {
        graphql_client,
        graphql_url,
    };
    
    // Create the Axum app
    let app = Router::new()
        .route("/", get(index))
        .route("/events", get(events_htmx))
        .route("/events/search", post(search_events))
        .route("/artist/:id", get(artist_page))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(app_state);

    // Start the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    println!("Web server running on http://127.0.0.1:3000");
    println!("Make sure GraphQL server is running on http://127.0.0.1:8080/graphql");
    axum::serve(listener, app).await.unwrap();
}

async fn fetch_events(state: &AppState, filter: &EventFilter) -> Result<Vec<Event>, String> {
    // Build GraphQL query with search parameters
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
    
    // Helper function to check if a filter value is non-empty
    let has_non_empty_filter = |opt: &Option<String>| {
        opt.as_ref().map_or(false, |s| !s.trim().is_empty())
    };
    
    let has_search = has_non_empty_filter(&filter.search);
    let has_venue = has_non_empty_filter(&filter.venue);
    
    // Use searchEvents if we have search criteria, otherwise use events
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
            query {{
                events {{
                    {}
                }}
            }}
            "#,
            fields
        )
    };

    // Build variables for the query
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
        None
    };

    let request = GraphQLRequest {
        query,
        variables,
    };

    let response = state
        .graphql_client
        .post(&state.graphql_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let response_text = response.text().await.map_err(|e| format!("Failed to get response text: {}", e))?;
    
    // Parse the response
    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse JSON: {} - Response: {}", e, response_text))?;

    if let Some(errors) = response_json.get("errors") {
        return Err(format!("GraphQL errors: {}", errors));
    }

    // Extract events from either 'events' or 'searchEvents' field
    let events_json = if let Some(data) = response_json.get("data") {
        if let Some(search_events) = data.get("searchEvents") {
            search_events
        } else if let Some(events) = data.get("events") {
            events
        } else {
            return Err("No events field found in response".to_string());
        }
    } else {
        return Err("No data field found in response".to_string());
    };

    // Deserialize the events
    let mut events: Vec<Event> = serde_json::from_value(events_json.clone())
        .map_err(|e| format!("Failed to deserialize events: {} - JSON: {}", e, events_json))?;

    // Sort events by date (ascending)
    events.sort_by(|a, b| a.event_day.cmp(&b.event_day));

    Ok(events)
}

async fn index(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let empty_filter = EventFilter {
        search: None,
        venue: None,
    };
    
    match fetch_events(&state, &empty_filter).await {
        Ok(events) => {
            let template = IndexTemplate { events };
            Html(template.render().expect("Template rendering failed"))
        }
        Err(e) => {
            Html(format!("<h1>Error loading events: {}</h1>", e))
        }
    }
}

async fn events_htmx(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(filter): Query<EventFilter>,
) -> impl IntoResponse {
    // Server-side filtering - no client-side filtering needed
    let events = fetch_events(&state, &filter).await.unwrap_or_default();

    let template = EventsListTemplate { events };
    Html(template.render().expect("Template rendering failed"))
}

async fn search_events(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Form(filter): axum::extract::Form<EventFilter>,
) -> impl IntoResponse {
    events_htmx(
        axum::extract::State(state),
        Query(filter),
    ).await
}

async fn artist_page(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(artist_id): Path<String>,
) -> impl IntoResponse {
    // Fetch artist details
    let artist_query = format!(
        r#"
        query($id: ID!) {{
            artist(id: $id) {{
                id
                name
                nameSlug
                bio
                artistImageUrl
            }}
        }}
        "#
    );

    let mut artist_vars = serde_json::Map::new();
    artist_vars.insert("id".to_string(), serde_json::Value::String(artist_id.clone()));
    
    let artist_request = GraphQLRequest {
        query: artist_query,
        variables: Some(serde_json::Value::Object(artist_vars)),
    };

    // Make the request for artist data
    let artist_response = match state
        .graphql_client
        .post(&state.graphql_url)
        .json(&artist_request)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => return Html(format!("<h1>Error fetching artist: {}</h1>", e)),
    };

    let artist_response_text = match artist_response.text().await {
        Ok(text) => text,
        Err(e) => return Html(format!("<h1>Error reading artist response: {}</h1>", e)),
    };

    let artist_response_json: serde_json::Value = match serde_json::from_str(&artist_response_text) {
        Ok(json) => json,
        Err(e) => return Html(format!("<h1>Error parsing artist JSON: {} - Response: {}</h1>", e, artist_response_text)),
    };

    if let Some(errors) = artist_response_json.get("errors") {
        return Html(format!("<h1>GraphQL errors: {}</h1>", errors));
    }

    let artist = match artist_response_json
        .get("data")
        .and_then(|data| data.get("artist"))
    {
        Some(artist_json) if !artist_json.is_null() => {
            match serde_json::from_value::<Artist>(artist_json.clone()) {
                Ok(artist) => artist,
                Err(e) => return Html(format!("<h1>Error deserializing artist: {} - JSON: {}</h1>", e, artist_json)),
            }
        }
        _ => return Html("<h1>Artist not found</h1>".to_string()),
    };

    // Fetch events for this artist - for now, we'll filter client-side
    // In a real application, you'd want a proper GraphQL query for this
    let all_events = match fetch_events(&state, &EventFilter { search: None, venue: None }).await {
        Ok(events) => events,
        Err(e) => return Html(format!("<h1>Error fetching events: {}</h1>", e)),
    };

    // Filter events that include this artist
    let artist_events: Vec<Event> = all_events
        .into_iter()
        .filter(|event| event.artists.iter().any(|a| a.id == artist_id))
        .collect();

    let template = ArtistTemplate {
        artist,
        events: artist_events,
    };

    Html(template.render().expect("Template rendering failed"))
}

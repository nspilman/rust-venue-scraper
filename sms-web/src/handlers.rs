use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse},
};
use axum::http::HeaderMap;
use askama::Template;

use crate::graphql::{fetch_artist, fetch_events, fetch_venue_by_slug, fetch_venues};
use crate::models::{EventFilter, WebEvent};
use crate::state::AppState;
use crate::templates::{ArtistTemplate, EventsListTemplate, IndexTemplate, VenueTemplate, VenuesListTemplate};

pub async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let empty_filter = EventFilter {
        search: None,
        venue: None,
        limit: Some(20),
        offset: Some(0),
    };

    match fetch_events(&state, &empty_filter, empty_filter.limit, empty_filter.offset).await {
        Ok(events) => {
            let template = IndexTemplate { events };
            Html(template.render().expect("Template rendering failed"))
        }
        Err(e) => Html(format!("<h1>Error loading events: {}</h1>", e)),
    }
}

pub async fn events_htmx(
    State(state): State<AppState>,
    Query(filter): Query<EventFilter>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let events = fetch_events(&state, &filter, filter.limit, filter.offset).await.unwrap_or_default();

    // If it's an HTMX request, return just the partial. Otherwise, return the full page.
    let is_htmx = headers.get("HX-Request").is_some();
    if is_htmx {
        let template = EventsListTemplate { events };
        Html(template.render().expect("Template rendering failed"))
    } else {
        let template = IndexTemplate { events };
        Html(template.render().expect("Template rendering failed"))
    }
}

pub async fn search_events(
    State(state): State<AppState>,
    axum::extract::Form(filter): axum::extract::Form<EventFilter>,
) -> impl IntoResponse {
    // Treat form submissions like HTMX requests by setting the header manually
    let mut headers = HeaderMap::new();
    headers.insert("HX-Request", axum::http::HeaderValue::from_static("true"));
    events_htmx(State(state), Query(filter), headers).await
}

pub async fn artist_page(
    State(state): State<AppState>,
    Path(artist_id): Path<String>,
) -> impl IntoResponse {
    // Fetch artist details
    let artist = match fetch_artist(&state, &artist_id).await {
        Ok(Some(artist)) => artist,
        Ok(None) => return Html("<h1>Artist not found</h1>".to_string()),
        Err(e) => return Html(format!("<h1>Error fetching artist: {}</h1>", e)),
    };

    // Fetch all events and filter for this artist
    let filter = EventFilter { search: None, venue: None, limit: None, offset: None };
    let all_events = match fetch_events(&state, &filter, filter.limit, filter.offset).await {
        Ok(events) => events,
        Err(e) => return Html(format!("<h1>Error fetching events: {}</h1>", e)),
    };

    let artist_events: Vec<WebEvent> = all_events
        .into_iter()
        .filter(|event| event.artists.iter().any(|a| a.id == artist_id))
        .collect();

    let template = ArtistTemplate { artist, events: artist_events };

    Html(template.render().expect("Template rendering failed"))
}

pub async fn venues_list(State(state): State<AppState>) -> impl IntoResponse {
    match fetch_venues(&state).await {
        Ok(venues) => {
            let template = VenuesListTemplate { venues };
            Html(template.render().expect("Template rendering failed"))
        }
        Err(e) => Html(format!("<h1>Error loading venues: {}</h1>", e)),
    }
}

pub async fn venue_page(
    State(state): State<AppState>,
    Path(venue_slug): Path<String>,
) -> impl IntoResponse {
    // Fetch venue details by slug
    let venue = match fetch_venue_by_slug(&state, &venue_slug).await {
        Ok(Some(venue)) => venue,
        Ok(None) => return Html("<h1>Venue not found</h1>".to_string()),
        Err(e) => return Html(format!("<h1>Error fetching venue: {}</h1>", e)),
    };

    // Fetch all events and filter for this venue (still need to use venue ID for filtering)
    let filter = EventFilter { search: None, venue: None, limit: None, offset: None };
    let all_events = match fetch_events(&state, &filter, filter.limit, filter.offset).await {
        Ok(events) => events,
        Err(e) => return Html(format!("<h1>Error fetching events: {}</h1>", e)),
    };

    let venue_events: Vec<WebEvent> = all_events
        .into_iter()
        .filter(|event| event.venue.as_ref().map_or(false, |v| v.id == venue.id))
        .collect();

    let template = VenueTemplate { venue, events: venue_events };

    Html(template.render().expect("Template rendering failed"))
}

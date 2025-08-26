use axum::{routing::{get, post}, Router};
use tower_http::services::ServeDir;

use crate::handlers::{artist_page, events_htmx, index, search_events, venue_page, venues_list};
use crate::state::AppState;

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/events", get(events_htmx))
        .route("/events/search", post(search_events))
        .route("/venues", get(venues_list))
        .route("/artist/:id", get(artist_page))
        .route("/venue/:slug", get(venue_page))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}

// main.rs only boots the router and server

// New module declarations (code moved out of main.rs)
mod state;
mod models;
mod templates;
mod graphql;
mod handlers;
mod router;

// Bring shared state type into scope from module
use state::AppState;
use reqwest::Client;
use std::env;

#[tokio::main]
async fn main() {
    // Create HTTP client for GraphQL requests
    let graphql_client = Client::new();
    let graphql_url = env::var("GRAPHQL_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/graphql".to_string());

    let app_state = AppState {
        graphql_client,
        graphql_url,
    };

    // Build router from new router module
    let app = router::app_router(app_state);

    // Start the server
    let port: u16 = env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(3000);
    let bind_addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap();

    println!("Web server listening on {} (visit http://127.0.0.1:{})", bind_addr, port);
    println!("GraphQL server URL: {}", env::var("GRAPHQL_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/graphql".to_string()));
    axum::serve(listener, app).await.unwrap();
}
// handlers moved to crate::handlers

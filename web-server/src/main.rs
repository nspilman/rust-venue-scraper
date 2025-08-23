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

#[tokio::main]
async fn main() {
    // Create HTTP client for GraphQL requests
    let graphql_client = Client::new();
    let graphql_url = "http://127.0.0.1:8080/graphql".to_string();

    let app_state = AppState {
        graphql_client,
        graphql_url,
    };

    // Build router from new router module
    let app = router::app_router(app_state);

    // Start the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("Web server running on http://127.0.0.1:3000");
    println!("Make sure GraphQL server is running on http://127.0.0.1:8080/graphql");
    axum::serve(listener, app).await.unwrap();
}
// handlers moved to crate::handlers

use crate::graphql::{create_schema, GraphQLSchema};
use crate::storage::Storage;
use async_graphql::http::GraphiQLSource;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    http::Method,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

/// GraphQL endpoint handler
async fn graphql_handler(
    State(schema): State<GraphQLSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

/// GraphiQL playground handler
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/graphql").finish())
}

/// Health check endpoint
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "sms-scraper-graphql",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Create the GraphQL server with all routes
pub fn create_graphql_server(storage: Arc<dyn Storage>) -> Router {
    let schema = create_schema(storage);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    Router::new()
        .route("/graphql", post(graphql_handler))
        .route("/graphql", get(graphql_handler))
        .route("/graphiql", get(graphiql))
        .route("/playground", get(graphiql)) // Alternative endpoint name
        .route("/health", get(health))
        .layer(ServiceBuilder::new().layer(cors))
        .with_state(schema)
}

/// Start the GraphQL server on the specified port
pub async fn start_server(
    storage: Arc<dyn Storage>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_graphql_server(storage);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    println!("🚀 GraphQL server running on http://localhost:{port}");
    println!("📊 GraphQL endpoint: http://localhost:{port}/graphql");
    println!("🎮 GraphiQL playground: http://localhost:{port}/graphiql");
    println!("💚 Health check: http://localhost:{port}/health");

    axum::serve(listener, app).await?;

    Ok(())
}

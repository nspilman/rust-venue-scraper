use sms_core::storage::Storage;
use crate::graphql::schema::{create_schema, GraphQLSchema};

use axum::{
    response::{Html, IntoResponse, Json},
    routing::get,
    Extension, Router,
};
use std::sync::Arc;

/// Health check endpoint
async fn health() -> impl IntoResponse {
    "OK"
}

/// GraphiQL IDE endpoint
async fn graphiql() -> impl IntoResponse {
    Html(async_graphql::http::GraphiQLSource::build().endpoint("/graphql").finish())
}

/// GraphQL endpoint handler
async fn graphql_handler(
    Extension(schema): Extension<GraphQLSchema>,
    req: String,
) -> impl IntoResponse {
    let request = match serde_json::from_str::<async_graphql::Request>(&req) {
        Ok(req) => req,
        Err(_) => return Json(serde_json::json!({"error": "Invalid request"})),
    };
    
    let response = schema.execute(request).await;
    Json(serde_json::to_value(response).unwrap_or_default())
}

/// Create the HTTP server router
pub fn create_server(storage: Arc<dyn Storage>) -> Router {
    let schema = create_schema(storage.clone());

    Router::new()
        .route("/health", get(health))
        .route("/graphiql", get(graphiql))
        .route(
            "/graphql",
            get(graphiql).post(graphql_handler),
        )
        .layer(Extension(schema))
}

/// Start the HTTP server
pub async fn start_server(storage: Arc<dyn Storage>, port: u16) -> anyhow::Result<()> {
    let app = create_server(storage);
    let addr = format!("0.0.0.0:{}", port);
    
    println!("🚀 HTTP server running on http://{}", addr);
    println!("💚 Health check: http://{}/health", addr);
    println!("🔎 GraphQL:      http://{}/graphql", addr);
    println!("🧪 GraphiQL UI:  http://{}/graphiql", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
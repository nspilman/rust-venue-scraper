use crate::pipeline::storage::Storage;
use crate::pipeline::tasks::{
    gateway_once, parse_run, GatewayOnceParams, GatewayOnceResult, ParseParams, ParseResultSummary,
};
use axum::{
    http::Method,
    response::{IntoResponse, Json},
    routing::{get, post},
    Json as AxumJson, Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

/// Health check endpoint
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "sms-scraper-graphql",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Create the HTTP server with all routes (no GraphQL to reduce deps)
pub fn create_server(storage: Arc<dyn Storage>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        // Admin/task endpoints
        .route(
            "/admin/gateway-once",
            post({
                let st = storage.clone();
                move |AxumJson(p): AxumJson<GatewayOnceParams>| {
                    let st = st.clone();
                    async move {
                        match gateway_once(st, p).await {
                            Ok(res) => AxumJson::<GatewayOnceResult>(res).into_response(),
                            Err(e) => {
                                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                                    .into_response()
                            }
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/parse",
            post({
                let st = storage.clone();
                move |AxumJson(p): AxumJson<ParseParams>| {
                    let st = st.clone();
                    async move {
                        match parse_run(st, p).await {
                            Ok(res) => AxumJson::<ParseResultSummary>(res).into_response(),
                            Err(e) => {
                                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                                    .into_response()
                            }
                        }
                    }
                }
            }),
        )
        .layer(ServiceBuilder::new().layer(cors))
        .with_state(())
}

/// Start the GraphQL server on the specified port
pub async fn start_server(
    storage: Arc<dyn Storage>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_server(storage);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    println!("🚀 HTTP server running on http://localhost:{port}");
    println!("💚 Health check: http://localhost:{port}/health");

    axum::serve(listener, app).await?;

    Ok(())
}

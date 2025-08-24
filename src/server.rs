use crate::pipeline::storage::Storage;
use crate::pipeline::tasks::{
    gateway_once, parse_run, GatewayOnceParams, GatewayOnceResult, ParseParams, ParseResultSummary,
};
use axum::{
    http::Method,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Extension, Json as AxumJson, Router,
};
use hyper::Server;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

// GraphQL imports
use crate::graphql::{
    resolvers::Query,
    schema::{GraphQLContext, GraphQLSchema},
};
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};

/// Health check endpoint
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "sms-scraper-graphql",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// GraphQL handler (supports GET and POST)
async fn graphql_handler(
    Extension(schema): Extension<GraphQLSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

/// GraphiQL UI (pinned CDN versions to avoid upstream breaking changes)
async fn graphiql() -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>
<html lang=\"en\">
  <head>
    <meta charset=\"utf-8\" />
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
    <title>GraphiQL</title>
    <link id=\"graphiql-css\" rel=\"stylesheet\" href=\"/assets/graphiql/graphiql.min.css\" onerror=\"this.onerror=null;this.href='https://cdn.jsdelivr.net/npm/graphiql@2.7.5/graphiql.min.css'\" />
    <style>
      html, body, #graphiql { height: 100%; margin: 0; width: 100%; }
      .error { padding: 16px; font-family: sans-serif; color: #b00020; }
    </style>
  </head>
  <body>
    <div id=\"graphiql\"></div>
    <div id=\"error\" class=\"error\" style=\"display:none\"></div>
    <script>
      function loadScript(src, cb){
        var s=document.createElement('script');
        s.src=src; s.crossOrigin='anonymous';
        s.onload=function(){ cb && cb(); };
        s.onerror=function(){ cb && cb(new Error('load failed: '+src)); };
        document.head.appendChild(s);
      }
      function render(){
        try {
          const fetcher = GraphiQL.createFetcher({ url: '/graphql' });
          const root = ReactDOM.createRoot(document.getElementById('graphiql'));
          root.render(React.createElement(GraphiQL, { fetcher }));
        } catch (e) {
          var el=document.getElementById('error');
          el.style.display='block';
          el.textContent='Failed to initialize GraphiQL: '+e;
        }
      }
      function start(){
        if (window.GraphiQL && window.React && window.ReactDOM) { return render(); }
        // Prefer local assets, then fall back to CDNs
        loadScript('/assets/graphiql/react.production.min.js', function(){
          if (!window.React) return loadScript('https://cdn.jsdelivr.net/npm/react@18/umd/react.production.min.js', function(){});
        });
        loadScript('/assets/graphiql/react-dom.production.min.js', function(){
          if (!window.ReactDOM) return loadScript('https://cdn.jsdelivr.net/npm/react-dom@18/umd/react-dom.production.min.js', function(){});
        });
        loadScript('/assets/graphiql/graphiql.min.js', function(err){
          if (err || !window.GraphiQL){
            loadScript('https://cdn.jsdelivr.net/npm/graphiql@2.7.5/graphiql.min.js', function(){
              if (window.GraphiQL) render();
              else {
                var el=document.getElementById('error');
                el.style.display='block';
                el.textContent='Could not load GraphiQL assets from CDNs. Check network/CSP or use a different network.';
              }
            });
          } else {
            render();
          }
        });
      }
      // Kick off after DOM ready
      (document.readyState === 'loading') ? document.addEventListener('DOMContentLoaded', start) : start();
    </script>
  </body>
</html>"#;
    Html(html.to_string())
}

/// Create the HTTP server with all routes, including GraphQL
pub fn create_server(storage: Arc<dyn Storage>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    // Build GraphQL schema and attach storage in context
    let schema: GraphQLSchema = Schema::build(Query, EmptyMutation, EmptySubscription)
        .data(GraphQLContext { storage: storage.clone() })
        .finish();

    Router::new()
        .route("/health", get(health))
        // Serve local assets (GraphiQL JS/CSS)
        .nest_service("/assets", ServeDir::new("assets"))
        // GraphQL endpoints
        .route("/graphql", post(graphql_handler).get(graphql_handler))
        .route("/graphiql", get(graphiql))
        .layer(Extension(schema))
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
}

/// Start the HTTP server on the specified port
pub async fn start_server(
    storage: Arc<dyn Storage>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_server(storage);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("🚀 HTTP server running on http://localhost:{port}");
    println!("💚 Health check: http://localhost:{port}/health");
    println!("🔎 GraphQL:      http://localhost:{port}/graphql");
    println!("🧪 GraphiQL UI:  http://localhost:{port}/graphiql");

    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

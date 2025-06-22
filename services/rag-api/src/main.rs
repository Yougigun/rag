use anyhow::Result;
use axum::{
    extract::Json,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use xlib::app::{serve::serve_service, tracing::init_tracing};

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
    system_prompt: Option<String>,
    user_prompt: Option<String>,
    json_mode: Option<bool>,
}

#[derive(Serialize)]
struct QueryResponse {
    status: String,
    message: String,
    query: String,
}

async fn health_check() -> impl IntoResponse {
    Json(json!({"status": "ok", "service": "rag-api"}))
}

async fn query_handler(Json(payload): Json<QueryRequest>) -> impl IntoResponse {
    info!("Received query: {}", payload.query);

    let response = QueryResponse {
        status: "ok".to_string(),
        message: "Query processed successfully".to_string(),
        query: payload.query,
    };

    Json(response)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    init_tracing();

    info!("Starting RAG API service...");

    let app = Router::new()
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/query", post(query_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    serve_service(app, addr, "RAG API").await?;

    Ok(())
}

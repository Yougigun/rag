mod handlers;
mod models;

use anyhow::Result;
use axum::{
    extract::Json,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx;
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use xlib::{
    app::{serve::serve_service, tracing::init_tracing},
    client::{KafkaClient, KafkaClientConfig, PostgresClient, PostgresClientConfig},
};

use handlers::file_embedding_task::{
    create_task, delete_task, get_task, list_tasks, update_task,
};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::Pool<sqlx::Postgres>,
    pub kafka_client: std::sync::Arc<KafkaClient>,
}

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

    // Initialize database connection
    let postgres_config = PostgresClientConfig {
        hostname: std::env::var("DATABASE_HOSTNAME").unwrap_or_else(|_| "localhost".to_string()),
        port: std::env::var("DATABASE_PORT")
            .ok()
            .and_then(|p| p.parse().ok()),
        user: Some(std::env::var("DATABASE_USER").unwrap_or_else(|_| "raguser".to_string())),
        password: Some(std::env::var("DATABASE_PASSWORD").unwrap_or_else(|_| "ragpassword".to_string())),
        db_name: std::env::var("DATABASE_NAME").unwrap_or_else(|_| "rag".to_string()),
    };

    let postgres_client = PostgresClient::build(&postgres_config).await?;
    let pool = postgres_client.into_inner();

    // Initialize Kafka client
    let kafka_config = KafkaClientConfig {
        bootstrap_servers: std::env::var("KAFKA_BOOTSTRAP_SERVERS")
            .unwrap_or_else(|_| "localhost:9092".to_string()),
        group_id: Some("rag-api-group".to_string()),
    };

    let kafka_client = KafkaClient::new_with_retry(
        kafka_config, 
        5, // max retries
        std::time::Duration::from_secs(2) // retry delay
    ).await?;
    let kafka_client = std::sync::Arc::new(kafka_client);

    // Create application state
    let app_state = AppState {
        db_pool: pool,
        kafka_client,
    };

    let app = Router::new()
        // Health and query endpoints
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/query", post(query_handler))
        // Embedding task endpoints
        .route("/api/v1/embedding-tasks", post(create_task))
        .route("/api/v1/embedding-tasks", get(list_tasks))
        .route("/api/v1/embedding-tasks/{id}", get(get_task))
        .route("/api/v1/embedding-tasks/{id}", put(update_task))
        .route("/api/v1/embedding-tasks/{id}", delete(delete_task))
        .with_state(app_state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    serve_service(app, addr, "RAG API").await?;

    Ok(())
}

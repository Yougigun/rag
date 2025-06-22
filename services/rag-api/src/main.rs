use anyhow::Result;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, net::SocketAddr, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use xlib::{
    app::{serve::serve_service, tracing::init_tracing},
    client::{
        OpenAIClient, OpenAIClientConfig, PostgresClient, PostgresClientConfig, ChatMessage,
    },
};

#[derive(Clone)]
struct AppState {
    pub pg_client: Arc<PostgresClient>,
    pub openai_client: Arc<OpenAIClient>,
    pub qdrant_client: Arc<Qdrant>,
}

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
    system_prompt: Option<String>,
    user_prompt: Option<String>,
    json_mode: Option<bool>,
    api_endpoints: Option<Vec<String>>,
}

#[derive(Serialize)]
struct QueryResponse {
    response: String,
    sources: Vec<String>,
    retrieved_files: Vec<RetrievedFile>,
}

#[derive(Serialize)]
struct RetrievedFile {
    filename: String,
    similarity_score: f32,
    chunk_id: Option<i32>,
}

async fn health_check() -> impl IntoResponse {
    Json(json!({"status": "ok", "service": "rag-api"}))
}

async fn query_handler(
    State(state): State<AppState>,
    Json(payload): Json<QueryRequest>,
) -> Response {
    match process_query(state, payload).await {
        Ok(response) => Json(response).into_response(),
        Err(e) => {
            tracing::error!("Failed to process query: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to process query"})),
            )
                .into_response()
        }
    }
}

async fn process_query(state: AppState, request: QueryRequest) -> Result<QueryResponse> {
    // 1. Create embedding for the query
    let query_embedding = state
        .openai_client
        .create_embedding(&request.query)
        .await?;

    // 2. Search for similar documents in Qdrant
    let search_result = search_similar_documents(&state.qdrant_client, query_embedding, 5).await?;

    // 3. Retrieve document content from database
    let retrieved_files = retrieve_document_content(&state.pg_client, &search_result).await?;

    // 4. Build context from retrieved documents
    let context = build_context(&retrieved_files);

    // 5. Create OpenAI messages
    let mut messages = vec![];
    
    if let Some(system_prompt) = request.system_prompt {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        });
    }

    let user_content = if let Some(user_prompt) = request.user_prompt {
        format!("{}\n\nContext:\n{}\n\nQuery: {}", user_prompt, context, request.query)
    } else {
        format!("Context:\n{}\n\nQuery: {}", context, request.query)
    };

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: user_content,
    });

    // 6. Get OpenAI response
    let json_mode = request.json_mode.unwrap_or(false);
    let ai_response = state
        .openai_client
        .chat_completion(messages, json_mode)
        .await?;

    // 7. Build response
    let sources: Vec<String> = retrieved_files
        .iter()
        .map(|f| f.filename.clone())
        .collect();

    Ok(QueryResponse {
        response: ai_response,
        sources,
        retrieved_files,
    })
}

async fn search_similar_documents(
    qdrant_client: &Qdrant,
    query_embedding: Vec<f32>,
    limit: u64,
) -> Result<Vec<(String, f32)>> {
    use qdrant_client::qdrant::QueryPointsBuilder;

    let collection_name = "rag_documents";
    
    let query = QueryPointsBuilder::new(collection_name)
        .query(query_embedding)
        .limit(limit)
        .with_payload(true);

    let search_result = qdrant_client.query(query).await?;
    
    let mut results = Vec::new();
    for result in search_result.result {
        if let Some(payload) = result.payload.get("document_id") {
            if let Some(doc_id) = payload.as_str() {
                results.push((doc_id.to_string(), result.score));
            }
        }
    }
    
    Ok(results)
}

async fn retrieve_document_content(
    _pg_client: &PostgresClient,
    search_results: &[(String, f32)],
) -> Result<Vec<RetrievedFile>> {
    let mut retrieved_files = Vec::new();
    
    for (doc_id, score) in search_results {
        // In a real implementation, you'd query the database here
        // For now, we'll create mock data
        retrieved_files.push(RetrievedFile {
            filename: format!("document_{}.txt", doc_id),
            similarity_score: *score,
            chunk_id: Some(1),
        });
    }
    
    Ok(retrieved_files)
}

fn build_context(retrieved_files: &[RetrievedFile]) -> String {
    let mut context = String::new();
    
    for (i, file) in retrieved_files.iter().enumerate() {
        context.push_str(&format!(
            "[File {}: {}]\nSample content for {}...\n\n",
            i + 1,
            file.filename,
            file.filename
        ));
    }
    
    context
}

async fn init_clients() -> Result<AppState> {
    // Initialize database client
    let db_config = PostgresClientConfig {
        hostname: env::var("DATABASE_HOSTNAME").unwrap_or_else(|_| "localhost".to_string()),
        port: Some(5432),
        user: Some(env::var("DATABASE_USER").unwrap_or_else(|_| "postgres".to_string())),
        password: Some(env::var("DATABASE_PASSWORD").unwrap_or_else(|_| "password".to_string())),
        db_name: "rag".to_string(),
    };
    let pg_client = PostgresClient::build(&db_config).await?;

    // Initialize OpenAI client
    let openai_config = OpenAIClientConfig {
        api_key: env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY environment variable is required"),
        base_url: None,
    };
    let openai_client = OpenAIClient::new(openai_config)?;

    // Initialize Qdrant client
    let qdrant_url = env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());
    let qdrant_client = Qdrant::from_url(&qdrant_url).build()?;

    Ok(AppState {
        pg_client: Arc::new(pg_client),
        openai_client: Arc::new(openai_client),
        qdrant_client: Arc::new(qdrant_client),
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    init_tracing();

    info!("Starting RAG API service...");

    let state = init_clients().await?;

    let app = Router::new()
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/query", post(query_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    
    serve_service(app, addr, "RAG API").await?;

    Ok(())
} 
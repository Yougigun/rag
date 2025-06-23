use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use qdrant_client::qdrant::{SearchParamsBuilder, SearchPointsBuilder};
use reqwest;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::AppState;

#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<u64>,
}


#[derive(Serialize)]
pub struct SearchResult {
    pub score: f32,
    pub task_id: u64,
    pub file_name: String,
    pub content_snippet: String,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total_found: usize,
}

// OpenAI API structures (same as in file-processor)
#[derive(Serialize)]
struct EmbeddingRequest {
    input: String,
    model: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

const COLLECTION_NAME: &str = "rag-collection";

async fn generate_query_embedding(query: &str) -> Result<Vec<f32>> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable not set"))?;
        
    let client = reqwest::Client::new();
    
    let request_body = EmbeddingRequest {
        input: query.to_string(),
        model: "text-embedding-3-small".to_string(),
    };
    
    info!("🔍 Generating embedding for search query: '{}'", query);
    
    let response = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!("OpenAI API request failed with status {}: {}", status, error_text));
    }
    
    let embedding_response: EmbeddingResponse = response.json().await?;
    
    if let Some(embedding_data) = embedding_response.data.first() {
        let embedding = embedding_data.embedding.clone();
        info!("✅ Query embedding generated successfully ({} dimensions)", embedding.len());
        Ok(embedding)
    } else {
        Err(anyhow::anyhow!("No embedding data received from OpenAI API"))
    }
}

// Search endpoint with JSON body
pub async fn search_embeddings(
    State(app_state): State<AppState>,
    Json(search_request): Json<SearchRequest>,
) -> impl IntoResponse {
    info!("🔍 Search request received: '{}'", search_request.query);
    
    let limit = search_request.limit.unwrap_or(5);
    
    match perform_search(&app_state, &search_request.query, limit).await {
        Ok(response) => (StatusCode::OK, Json(response)),
        Err(e) => {
            error!("Search failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SearchResponse {
                    query: search_request.query,
                    results: vec![],
                    total_found: 0,
                }),
            )
        }
    }
}


async fn perform_search(app_state: &AppState, query: &str, limit: u64) -> Result<SearchResponse> {
    // Generate embedding for the search query
    let query_embedding = generate_query_embedding(query).await?;
    
    // Perform similarity search in Qdrant
    info!("🎯 Searching for similar embeddings in Qdrant...");
    
    let search_result = app_state
        .qdrant_client
        .search_points(
            SearchPointsBuilder::new(COLLECTION_NAME, query_embedding, limit)
                .with_payload(true)
                .params(SearchParamsBuilder::default()),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Qdrant search failed: {}", e))?;
    
    info!("📊 Found {} similar results", search_result.result.len());
    
    // Convert Qdrant results to our response format
    let mut results = Vec::new();
    let total_found = search_result.result.len();
    
    for point in search_result.result {
        let payload = point.payload;
        let task_id = payload.get("task_id")
            .and_then(|v| v.as_integer())
            .unwrap_or(0) as u64;
            
        let file_name = payload.get("file_name")
            .and_then(|v| v.as_str())
            .map_or_else(|| "unknown".to_string(), |s| s.to_string());
            
        let content_snippet = payload.get("content_snippet")
            .and_then(|v| v.as_str())
            .map_or_else(String::new, |s| s.to_string());
        
        results.push(SearchResult {
            score: point.score,
            task_id,
            file_name,
            content_snippet,
        });
    }
    
    Ok(SearchResponse {
        query: query.to_string(),
        results,
        total_found,
    })
}
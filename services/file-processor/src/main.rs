#![allow(clippy::redundant_pub_crate)]

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointStruct, UpsertPointsBuilder, VectorParamsBuilder,
};
use qdrant_client::Qdrant;
use uuid::Uuid;
use reqwest;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};
use xlib::{
    app::{graceful_shutdown::shutdown_signal, tracing::init_tracing},
    client::{KafkaClient, KafkaClientConfig},
};

#[derive(Serialize)]
struct EmbeddingRequest {
    input: String,
    model: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    model: String,
    usage: Usage,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: i32,
}

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: i32,
    total_tokens: i32,
}

#[derive(Serialize)]
struct UpdateTaskRequest {
    status: Option<String>,
    error_message: Option<String>,
    embedding_count: Option<i32>,
}

const COLLECTION_NAME: &str = "rag-collection";
const VECTOR_SIZE: u64 = 1536; // OpenAI text-embedding-3-small dimensions

async fn update_task_status(
    task_id: u64,
    status: &str,
    error_message: Option<String>,
    embedding_count: Option<i32>,
) -> Result<()> {
    let rag_api_url = std::env::var("RAG_API_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    
    let client = reqwest::Client::new();
    let update_request = UpdateTaskRequest {
        status: Some(status.to_string()),
        error_message,
        embedding_count,
    };
    
    let url = format!("{}/api/v1/embedding-tasks/{}", rag_api_url, task_id);
    info!("🔄 Updating task {} status to: {}", task_id, status);
    
    match client
        .put(&url)
        .header("Content-Type", "application/json")
        .json(&update_request)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                info!("✅ Successfully updated task {} status to {}", task_id, status);
                Ok(())
            } else {
                let status_code = response.status();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                warn!("⚠️ Failed to update task {} status: {} - {}", task_id, status_code, error_text);
                Err(anyhow::anyhow!("HTTP {} - {}", status_code, error_text))
            }
        }
        Err(e) => {
            warn!("⚠️ Failed to send status update request for task {}: {}", task_id, e);
            Err(anyhow::anyhow!("Request failed: {}", e))
        }
    }
}

async fn ensure_collection_exists(qdrant_client: &Qdrant) -> Result<()> {
    info!("🗄️ Checking if collection '{}' exists...", COLLECTION_NAME);

    // Check if collection exists
    match qdrant_client.collection_exists(COLLECTION_NAME).await {
        Ok(exists) => {
            if exists {
                info!("✅ Collection '{}' already exists", COLLECTION_NAME);
                return Ok(());
            }
        }
        Err(e) => {
            warn!("Failed to check collection existence: {}", e);
        }
    }

    // Create collection if it doesn't exist
    info!(
        "🏗️ Creating collection '{}' with {} dimensions...",
        COLLECTION_NAME, VECTOR_SIZE
    );

    qdrant_client
        .create_collection(
            CreateCollectionBuilder::new(COLLECTION_NAME)
                .vectors_config(VectorParamsBuilder::new(VECTOR_SIZE, Distance::Cosine)),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create collection: {}", e))?;

    info!("✅ Successfully created collection '{}'", COLLECTION_NAME);
    Ok(())
}

async fn store_embedding_in_qdrant(
    qdrant_client: &Qdrant,
    task_id: u64,
    embedding: Vec<f32>,
    file_name: String,
    content: String,
) -> Result<()> {
    info!("💾 Storing embedding for task {} in Qdrant...", task_id);

    // Create a truncated content snippet for metadata
    let content_snippet = if content.len() > 200 {
        format!("{}...", &content[..200])
    } else {
        content.clone()
    };

    // Generate a deterministic UUID from file_name only - same file will update existing embedding
    let point_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, file_name.as_bytes());
    
    let point = PointStruct::new(
        point_id.to_string(),
        embedding,
        [
            ("file_name", file_name.into()),
            ("task_id", (task_id as i64).into()),
            ("content_snippet", content_snippet.into()),
            ("full_content", content.into()),
        ],
    );

    qdrant_client
        .upsert_points(UpsertPointsBuilder::new(COLLECTION_NAME, vec![point]))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to store embedding in Qdrant: {}", e))?;

    info!(
        "✅ Successfully stored embedding for task {} in Qdrant",
        task_id
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    init_tracing();

    info!("Starting file processor worker...");

    // Initialize Kafka client
    let kafka_config = KafkaClientConfig {
        bootstrap_servers: std::env::var("KAFKA_BOOTSTRAP_SERVERS")
            .unwrap_or_else(|_| "localhost:9092".to_string()),
        group_id: Some("file-processor-group".to_string()),
    };

    let kafka_client = KafkaClient::new_with_retry(
        kafka_config,
        5,                                 // max retries
        std::time::Duration::from_secs(2), // retry delay
    )
    .await?;
    let kafka_client = std::sync::Arc::new(kafka_client);

    // Initialize Qdrant client
    let qdrant_url =
        std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());

    info!("Connecting to Qdrant at: {}", qdrant_url);
    let qdrant_client = std::sync::Arc::new(
        Qdrant::from_url(&qdrant_url)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to connect to Qdrant: {}", e))?
    );

    // Ensure collection exists
    ensure_collection_exists(&qdrant_client).await?;

    // Subscribe to the topic
    kafka_client
        .subscribe_to_topics(&["file-embedding-tasks"])
        .await?;

    info!("File processor subscribed to Kafka topics and ready to process messages");

    // Run indefinitely until shutdown signal
    tokio::select! {
        () = kafka_consumer_loop(&kafka_client, &qdrant_client) => {
            info!("Kafka consumer loop completed");
        }
        () = shutdown_signal() => {
            info!("Shutdown signal received");
        }
    }

    info!("File processor worker shutting down gracefully");
    Ok(())
}

async fn generate_embedding(text: &str) -> Result<Vec<f32>> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable not set"))?;

    let client = reqwest::Client::new();

    let request_body = EmbeddingRequest {
        input: text.to_string(),
        model: "text-embedding-3-small".to_string(),
    };

    info!("🤖 Generating embedding for text: '{}'", text);

    let response = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!(
            "OpenAI API request failed with status {}: {}",
            status,
            error_text
        ));
    }

    let embedding_response: EmbeddingResponse = response.json().await?;

    if let Some(embedding_data) = embedding_response.data.first() {
        let embedding = embedding_data.embedding.clone();

        info!("✅ Successfully generated embedding!");
        info!("📊 Model: {}", embedding_response.model);
        info!("🔢 Embedding dimensions: {}", embedding.len());
        info!(
            "💰 Token usage: {} prompt tokens, {} total tokens",
            embedding_response.usage.prompt_tokens, embedding_response.usage.total_tokens
        );

        // Print embedding summary instead of full vector
        let sample_values = if embedding.len() >= 3 {
            format!("{:.4}, {:.4}, {:.4}...", embedding[0], embedding[1], embedding[2])
        } else {
            format!("{:?}", embedding)
        };
        info!("🎯 Embedding vector summary: [{}] (length: {})", sample_values, embedding.len());

        Ok(embedding)
    } else {
        Err(anyhow::anyhow!(
            "No embedding data received from OpenAI API"
        ))
    }
}

async fn process_file_content(
    file_content: &str,
    task_id: u64,
    file_name: String,
    qdrant_client: &Qdrant,
) -> Result<()> {
    // Update status to processing\n    if let Err(e) = update_task_status(task_id, \"processing\", None, None).await {\n        warn!(\"Failed to update task {} to processing status: {}\", task_id, e);\n        // Continue processing even if status update fails\n    }\n\n    // Decode base64 content
    let decoded_bytes = general_purpose::STANDARD
        .decode(file_content)
        .map_err(|e| anyhow::anyhow!("Failed to decode base64 content: {}", e))?;

    // Convert to UTF-8 string
    let decoded_text = String::from_utf8(decoded_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to convert decoded bytes to UTF-8: {}", e))?;

    info!("📄 Successfully decoded file content: '{}'", decoded_text);
    info!("📝 Content length: {} characters", decoded_text.len());

    // Generate embedding
    let embedding = generate_embedding(&decoded_text).await?;
    info!("🎉 Embedding generation completed successfully!");
    info!("📊 Generated {} dimensional embedding", embedding.len());

    // Store in Qdrant
    store_embedding_in_qdrant(qdrant_client, task_id, embedding, file_name, decoded_text).await?;
    info!(
        "🎯 Successfully stored embedding in Qdrant for task {}",
        task_id
    );

    // Update task status to completed
    if let Err(e) = update_task_status(task_id, "completed", None, Some(1)).await {
        warn!("Failed to update task {} to completed status: {}", task_id, e);
    }

    Ok(())
}

async fn process_task_created_message(
    payload: &serde_json::Map<String, serde_json::Value>,
    qdrant_client: &Qdrant,
) -> Result<()> {
    let task_id = payload
        .get("task_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("Invalid or missing task_id"))?;

    let file_name = payload
        .get("file_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown_file")
        .to_string();

    let file_content = payload
        .get("file_content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No file_content found in message"))?;

    info!("🚀 Processing file embedding task {}", task_id);

    process_file_content(file_content, task_id, file_name, qdrant_client).await
}

async fn kafka_consumer_loop(kafka_client: &KafkaClient, qdrant_client: &Qdrant) {
    loop {
        match kafka_client.consume_message().await {
            Ok(Some(message)) => {
                info!("📨 Received Kafka message:");
                info!("  Event Type: {}", message.event_type);
                info!("  Timestamp: {}", message.timestamp);
                info!(
                    "  Payload: {}",
                    serde_json::to_string_pretty(&message.payload)
                        .unwrap_or_else(|_| "Invalid JSON".to_string())
                );

                if message.event_type == "task_created" {
                    // Convert serde_json::Value to Map if it's an object
                    if let Some(payload_map) = message.payload.as_object() {
                        if let Err(e) =
                            process_task_created_message(payload_map, qdrant_client).await
                        {
                            error!("Failed to process task_created message: {}", e);
                        }
                    } else {
                        error!("Message payload is not a JSON object");
                    }
                }
            }
            Ok(None) => {
                time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => {
                error!("Failed to consume Kafka message: {}", e);
                time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

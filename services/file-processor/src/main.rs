use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use reqwest;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time;
use tracing::{error, info};
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
        5, // max retries
        std::time::Duration::from_secs(2) // retry delay
    ).await?;
    
    // Subscribe to the topic
    kafka_client.subscribe_to_topics(&["file-embedding-tasks"]).await?;
    
    info!("File processor subscribed to Kafka topics and ready to process messages");

    // Run indefinitely until shutdown signal
    tokio::select! {
        _ = kafka_consumer_loop(&kafka_client) => {
            info!("Kafka consumer loop completed");
        }
        _ = shutdown_signal() => {
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
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!("OpenAI API request failed with status {}: {}", status, error_text));
    }
    
    let embedding_response: EmbeddingResponse = response.json().await?;
    
    if let Some(embedding_data) = embedding_response.data.first() {
        let embedding = embedding_data.embedding.clone();
        
        info!("✅ Successfully generated embedding!");
        info!("📊 Model: {}", embedding_response.model);
        info!("🔢 Embedding dimensions: {}", embedding.len());
        info!("💰 Token usage: {} prompt tokens, {} total tokens", 
              embedding_response.usage.prompt_tokens, 
              embedding_response.usage.total_tokens);
        
        // Print the embedding vector as requested
        info!("🎯 Embedding vector: {:?}", embedding);
        
        Ok(embedding)
    } else {
        Err(anyhow::anyhow!("No embedding data received from OpenAI API"))
    }
}

async fn kafka_consumer_loop(kafka_client: &KafkaClient) {
    loop {
        match kafka_client.consume_message().await {
            Ok(Some(message)) => {
                info!("📨 Received Kafka message:");
                info!("  Event Type: {}", message.event_type);
                info!("  Timestamp: {}", message.timestamp);
                info!("  Payload: {}", serde_json::to_string_pretty(&message.payload).unwrap_or_else(|_| "Invalid JSON".to_string()));
                
                // Process the message based on event type
                if message.event_type == "task_created" {
                    if let Some(task_id) = message.payload.get("task_id") {
                        info!("🚀 Processing file embedding task {}", task_id);
                        
                        // Extract file content if present
                        if let Some(file_content) = message.payload.get("file_content") {
                            if let Some(content_str) = file_content.as_str() {
                                match general_purpose::STANDARD.decode(content_str) {
                                    Ok(decoded_bytes) => {
                                        match String::from_utf8(decoded_bytes) {
                                            Ok(decoded_text) => {
                                                info!("📄 Successfully decoded file content: '{}'", decoded_text);
                                                info!("📝 Content length: {} characters", decoded_text.len());
                                                
                                                // Generate embedding for the decoded text
                                                match generate_embedding(&decoded_text).await {
                                                    Ok(embedding) => {
                                                        info!("🎉 Embedding generation completed successfully!");
                                                        info!("📊 Generated {} dimensional embedding", embedding.len());
                                                    }
                                                    Err(e) => {
                                                        error!("Failed to generate embedding: {}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!("Failed to convert decoded bytes to UTF-8: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to decode base64 content: {}", e);
                                    }
                                }
                            }
                        } else {
                            info!("⚠️ No file_content found in message");
                        }
                    }
                }
            }
            Ok(None) => {
                // No message received, continue
                time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => {
                error!("Failed to consume Kafka message: {}", e);
                time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

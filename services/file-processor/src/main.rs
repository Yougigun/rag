use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use std::time::Duration;
use tokio::time;
use tracing::{error, info};
use xlib::{
    app::{graceful_shutdown::shutdown_signal, tracing::init_tracing},
    client::{KafkaClient, KafkaClientConfig},
};

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

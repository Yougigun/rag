use anyhow::{Context, Result};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    producer::{FutureProducer, FutureRecord},
    Message,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

pub struct KafkaClient {
    producer: FutureProducer,
    consumer: StreamConsumer,
}

#[derive(Clone)]
pub struct KafkaClientConfig {
    pub bootstrap_servers: String,
    pub group_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KafkaMessage {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl KafkaClient {
    pub fn new(config: KafkaClientConfig) -> Result<Self> {
        // Producer configuration with better settings
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.bootstrap_servers)
            .set("message.timeout.ms", "10000")
            .set("request.timeout.ms", "5000")
            .set("delivery.timeout.ms", "15000")
            .set("retry.backoff.ms", "100")
            .set("reconnect.backoff.ms", "100")
            .set("reconnect.backoff.max.ms", "1000")
            .create()
            .context("Failed to create Kafka producer")?;

        // Consumer configuration with better settings
        let mut consumer_config = ClientConfig::new();
        consumer_config.set("bootstrap.servers", &config.bootstrap_servers);
        consumer_config.set("enable.partition.eof", "false");
        consumer_config.set("session.timeout.ms", "10000");
        consumer_config.set("heartbeat.interval.ms", "3000");
        consumer_config.set("enable.auto.commit", "true");
        consumer_config.set("auto.offset.reset", "latest");
        consumer_config.set("reconnect.backoff.ms", "100");
        consumer_config.set("reconnect.backoff.max.ms", "1000");
        
        if let Some(group_id) = config.group_id {
            consumer_config.set("group.id", group_id);
        } else {
            consumer_config.set("group.id", "rag-consumer-group");
        }

        let consumer: StreamConsumer = consumer_config
            .create()
            .context("Failed to create Kafka consumer")?;

        Ok(Self { producer, consumer })
    }

    pub async fn new_with_retry(config: KafkaClientConfig, max_retries: u32, retry_delay: Duration) -> Result<Self> {
        let mut last_error = None;
        
        for attempt in 1..=max_retries {
            match Self::new(config.clone()) {
                Ok(client) => {
                    info!("Kafka client connected successfully on attempt {}", attempt);
                    return Ok(client);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        warn!("Failed to connect to Kafka on attempt {} of {}: {}. Retrying in {:?}...", 
                              attempt, max_retries, last_error.as_ref().unwrap(), retry_delay);
                        sleep(retry_delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap().context(format!("Failed to connect to Kafka after {} attempts", max_retries)))
    }

    pub async fn produce_event(
        &self,
        topic: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<()> {
        let message = KafkaMessage {
            event_type: event_type.to_string(),
            payload,
            timestamp: chrono::Utc::now(),
        };

        let payload_str = serde_json::to_string(&message)
            .context("Failed to serialize message")?;

        let record = FutureRecord::to(topic)
            .key(&message.event_type)
            .payload(&payload_str);

        match self.producer.send(record, Duration::from_secs(0)).await {
            Ok(delivery) => {
                info!("Message delivered to topic '{}': {:?}", topic, delivery);
                Ok(())
            }
            Err((e, _)) => {
                error!("Failed to deliver message: {}", e);
                Err(anyhow::anyhow!("Failed to deliver message: {}", e))
            }
        }
    }

    pub async fn subscribe_to_topics(&self, topics: &[&str]) -> Result<()> {
        self.consumer
            .subscribe(topics)
            .context("Failed to subscribe to topics")?;
        
        info!("Subscribed to topics: {:?}", topics);
        Ok(())
    }

    pub async fn consume_message(&self) -> Result<Option<KafkaMessage>> {
        match self.consumer.recv().await {
            Ok(message) => {
                if let Some(payload) = message.payload_view::<str>() {
                    match payload {
                        Ok(payload_str) => {
                            match serde_json::from_str::<KafkaMessage>(payload_str) {
                                Ok(kafka_message) => {
                                    info!("Received message: {:?}", kafka_message);
                                    Ok(Some(kafka_message))
                                }
                                Err(e) => {
                                    error!("Failed to deserialize message: {}", e);
                                    Ok(None)
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse message payload: {}", e);
                            Ok(None)
                        }
                    }
                } else {
                    Ok(None)
                }
            }
            Err(e) => {
                error!("Failed to receive message: {}", e);
                Err(anyhow::anyhow!("Failed to receive message: {}", e))
            }
        }
    }
} 
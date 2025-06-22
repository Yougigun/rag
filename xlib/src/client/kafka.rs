use anyhow::{Context, Result};
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    producer::{FutureProducer, FutureRecord},
    Message,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info};

pub struct KafkaClient {
    producer: FutureProducer,
    consumer: StreamConsumer,
}

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
        // Producer configuration
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &config.bootstrap_servers)
            .set("message.timeout.ms", "5000")
            .create()
            .context("Failed to create Kafka producer")?;

        // Consumer configuration
        let mut consumer_config = ClientConfig::new();
        consumer_config.set("bootstrap.servers", &config.bootstrap_servers);
        consumer_config.set("enable.partition.eof", "false");
        consumer_config.set("session.timeout.ms", "6000");
        consumer_config.set("enable.auto.commit", "true");
        
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
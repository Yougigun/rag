#!/bin/bash

echo "Setting up Kafka topics..."

# Create the file-embedding-tasks topic
/opt/kafka/bin/kafka-topics.sh \
    --bootstrap-server broker:19092 \
    --create \
    --topic file-embedding-tasks \
    --partitions 1 \
    --replication-factor 1 \
    --if-not-exists

echo "Kafka topic setup completed successfully"
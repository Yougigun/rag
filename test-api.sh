#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== RAG API Load Testing Script ===${NC}"

# Check if k6 is installed
if ! command -v k6 &>/dev/null; then
    echo -e "${YELLOW}k6 is not installed. Installing k6...${NC}"

    # Install k6 based on the OS
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        if command -v brew &>/dev/null; then
            brew install k6
        else
            echo -e "${RED}Homebrew not found. Please install Homebrew first or install k6 manually.${NC}"
            echo "Visit: https://k6.io/docs/get-started/installation/"
            exit 1
        fi
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        # Linux
        sudo gpg -k
        sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
        echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
        sudo apt-get update
        sudo apt-get install k6
    else
        echo -e "${RED}Unsupported OS. Please install k6 manually.${NC}"
        echo "Visit: https://k6.io/docs/get-started/installation/"
        exit 1
    fi
fi

# Check if rag-api is running
echo -e "${BLUE}Checking if rag-api is running...${NC}"
if curl -s http://localhost:3000/api/v1/health >/dev/null; then
    echo -e "${GREEN}✓ rag-api is running${NC}"
else
    echo -e "${YELLOW}⚠ rag-api is not running. Starting it now...${NC}"

    # Start rag-api in background
    echo -e "${BLUE}Starting rag-api...${NC}"
    cd services/rag-api
    cargo run &
    RAG_API_PID=$!
    cd ../..

    # Wait for the service to start
    echo -e "${BLUE}Waiting for rag-api to start...${NC}"
    for i in {1..30}; do
        if curl -s http://localhost:3000/api/v1/health >/dev/null; then
            echo -e "${GREEN}✓ rag-api started successfully${NC}"
            break
        fi
        echo -n "."
        sleep 1
    done

    if ! curl -s http://localhost:3000/api/v1/health >/dev/null; then
        echo -e "${RED}✗ Failed to start rag-api${NC}"
        kill $RAG_API_PID 2>/dev/null
        exit 1
    fi
fi

# Run the load test
echo -e "${BLUE}Running k6 load test...${NC}"
k6 run load-test.js

# If we started rag-api, stop it
if [ ! -z "$RAG_API_PID" ]; then
    echo -e "${BLUE}Stopping rag-api...${NC}"
    kill $RAG_API_PID 2>/dev/null
    echo -e "${GREEN}✓ rag-api stopped${NC}"
fi

echo -e "${GREEN}Load test completed!${NC}"

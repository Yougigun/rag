# K6 Load Testing for RAG API

This directory contains k6 load testing scripts to verify and test the performance of the RAG API endpoints.

## Files

- `load-test.js` - Main k6 load testing script
- `test-api.sh` - Automated test runner script that handles service startup and k6 execution
- `README-k6.md` - This documentation file

## Quick Start

### Option 1: Automated Testing (Recommended)
```bash
# Run the automated test script (handles everything)
./test-api.sh
```

### Option 2: Manual Testing
```bash
# 1. Start the rag-api service
cd services/rag-api
cargo run &

# 2. Run k6 load test
k6 run load-test.js

# 3. Stop the service
pkill rag-api
```

## What the Load Test Does

The k6 script tests both RAG API endpoints:

### Health Check Endpoint
- **Method**: GET
- **URL**: `/api/v1/health`
- **Expected Response**: 
  ```json
  {
    "status": "ok",
    "service": "rag-api"
  }
  ```

### Query Endpoint
- **Method**: POST
- **URL**: `/api/v1/query`
- **Request Body**:
  ```json
  {
    "query": "What is the meaning of life?",
    "system_prompt": "You are a helpful assistant",
    "user_prompt": "Please answer the following question",
    "json_mode": false
  }
  ```
- **Expected Response**:
  ```json
  {
    "status": "ok",
    "message": "Query processed successfully",
    "query": "What is the meaning of life?"
  }
  ```

## Test Configuration

The load test is configured with:
- **Duration**: 50 seconds total
  - 10s ramp-up to 5 users
  - 30s sustained load with 5 users
  - 10s ramp-down to 0 users
- **Performance Thresholds**:
  - 95% of requests must complete under 500ms
  - Error rate must be below 10%
- **Test Pattern**: Each user makes requests to both endpoints with 1-second intervals

## Sample Output

```
=== K6 Load Test Summary ===
Total Requests: 412
Failed Requests: 0
Average Response Time: 0.73ms
95th Percentile: 2.21ms
Test Duration: 50.12s
```

## Requirements

- **k6**: Load testing tool (automatically installed by `test-api.sh` on macOS)
- **curl**: For endpoint validation
- **Rust/Cargo**: To build and run the rag-api service

## Installation

### macOS (Homebrew)
```bash
brew install k6
```

### Linux (Ubuntu/Debian)
```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

### Windows
Download from: https://k6.io/docs/get-started/installation/

## Customizing Tests

You can modify `load-test.js` to:
- Change the load pattern (users, duration, ramp-up/down)
- Add new endpoints to test
- Modify performance thresholds
- Add custom validation logic
- Change request payloads 
# RAG System - Retrieval-Augmented Generation

A modern RAG system built with Rust, featuring microservices architecture with Docker orchestration and k6 testing.

## 🏗️ Architecture

This project follows a microservices architecture with the following components:

### Services
- **rag-api**: REST API service with health check and query endpoints
- **file-processor**: Background worker service with graceful shutdown
- **xlib**: Shared library with common utilities (database, OpenAI, Kafka clients)

### Infrastructure
- **PostgreSQL**: Document metadata storage
- **Qdrant**: Vector database for similarity search
- **Kafka**: Event streaming for inter-service communication

## 🚀 Quick Start

### Prerequisites
- Docker and Docker Compose
- k6 (for testing)
- make (for running commands)

### Setup & Run

1. **Clone repository**:
   ```bash
   git clone <repository>
   cd rag
   ```

2. **Set up environment** (optional):
   ```bash
   # Create .env file if you need to customize environment variables
   echo "OPENAI_API_KEY=test_key" > .env
   ```

3. **Start everything**:
   ```bash
   make run
   ```
   This command will:
   - Build all Docker services
   - Start all infrastructure and application services
   - Wait for services to be ready
   - Show service URLs

4. **Database migrations** (automatic):
   Migrations in the `migrations/` folder are automatically applied during startup. You can also run them manually:
   ```bash
   make migrate
   ```

5. **Test the API**:
   ```bash
   make test
   ```

6. **Stop services**:
   ```bash
   make down
   ```

## 📁 Project Structure

```
rag/
├── services/
│   ├── rag-api/          # REST API service
│   └── file-processor/   # Document processing service
├── xlib/                 # Shared library
│   ├── src/app/         # Application utilities
│   └── src/client/      # Client implementations
├── documents/           # Sample documents directory
├── migrations/          # Database migration scripts
├── docker-compose.yaml  # Service orchestration
├── Makefile            # Development commands
├── .gitignore          # Git ignore rules
└── README.md
```


## 🔧 Development

### Available Commands

```bash
make help          # Show all available commands
make run           # Build and start all services (infrastructure + applications)
make migrate       # Run database migrations
make migrate-down  # Rollback database migrations
make test          # Run k6 load test against the API
make down          # Stop all services
make clean         # Stop services and remove volumes
```

### Manual Testing

Test individual endpoints:

```bash
# Health check
curl http://localhost:3000/api/v1/health

# Query endpoint
curl -X POST http://localhost:3000/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "How do I optimize database queries?",
    "system_prompt": "You are a senior database engineer.",
    "json_mode": false
  }'
```

### Local Development

To run services locally without Docker:

1. **Start infrastructure only**:
   ```bash
   docker compose up postgres qdrant kafka -d
   ```

2. **Run services locally**:
   ```bash
   # Terminal 1: API service
   cd services/rag-api
   cargo run

   # Terminal 2: File processor
   cd services/file-processor
   cargo run
   ```

## 📚 API Reference

### Endpoints

#### Health Check
```
GET /api/v1/health
```

Response:
```json
{
  "status": "ok",
  "service": "rag-api"
}
```

#### Query Documents
```
POST /api/v1/query
```

Request:
```json
{
  "query": "How do I optimize database queries?",
  "system_prompt": "You are a senior software engineer.",
  "user_prompt": "Based on the documentation, provide specific advice.",
  "json_mode": true
}
```

Response (currently mock implementation):
```json
{
  "status": "ok",
  "message": "Query processed successfully",
  "query": "How do I optimize database queries?"
}
```

#### Embedding Task Management

##### Create Embedding Task
```
POST /api/v1/embedding-tasks
```

Request:
```json
{
  "file_name": "sample-document.txt"
}
```

Response:
```json
{
  "id": 1,
  "file_name": "sample-document.txt",
  "status": "pending",
  "created_at": "2025-06-22T14:00:00Z",
  "updated_at": "2025-06-22T14:00:00Z",
  "started_at": null,
  "completed_at": null,
  "error_message": null,
  "embedding_count": null
}
```

##### List Embedding Tasks
```
GET /api/v1/embedding-tasks?status=pending&limit=10&offset=0
```

Query Parameters:
- `status` (optional): Filter by task status (pending, processing, completed, failed)
- `limit` (optional): Number of tasks to return (default: 50)
- `offset` (optional): Number of tasks to skip (default: 0)

Response:
```json
[
  {
    "id": 1,
    "file_name": "sample-document.txt",
    "status": "pending",
    "created_at": "2025-06-22T14:00:00Z",
    "updated_at": "2025-06-22T14:00:00Z",
    "started_at": null,
    "completed_at": null,
    "error_message": null,
    "embedding_count": null
  }
]
```

##### Get Embedding Task
```
GET /api/v1/embedding-tasks/{id}
```

Response:
```json
{
  "id": 1,
  "file_name": "sample-document.txt",
  "status": "processing",
  "created_at": "2025-06-22T14:00:00Z",
  "updated_at": "2025-06-22T14:05:00Z",
  "started_at": "2025-06-22T14:05:00Z",
  "completed_at": null,
  "error_message": null,
  "embedding_count": null
}
```

##### Update Embedding Task
```
PUT /api/v1/embedding-tasks/{id}
```

Request:
```json
{
  "status": "completed",
  "embedding_count": 150
}
```

Response:
```json
{
  "id": 1,
  "file_name": "sample-document.txt",
  "status": "completed",
  "created_at": "2025-06-22T14:00:00Z",
  "updated_at": "2025-06-22T14:10:00Z",
  "started_at": "2025-06-22T14:05:00Z",
  "completed_at": "2025-06-22T14:10:00Z",
  "error_message": null,
  "embedding_count": 150
}
```

##### Delete Embedding Task
```
DELETE /api/v1/embedding-tasks/{id}
```

Response: `204 No Content`

## 🗄️ Database Migrations

The project uses the [migrate/migrate](https://github.com/golang-migrate/migrate) tool for database schema management.

### Migration Commands

```bash
# Apply all pending migrations
make migrate

# Rollback the last migration
make migrate-down
```

### Adding New Migrations

1. Create migration files in the `migrations/` folder:
   ```bash
   # Create up migration
   touch migrations/001_create_users_table.up.sql
   
   # Create down migration  
   touch migrations/001_create_users_table.down.sql
   ```

2. Write your SQL:
   ```sql
   -- migrations/001_create_users_table.up.sql
   CREATE TABLE users (
       id SERIAL PRIMARY KEY,
       email VARCHAR(255) UNIQUE NOT NULL,
       created_at TIMESTAMP DEFAULT NOW()
   );
   
   -- migrations/001_create_users_table.down.sql
   DROP TABLE users;
   ```

3. Apply the migration:
   ```bash
   make migrate
   ```

### Migration Requirements

- PostgreSQL must be running (started by `make run`)
- Migration files must follow the naming pattern: `{version}_{description}.{up|down}.sql`
- Always create both `up` and `down` migrations for reversibility

## 🔧 Configuration

### Environment Variables

Environment variables are configured via `.env` file (optional):

```bash
# Optional (defaults provided in docker-compose.yaml)
OPENAI_API_KEY=test_key
DATABASE_HOSTNAME=postgres
DATABASE_USER=raguser
DATABASE_PASSWORD=ragpassword
KAFKA_BOOTSTRAP_SERVERS=kafka:9092
QDRANT_URL=http://qdrant:6333
DOCUMENTS_PATH=/documents
```

### Service URLs

When running with `make run`, services are available at:

- **RAG API**: http://localhost:3000
- **PostgreSQL**: localhost:5432
- **Qdrant**: http://localhost:6333  
- **Kafka**: localhost:9092

## 🛠️ Features

### ✅ Implemented
- Microservices architecture with Docker
- REST API with health check and query endpoints
- Infrastructure services (PostgreSQL, Qdrant, Kafka)
- Background worker service with graceful shutdown
- Shared library with client utilities
- k6 load testing setup
- Simple development workflow

### 🔄 Planned
- Actual RAG functionality (vector search, OpenAI integration)
- Document ingestion and processing
- Vector embeddings and similarity search
- File upload API
- Web frontend interface

## 🏛️ Architecture Details

### Current Implementation

1. **rag-api**: REST API service with Axum framework providing health check and query endpoints
2. **file-processor**: Background worker service that runs continuously with graceful shutdown
3. **xlib**: Shared library containing client utilities for PostgreSQL, Qdrant, Kafka, and OpenAI
4. **Infrastructure**: PostgreSQL, Qdrant, and Kafka services managed via Docker Compose

### Technology Stack

- **Language**: Rust 🦀
- **Web Framework**: Axum
- **Database**: PostgreSQL + SQLx (clients implemented)
- **Vector DB**: Qdrant (client implemented)
- **Message Queue**: Apache Kafka (client implemented)
- **AI**: OpenAI integration (client implemented)
- **Containerization**: Docker + Docker Compose
- **Testing**: k6 for API testing

## 🛠️ Testing

### k6 Load Testing

The project includes a simple k6 test that:
- Makes a single request to the health endpoint
- Verifies the API returns correct status and service name
- Provides clear pass/fail feedback

Run with: `make test`

### Manual Testing

Test endpoints directly:
```bash
# Health check
curl http://localhost:3000/api/v1/health

# Query endpoint (returns mock response)
curl -X POST http://localhost:3000/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "test query"}'

# Create embedding task
curl -X POST http://localhost:3000/api/v1/embedding-tasks \
  -H "Content-Type: application/json" \
  -d '{"file_name": "sample-document.txt"}'

# List embedding tasks
curl http://localhost:3000/api/v1/embedding-tasks

# Get specific embedding task
curl http://localhost:3000/api/v1/embedding-tasks/1

# Update embedding task
curl -X PUT http://localhost:3000/api/v1/embedding-tasks/1 \
  -H "Content-Type: application/json" \
  -d '{"status": "completed", "embedding_count": 150}'

# Delete embedding task
curl -X DELETE http://localhost:3000/api/v1/embedding-tasks/1
```

## 🚀 Development Workflow

1. **Start services**: `make run`
2. **Migrations**: Automatically applied during startup
3. **Test API**: `make test`
4. **View logs**: `docker compose logs -f`
5. **Stop services**: `make down`

## 🆘 Troubleshooting

### Common Issues

1. **Services not starting**:
   - Ensure Docker is running
   - Check ports 3000, 5432, 6333, 9092 are available
   - Run `docker compose ps` to check status

2. **k6 test failing**:
   - Ensure services are running: `make run`
   - Wait for services to be ready (handled automatically by `make run`)
   - Check API health: `curl http://localhost:3000/api/v1/health`

3. **Build issues**:
   - Clean up: `make clean`
   - Rebuild: `make run`

### Getting Help

- Check all services: `docker compose ps`
- View logs: `docker compose logs -f [service-name]`
- Test connectivity: `curl http://localhost:3000/api/v1/health` 
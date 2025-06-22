# RAG System - Retrieval-Augmented Generation

A modern RAG system built with Rust, featuring local text file retrieval, OpenAI GPT-4o integration, and event streaming with Kafka.

## 🏗️ Architecture

This project follows a microservices architecture with the following components:

### Services
- **rag-api**: REST API service handling query requests and responses
- **file-processor**: Background service for document ingestion and processing
- **xlib**: Shared library with common utilities (database, OpenAI, Kafka clients)

### Infrastructure
- **PostgreSQL**: Document metadata storage
- **Qdrant**: Vector database for similarity search
- **Kafka**: Event streaming for inter-service communication
- **OpenAI**: GPT-4o for text generation and embeddings

## 🚀 Quick Start

### Prerequisites
- Docker and Docker Compose
- OpenAI API key

### Setup

1. **Clone and setup environment**:
   ```bash
   git clone <repository>
   cd rag
   make setup  # Creates .env file
   ```

2. **Add your OpenAI API key**:
   ```bash
   # Edit .env file (created by make setup)
   OPENAI_API_KEY=your_actual_api_key_here
   ```
   
   **Note**: The `.env` file is git-ignored for security. Never commit API keys!

3. **Build and start services**:
   ```bash
   make build
   make run
   ```

4. **Check service status**:
   ```bash
   make status
   ```

### Testing

Test the API endpoints:
```bash
make test-api
```

Or manually:
```bash
# Health check
curl http://localhost:3000/api/v1/health

# Query example
curl -X POST http://localhost:3000/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "How do I optimize database queries?",
    "system_prompt": "You are a senior database engineer.",
    "json_mode": false
  }'
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
├── docker-compose.yaml  # Service orchestration
├── Makefile            # Development commands
├── .gitignore          # Git ignore rules
└── README.md
```

### Important Files
- **`.env`**: Environment variables (git-ignored, create with `make setup`)
- **`target/`**: Rust build artifacts (git-ignored)
- **`.gitignore`**: Excludes sensitive and generated files from version control

## 🔧 Development

### Available Commands

```bash
make help                # Show all available commands
make build              # Build all services
make run                # Start services in background
make run-logs           # Start services with logs
make down               # Stop services
make clean              # Stop services and remove volumes
make logs               # View all logs
make logs-api           # View API service logs
make logs-processor     # View file processor logs
make check              # Run cargo check
make test               # Run tests
make fmt                # Format code
make clippy             # Run clippy linter
```

### Local Development

To run services locally without Docker:

1. **Start infrastructure**:
   ```bash
   # Start only Postgres, Qdrant, and Kafka
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
  "json_mode": true,
  "api_endpoints": ["https://api.company.com/metrics"]
}
```

Response:
```json
{
  "response": "Based on the documentation...",
  "sources": ["database-optimization.txt", "sql-performance.md"],
  "retrieved_files": [
    {
      "filename": "database-optimization.txt",
      "similarity_score": 0.89,
      "chunk_id": 1
    }
  ]
}
```

## 🔧 Configuration

### Environment Variables

Create a `.env` file with:

```bash
# Required
OPENAI_API_KEY=your_openai_api_key_here

# Optional (defaults provided)
DATABASE_HOSTNAME=postgres
DATABASE_USER=raguser
DATABASE_PASSWORD=ragpassword
KAFKA_BOOTSTRAP_SERVERS=kafka:9092
QDRANT_URL=http://qdrant:6333
DOCUMENTS_PATH=/documents
```

### Adding Documents

Place `.txt` or `.md` files in the `documents/` directory. The file processor will automatically:

1. Detect new files
2. Chunk the content
3. Generate embeddings
4. Store in Qdrant vector database
5. Send processing events to Kafka

## 🛠️ Features

### ✅ Implemented
- Vector similarity search (top 5 files)
- OpenAI GPT-4o integration
- JSON and plain text response modes
- File ingestion and chunking
- Event streaming with Kafka
- System/user prompt support
- Graceful shutdown handling
- Comprehensive logging

### 🔄 Planned
- Web frontend interface
- File upload API
- API data integration
- Advanced chunking strategies
- Performance monitoring dashboard

## 🏛️ Architecture Details

### Data Flow

1. **Document Ingestion**: Files placed in `documents/` → File Processor
2. **Processing**: File Processor → Chunks → OpenAI Embeddings → Qdrant
3. **Query**: User Query → RAG API → Embedding → Qdrant Search
4. **Generation**: Retrieved Context + Query → OpenAI GPT-4o → Response
5. **Events**: All operations publish events to Kafka

### Technology Stack

- **Language**: Rust 🦀
- **Web Framework**: Axum
- **Database**: PostgreSQL + SQLx
- **Vector DB**: Qdrant
- **Message Queue**: Apache Kafka
- **AI**: OpenAI GPT-4o + text-embedding-3-small
- **Containerization**: Docker + Docker Compose

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes following the coding standards
4. Run tests: `make test`
5. Format code: `make fmt`
6. Run linter: `make clippy`
7. Submit a pull request

## 📝 License

This project is licensed under the MIT License.

## 🆘 Troubleshooting

### Common Issues

1. **OpenAI API Key not set**:
   - Ensure `OPENAI_API_KEY` is set in `.env`
   - Restart services after updating environment variables

2. **Services not starting**:
   - Check logs: `make logs`
   - Ensure all ports are available (3000, 5432, 6333, 9092)

3. **File processing not working**:
   - Check file processor logs: `make logs-processor`
   - Ensure documents are in correct format (.txt or .md)
   - Verify OpenAI API key is valid

4. **Vector search returns no results**:
   - Wait for file processing to complete
   - Check Qdrant collection: `curl http://localhost:6333/collections`

### Getting Help

- Check service logs: `make logs`
- Verify service status: `make status`
- Test API health: `curl http://localhost:3000/api/v1/health`

For more help, please open an issue in the repository. 
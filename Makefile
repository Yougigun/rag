# Build all services
.PHONY: build
build:
	@printf '\033[0;34m> Building services...\033[0m\n'
	docker compose build

# Start all services
.PHONY: run
run:
	@printf '\033[0;34m> Starting services...\033[0m\n'
	docker compose up -d

# Start services with logs
.PHONY: run-logs
run-logs:
	@printf '\033[0;34m> Starting services with logs...\033[0m\n'
	docker compose up

# Stop all services
.PHONY: down
down:
	@printf '\033[0;34m> Stopping services...\033[0m\n'
	docker compose down

# Stop all services and remove volumes
.PHONY: clean
clean:
	@printf '\033[0;34m> Cleaning up services and volumes...\033[0m\n'
	docker compose down --volumes --remove-orphans

# View logs
.PHONY: logs
logs:
	docker compose logs -f

# View specific service logs
.PHONY: logs-api
logs-api:
	docker compose logs -f rag-api

.PHONY: logs-processor
logs-processor:
	docker compose logs -f file-processor

.PHONY: logs-kafka
logs-kafka:
	docker compose logs -f kafka

# Check service status
.PHONY: status
status:
	docker compose ps

# Run cargo check on all workspace members
.PHONY: check
check:
	@printf '\033[0;34m> Running cargo check...\033[0m\n'
	cargo check --workspace

# Run cargo test on all workspace members
.PHONY: test
test:
	@printf '\033[0;34m> Running tests...\033[0m\n'
	cargo test --workspace

# Format code
.PHONY: fmt
fmt:
	@printf '\033[0;34m> Formatting code...\033[0m\n'
	cargo fmt --all

# Run clippy
.PHONY: clippy
clippy:
	@printf '\033[0;34m> Running clippy...\033[0m\n'
	cargo clippy --workspace -- -D warnings

# Setup environment (create .env from .env.example)
.PHONY: setup
setup:
	@printf '\033[0;34m> Setting up environment...\033[0m\n'
	@if [ ! -f .env ]; then \
		echo "Creating .env file..."; \
		echo "OPENAI_API_KEY=your_openai_api_key_here" > .env; \
		echo "Please update .env with your OpenAI API key"; \
	else \
		echo ".env file already exists"; \
	fi

# Test API endpoints
.PHONY: test-api
test-api:
	@printf '\033[0;34m> Testing API endpoints...\033[0m\n'
	@echo "Health check:"
	curl -s http://localhost:3000/api/v1/health | jq .
	@echo "\nQuery test:"
	curl -s -X POST http://localhost:3000/api/v1/query \
		-H "Content-Type: application/json" \
		-d '{"query": "How do I optimize database queries?"}' | jq .

# Help
.PHONY: help
help:
	@echo "Available commands:"
	@echo "  build       - Build all services"
	@echo "  run         - Start all services in background"
	@echo "  run-logs    - Start all services with logs"
	@echo "  down        - Stop all services"
	@echo "  clean       - Stop services and remove volumes"
	@echo "  logs        - View all service logs"
	@echo "  logs-api    - View API service logs"
	@echo "  logs-processor - View file processor logs"
	@echo "  logs-kafka  - View Kafka logs"
	@echo "  status      - Check service status"
	@echo "  check       - Run cargo check"
	@echo "  test        - Run tests"
	@echo "  fmt         - Format code"
	@echo "  clippy      - Run clippy"
	@echo "  setup       - Create .env file"
	@echo "  test-api    - Test API endpoints"
	@echo "  help        - Show this help" 
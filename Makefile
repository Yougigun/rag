# Run all services (infrastructure + applications)
.PHONY: run
run:
	@printf '\033[0;34m> Building and starting all services...\033[0m\n'
	docker compose up --build -d
	@printf '\033[0;34m> Waiting for services to be ready...\033[0m\n'
	@echo "Checking rag-api health..."
	@while ! curl -s http://localhost:3000/api/v1/health > /dev/null 2>&1; do \
		echo "Waiting for rag-api..."; \
		sleep 2; \
	done
	@echo "✓ rag-api is ready"
	@echo "Checking Qdrant..."
	@while ! curl -s http://localhost:6333/collections > /dev/null 2>&1; do \
		echo "Waiting for Qdrant..."; \
		sleep 2; \
	done
	@echo "✓ Qdrant is ready"
	@printf '\033[0;32m> All services are running and ready!\033[0m\n'
	@echo "Services available:"
	@echo "  - RAG API: http://localhost:3000"
	@echo "  - PostgreSQL: localhost:5432"
	@echo "  - Qdrant: http://localhost:6333"
	@echo "  - Kafka: localhost:9092"

# Test the API with k6
.PHONY: test
test:
	@printf '\033[0;34m> Running k6 load test...\033[0m\n'
	k6 run load-test.js

# Run database migrations
.PHONY: migrate
migrate:
	@printf '\033[0;34m> Running database migrations...\033[0m\n'
	docker run --rm -v $(PWD)/migrations:/migrations --network host migrate/migrate \
		-path=/migrations -database="postgres://raguser:ragpassword@localhost:5432/rag?sslmode=disable" up

# Rollback database migrations
.PHONY: migrate-down
migrate-down:
	@printf '\033[0;34m> Rolling back database migrations...\033[0m\n'
	docker run --rm -v $(PWD)/migrations:/migrations --network host migrate/migrate \
		-path=/migrations -database="postgres://raguser:ragpassword@localhost:5432/rag?sslmode=disable" down 1

# Stop all services
.PHONY: down
down:
	@printf '\033[0;34m> Stopping services...\033[0m\n'
	docker compose down

# Clean up (stop services and remove volumes)
.PHONY: clean
clean:
	@printf '\033[0;34m> Cleaning up services and volumes...\033[0m\n'
	docker compose down --volumes --remove-orphans

# Show help
.PHONY: help
help:
	@echo "🚀 RAG System - Simple Commands:"
	@echo ""
	@echo "  run          - Build and start all services (infrastructure + applications)"
	@echo "  migrate      - Run database migrations"
	@echo "  migrate-down - Rollback database migrations"
	@echo "  test         - Run k6 load test against the API"
	@echo "  down         - Stop all services"
	@echo "  clean        - Stop services and remove volumes"
	@echo "  help         - Show this help"
	@echo ""
	@echo "💡 Quick Start:"
	@echo "  make run      # Start everything"
	@echo "  make migrate  # Run database migrations"
	@echo "  make test     # Test the API"
	@echo "  make down     # Stop when done"
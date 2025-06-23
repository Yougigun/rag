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

# Run all tests using Docker k6
.PHONY: test
test:
	@printf '\033[0;34m> Running API tests...\033[0m\n'
	@if docker run --rm -i --network host \
		-v $(PWD)/tests:/tests \
		grafana/k6:latest run /tests/api-test.js --quiet | tee /tmp/api-test.log && \
		grep -q "checks.*100\.00%" /tmp/api-test.log; then \
		printf '\033[0;32m✅ API tests PASSED (100%% checks successful)\033[0m\n'; \
	else \
		printf '\033[0;31m❌ API tests FAILED (some checks failed)\033[0m\n'; \
		grep "checks.*:" /tmp/api-test.log || true; \
		exit 1; \
	fi
	@printf '\033[0;34m> Running Qdrant integration tests...\033[0m\n'
	@if docker run --rm -i --network host \
		-v $(PWD)/tests:/tests \
		grafana/k6:latest run /tests/qdrant-integration-test.js --quiet | tee /tmp/qdrant-test.log && \
		grep -q "checks.*100\.00%" /tmp/qdrant-test.log; then \
		printf '\033[0;32m✅ Qdrant integration tests PASSED (100%% checks successful)\033[0m\n'; \
	else \
		printf '\033[0;31m❌ Qdrant integration tests FAILED (some checks failed)\033[0m\n'; \
		grep "checks.*:" /tmp/qdrant-test.log || true; \
		exit 1; \
	fi
	@printf '\033[0;32m🎉 All tests PASSED successfully!\033[0m\n'

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

# Complete verification pipeline: build, deploy, test, cleanup
.PHONY: verify-all
verify-all:
	@printf '\033[0;33m🚀 Starting complete verification pipeline...\033[0m\n'
	@printf '\033[0;34m> Step 1: Cleaning previous environment...\033[0m\n'
	$(MAKE) clean
	@printf '\033[0;34m> Step 2: Building and deploying services...\033[0m\n'
	$(MAKE) run
	@printf '\033[0;34m> Step 3: Running API tests...\033[0m\n'
	$(MAKE) test
	@printf '\033[0;34m> Step 4: Cleaning up containers...\033[0m\n'
	$(MAKE) clean
	@printf '\033[0;32m✅ Complete verification pipeline completed successfully!\033[0m\n'

# Show help
.PHONY: help
help:
	@echo "🚀 RAG System - Simple Commands:"
	@echo ""
	@echo "  run          - Build and start all services (infrastructure + applications)"
	@echo "  migrate      - Run database migrations"
	@echo "  migrate-down - Rollback database migrations"
	@echo "  test         - Run all tests (API + Qdrant integration)"
	@echo "  verify-all   - Complete pipeline: clean → build → deploy → test → clean"
	@echo "  down         - Stop all services"
	@echo "  clean        - Stop services and remove volumes"
	@echo "  help         - Show this help"
	@echo ""
	@echo "💡 Quick Start:"
	@echo "  make verify-all  # Complete verification pipeline (recommended)"
.PHONY: help
help:
	@echo "Family Todo App - Development Commands"
	@echo ""
	@echo "Development Environment:"
	@echo "  make dev-up        - Start local development environment"
	@echo "  make dev-down      - Stop local development environment"
	@echo "  make dev-status    - Show status of development services"
	@echo ""
	@echo "Code Quality:"
	@echo "  make fmt           - Format code (Rust + TypeScript)"
	@echo "  make lint          - Run linters (Rust + TypeScript)"
	@echo "  make typecheck     - Run type checking"
	@echo ""
	@echo "Testing:"
	@echo "  make test          - Run all tests"
	@echo "  make test-unit     - Run unit tests"
	@echo "  make test-integration - Run integration tests"
	@echo "  make test-e2e      - Run end-to-end tests"
	@echo ""
	@echo "Building & Deployment:"
	@echo "  make build         - Build all Lambda functions"
	@echo "  make build-frontend - Build frontend"
	@echo "  make deploy-local  - Deploy to local environment"
	@echo ""
	@echo "Database Management:"
	@echo "  make db-setup      - Setup local database"
	@echo "  make db-reset      - Reset local database"
	@echo "  make db-seed       - Seed test data"

.PHONY: dev-up
dev-up:
	@echo "Starting local development environment..."
	docker-compose up -d
	@echo "Waiting for services to be ready..."
	@sleep 5
	./scripts/setup-local-db.sh
	@echo "Development environment is ready!"
	@echo ""
	@echo "Services running:"
	@echo "  - DynamoDB Local: http://localhost:8000"
	@echo "  - LocalStack: http://localhost:4566"
	@echo "  - Redis: http://localhost:6379"
	@echo ""
	@echo "To start development servers:"
	@echo "  make dev-servers"

.PHONY: dev-servers
dev-servers:
	@echo "Starting development servers..."
	cd frontend && npm run dev &
	cd backend && cargo watch -x check -x test -x "run --example local"

.PHONY: dev-down
dev-down:
	@echo "Stopping local development environment..."
	docker-compose down
	pkill -f "npm run dev" || true
	pkill -f "cargo watch" || true
	@echo "Development environment stopped."

.PHONY: dev-status
dev-status:
	@echo "Development Environment Status:"
	@echo ""
	@echo "Docker containers:"
	@docker ps -f name=todo- --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
	@echo ""
	@echo "Services health check:"
	@curl -s http://localhost:8000/ >/dev/null 2>&1 && echo "  ✓ DynamoDB Local" || echo "  ✗ DynamoDB Local"
	@curl -s http://localhost:4566/_localstack/health >/dev/null 2>&1 && echo "  ✓ LocalStack" || echo "  ✗ LocalStack"
	@redis-cli -h localhost -p 6379 ping >/dev/null 2>&1 && echo "  ✓ Redis" || echo "  ✗ Redis"

.PHONY: fmt
fmt:
	@echo "Formatting Rust code..."
	cd backend && cargo fmt --all
	@echo "Formatting TypeScript code..."
	cd frontend && npm run format
	@echo "Code formatting complete!"

.PHONY: lint
lint:
	@echo "Running Rust linter..."
	cd backend && cargo clippy --all-targets --all-features -- -D warnings
	@echo "Running TypeScript linter..."
	cd frontend && npm run lint
	@echo "Linting complete!"

.PHONY: typecheck
typecheck:
	@echo "Running Rust type checking..."
	cd backend && cargo check --all-targets --all-features
	@echo "Running TypeScript type checking..."
	cd frontend && npm run typecheck
	@echo "Type checking complete!"

.PHONY: test
test: test-unit test-integration

.PHONY: test-unit
test-unit:
	@echo "Running unit tests..."
	cd backend && cargo test --lib --all-features
	cd frontend && npm run test:unit
	@echo "Unit tests complete!"

.PHONY: test-integration
test-integration:
	@echo "Running integration tests..."
	cd backend && cargo test --test '*' --all-features
	@echo "Integration tests complete!"

.PHONY: test-e2e
test-e2e:
	@echo "Running end-to-end tests..."
	cd frontend && npm run test:e2e
	@echo "End-to-end tests complete!"

.PHONY: build
build:
	@echo "Building Lambda functions..."
	cd infra && sam build --use-container --parallel --cached
	@echo "Build complete!"

.PHONY: build-frontend
build-frontend:
	@echo "Building frontend..."
	cd frontend && npm run build
	@echo "Frontend build complete!"

.PHONY: deploy-local
deploy-local: build
	@echo "Starting local API..."
	cd infra && sam local start-api \
		--warm-containers EAGER \
		--port 3001 \
		--env-vars ../env.json

.PHONY: db-setup
db-setup:
	@echo "Setting up local database..."
	./scripts/setup-local-db.sh

.PHONY: db-reset
db-reset:
	@echo "Resetting local database..."
	aws dynamodb delete-table \
		--endpoint-url http://localhost:8000 \
		--region ap-northeast-1 \
		--table-name MainTable || true
	@sleep 2
	./scripts/setup-local-db.sh

.PHONY: db-seed
db-seed:
	@echo "Seeding test data..."
	@echo "Test data seeding is not implemented yet"
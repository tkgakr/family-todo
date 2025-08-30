.PHONY: help
help:
	@echo "Available commands:"
	@echo "  make dev-up        - Start local development environment"
	@echo "  make dev-down      - Stop local development environment"
	@echo "  make test          - Run all tests"
	@echo "  make test-unit     - Run unit tests"
	@echo "  make test-integration - Run integration tests"
	@echo "  make fmt           - Format code"
	@echo "  make lint          - Run linters"
	@echo "  make build         - Build all Lambda functions"
	@echo "  make deploy-local  - Deploy to local environment"

.PHONY: dev-up
dev-up:
	docker-compose up -d
	./scripts/setup-local-db.sh
	cd frontend && npm run dev &
	cd backend && cargo watch -x test -x run

.PHONY: dev-down
dev-down:
	docker-compose down
	pkill -f "npm run dev" || true
	pkill -f "cargo watch" || true

.PHONY: test
test: test-unit test-integration

.PHONY: test-unit
test-unit:
	cd backend && cargo test --lib
	cd frontend && npm test

.PHONY: test-integration
test-integration:
	cd backend && cargo test --test '*'
	cd frontend && npm run test:e2e

.PHONY: fmt
fmt:
	cd backend && cargo fmt
	cd frontend && npm run format

.PHONY: lint
lint:
	cd backend && cargo clippy -- -D warnings
	cd frontend && npm run lint

.PHONY: build
build:
	sam build --use-container --parallel

.PHONY: deploy-local
deploy-local: build
	sam local start-api \
		--warm-containers EAGER \
		--port 3001 \
		--env-vars env.json
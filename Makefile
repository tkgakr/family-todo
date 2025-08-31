.PHONY: help build test clean deploy-dev deploy-prod frontend-dev frontend-build

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build all Rust crates
	cargo build --release

test: ## Run all tests
	cargo test --workspace
	cd frontend && npm test

lint: ## Run linting for Rust and frontend
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cd frontend && npm run lint

format: ## Format code
	cargo fmt --all
	cd frontend && npm run format

clean: ## Clean build artifacts
	cargo clean
	cd frontend && rm -rf dist node_modules

install-deps: ## Install all dependencies
	cargo build
	cd frontend && npm install

lambda-build: ## Build Lambda functions
	cargo lambda build --release --output-format zip
	mkdir -p target/lambda/family-invitation
	cp target/lambda/family-invitation/bootstrap target/lambda/family-invitation/

sam-local: ## Start SAM local API
	sam local start-api --port 8080 --env-vars local-env.json

docker-up: ## Start local development services (DynamoDB Local)
	docker-compose up -d

docker-down: ## Stop local development services
	docker-compose down

docker-logs: ## View logs from local services
	docker-compose logs -f

setup-local-table: ## Create local DynamoDB table
	aws dynamodb create-table \
		--endpoint-url http://localhost:8000 \
		--table-name family-todo-local \
		--attribute-definitions \
			AttributeName=PK,AttributeType=S \
			AttributeName=SK,AttributeType=S \
			AttributeName=GSI1PK,AttributeType=S \
			AttributeName=GSI1SK,AttributeType=S \
		--key-schema \
			AttributeName=PK,KeyType=HASH \
			AttributeName=SK,KeyType=RANGE \
		--global-secondary-indexes \
			IndexName=GSI1,KeySchema=[{AttributeName=GSI1PK,KeyType=HASH},{AttributeName=GSI1SK,KeyType=RANGE}],Projection={ProjectionType=ALL},ProvisionedThroughput={ReadCapacityUnits=5,WriteCapacityUnits=5} \
		--provisioned-throughput ReadCapacityUnits=5,WriteCapacityUnits=5 \
		--stream-specification StreamEnabled=true,StreamViewType=NEW_AND_OLD_IMAGES

local-dev: docker-up setup-local-table ## Start complete local development environment
	@echo "Local development environment started!"
	@echo "DynamoDB Local: http://localhost:8000"
	@echo "DynamoDB Admin: http://localhost:8001"
	@echo "Run 'make sam-local' to start the API server"

sam-build: ## Build SAM application
	sam build

deploy-dev: lambda-build ## Deploy to development environment
	sam deploy --config-env default

deploy-prod: lambda-build ## Deploy to production environment
	sam deploy --config-env production

frontend-dev: ## Start frontend development server
	cd frontend && npm run dev

frontend-build: ## Build frontend for production
	cd frontend && npm run build

setup: ## Complete development environment setup
	@chmod +x scripts/setup-dev.sh
	@scripts/setup-dev.sh

setup-quick: install-deps ## Quick setup without Docker services
	@echo "Quick setup complete!"
	@echo "Run 'make local-dev' to start local services"
	@echo "Run 'make frontend-dev' to start the frontend development server"
	@echo "Run 'make sam-local' to start the local API server"

validate-template: ## Validate SAM template
	sam validate --template template.yaml

logs-dev: ## Tail logs for development environment
	sam logs --stack-name family-todo-dev --tail

logs-prod: ## Tail logs for production environment
	sam logs --stack-name family-todo-prod --tail

health-check-dev: ## Run health checks for development environment
	@chmod +x scripts/health-check.sh
	@scripts/health-check.sh dev

health-check-prod: ## Run health checks for production environment
	@chmod +x scripts/health-check.sh
	@scripts/health-check.sh prod
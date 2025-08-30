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

sam-local: ## Start SAM local API
	sam local start-api --port 8080

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

setup: install-deps ## Initial project setup
	@echo "Project setup complete!"
	@echo "Run 'make frontend-dev' to start the frontend development server"
	@echo "Run 'make sam-local' to start the local API server"

validate-template: ## Validate SAM template
	sam validate --template template.yaml

logs-dev: ## Tail logs for development environment
	sam logs --stack-name family-todo-dev --tail

logs-prod: ## Tail logs for production environment
	sam logs --stack-name family-todo-prod --tail
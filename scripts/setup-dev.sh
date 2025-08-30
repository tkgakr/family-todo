#!/bin/bash

# Development Environment Setup Script
# This script sets up the complete development environment for the Family Todo App

set -e

echo "🚀 Setting up Family Todo App development environment..."

# Check prerequisites
echo "📋 Checking prerequisites..."

# Check Rust
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust is not installed. Please install from https://rustup.rs/"
    exit 1
fi
echo "✅ Rust found: $(rustc --version)"

# Check Node.js
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed. Please install Node.js 20+ from https://nodejs.org/"
    exit 1
fi
NODE_VERSION=$(node --version | cut -d'v' -f2 | cut -d'.' -f1)
if [ "$NODE_VERSION" -lt 20 ]; then
    echo "❌ Node.js version must be 20 or higher. Current: $(node --version)"
    exit 1
fi
echo "✅ Node.js found: $(node --version)"

# Check AWS CLI
if ! command -v aws &> /dev/null; then
    echo "❌ AWS CLI is not installed. Please install from https://aws.amazon.com/cli/"
    exit 1
fi
echo "✅ AWS CLI found: $(aws --version)"

# Check SAM CLI
if ! command -v sam &> /dev/null; then
    echo "❌ SAM CLI is not installed. Please install from https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/install-sam-cli.html"
    exit 1
fi
echo "✅ SAM CLI found: $(sam --version)"

# Check Docker
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install from https://www.docker.com/"
    exit 1
fi
echo "✅ Docker found: $(docker --version)"

# Install cargo-lambda
echo "📦 Installing cargo-lambda..."
if ! command -v cargo-lambda &> /dev/null; then
    cargo install cargo-lambda
    echo "✅ cargo-lambda installed"
else
    echo "✅ cargo-lambda already installed: $(cargo-lambda --version)"
fi

# Install Rust dependencies
echo "📦 Installing Rust dependencies..."
cargo build
echo "✅ Rust dependencies installed"

# Install frontend dependencies
echo "📦 Installing frontend dependencies..."
cd frontend
npm install
cd ..
echo "✅ Frontend dependencies installed"

# Create environment file
echo "⚙️ Setting up environment configuration..."
if [ ! -f .env.local ]; then
    cp .env.example .env.local
    echo "✅ Created .env.local from template"
    echo "📝 Please edit .env.local with your AWS configuration"
else
    echo "✅ .env.local already exists"
fi

# Validate SAM template
echo "🔍 Validating SAM template..."
sam validate --template template.yaml
echo "✅ SAM template is valid"

# Start Docker services
echo "🐳 Starting local development services..."
docker-compose up -d
echo "✅ Docker services started"

# Wait for DynamoDB Local to be ready
echo "⏳ Waiting for DynamoDB Local to be ready..."
sleep 5

# Create local DynamoDB table
echo "🗄️ Creating local DynamoDB table..."
make setup-local-table || echo "⚠️ Table might already exist"
echo "✅ Local DynamoDB table ready"

echo ""
echo "🎉 Development environment setup complete!"
echo ""
echo "📍 Next steps:"
echo "1. Edit .env.local with your AWS configuration"
echo "2. Run 'make sam-local' to start the API server"
echo "3. Run 'make frontend-dev' to start the frontend"
echo ""
echo "🔗 Development URLs:"
echo "- Frontend: http://localhost:3000"
echo "- API Gateway: http://localhost:8080"
echo "- DynamoDB Local: http://localhost:8000"
echo "- DynamoDB Admin: http://localhost:8001"
echo ""
echo "📚 Run 'make help' to see all available commands"
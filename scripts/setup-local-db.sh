#!/bin/bash

# Setup script for local DynamoDB development environment

set -e

echo "Setting up local DynamoDB..."

# Wait for DynamoDB Local to be ready
echo "Waiting for DynamoDB Local to start..."
until aws dynamodb list-tables --endpoint-url http://localhost:8000 --region ap-northeast-1 > /dev/null 2>&1; do
  sleep 1
done

echo "DynamoDB Local is ready!"

# Create the main table
echo "Creating MainTable..."
aws dynamodb create-table \
  --endpoint-url http://localhost:8000 \
  --region ap-northeast-1 \
  --table-name MainTable \
  --attribute-definitions \
    AttributeName=PK,AttributeType=S \
    AttributeName=SK,AttributeType=S \
    AttributeName=GSI1PK,AttributeType=S \
    AttributeName=GSI1SK,AttributeType=S \
  --key-schema \
    AttributeName=PK,KeyType=HASH \
    AttributeName=SK,KeyType=RANGE \
  --global-secondary-indexes \
    'IndexName=GSI1,KeySchema=[{AttributeName=GSI1PK,KeyType=HASH},{AttributeName=GSI1SK,KeyType=RANGE}],Projection={ProjectionType=ALL},BillingMode=PAY_PER_REQUEST' \
  --billing-mode PAY_PER_REQUEST \
  --stream-specification StreamEnabled=true,StreamViewType=NEW_AND_OLD_IMAGES

echo "Waiting for table to be active..."
aws dynamodb wait table-exists \
  --endpoint-url http://localhost:8000 \
  --region ap-northeast-1 \
  --table-name MainTable

echo "Local DynamoDB setup complete!"
echo ""
echo "Table created:"
echo "- MainTable (with GSI1 and DynamoDB Streams enabled)"
echo ""
echo "You can now start the Lambda functions locally with:"
echo "  sam local start-api --env-vars env.json"
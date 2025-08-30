#!/bin/bash

# Health Check Script for Family Todo App
# This script performs basic health checks on the deployed application

set -e

ENVIRONMENT=${1:-dev}
STACK_NAME="family-todo-${ENVIRONMENT}"

echo "🏥 Running health checks for ${ENVIRONMENT} environment..."

# Get API Gateway URL from CloudFormation outputs
echo "📡 Getting API Gateway URL..."
API_URL=$(aws cloudformation describe-stacks \
    --stack-name "$STACK_NAME" \
    --query 'Stacks[0].Outputs[?OutputKey==`ApiGatewayUrl`].OutputValue' \
    --output text 2>/dev/null || echo "")

if [ -z "$API_URL" ]; then
    echo "❌ Could not retrieve API Gateway URL. Stack might not be deployed."
    exit 1
fi

echo "✅ API Gateway URL: $API_URL"

# Test API Gateway health (if health endpoint exists)
echo "🔍 Testing API Gateway connectivity..."
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/health" || echo "000")

if [ "$HTTP_STATUS" = "200" ]; then
    echo "✅ API Gateway health check passed"
elif [ "$HTTP_STATUS" = "404" ]; then
    echo "⚠️ Health endpoint not implemented (404), but API Gateway is responding"
else
    echo "❌ API Gateway health check failed (HTTP $HTTP_STATUS)"
fi

# Check DynamoDB table
echo "🗄️ Checking DynamoDB table..."
TABLE_NAME=$(aws cloudformation describe-stacks \
    --stack-name "$STACK_NAME" \
    --query 'Stacks[0].Outputs[?OutputKey==`DynamoDBTableName`].OutputValue' \
    --output text 2>/dev/null || echo "")

if [ -n "$TABLE_NAME" ]; then
    TABLE_STATUS=$(aws dynamodb describe-table \
        --table-name "$TABLE_NAME" \
        --query 'Table.TableStatus' \
        --output text 2>/dev/null || echo "NOT_FOUND")
    
    if [ "$TABLE_STATUS" = "ACTIVE" ]; then
        echo "✅ DynamoDB table is active: $TABLE_NAME"
    else
        echo "❌ DynamoDB table status: $TABLE_STATUS"
    fi
else
    echo "❌ Could not retrieve DynamoDB table name"
fi

# Check Lambda functions
echo "🔧 Checking Lambda functions..."
FUNCTIONS=("command-handler" "query-handler" "event-processor" "snapshot-manager")

for FUNC in "${FUNCTIONS[@]}"; do
    FUNC_NAME="family-todo-${FUNC}-${ENVIRONMENT}"
    FUNC_STATUS=$(aws lambda get-function \
        --function-name "$FUNC_NAME" \
        --query 'Configuration.State' \
        --output text 2>/dev/null || echo "NOT_FOUND")
    
    if [ "$FUNC_STATUS" = "Active" ]; then
        echo "✅ Lambda function active: $FUNC_NAME"
    else
        echo "❌ Lambda function status: $FUNC_NAME ($FUNC_STATUS)"
    fi
done

# Check Cognito User Pool
echo "👤 Checking Cognito User Pool..."
USER_POOL_ID=$(aws cloudformation describe-stacks \
    --stack-name "$STACK_NAME" \
    --query 'Stacks[0].Outputs[?OutputKey==`UserPoolId`].OutputValue' \
    --output text 2>/dev/null || echo "")

if [ -n "$USER_POOL_ID" ]; then
    USER_POOL_STATUS=$(aws cognito-idp describe-user-pool \
        --user-pool-id "$USER_POOL_ID" \
        --query 'UserPool.Status' \
        --output text 2>/dev/null || echo "NOT_FOUND")
    
    if [ "$USER_POOL_STATUS" = "Enabled" ]; then
        echo "✅ Cognito User Pool is enabled: $USER_POOL_ID"
    else
        echo "❌ Cognito User Pool status: $USER_POOL_STATUS"
    fi
else
    echo "❌ Could not retrieve Cognito User Pool ID"
fi

# Check CloudWatch Log Groups
echo "📊 Checking CloudWatch Log Groups..."
for FUNC in "${FUNCTIONS[@]}"; do
    LOG_GROUP="/aws/lambda/family-todo-${FUNC}-${ENVIRONMENT}"
    LOG_EXISTS=$(aws logs describe-log-groups \
        --log-group-name-prefix "$LOG_GROUP" \
        --query 'logGroups[0].logGroupName' \
        --output text 2>/dev/null || echo "None")
    
    if [ "$LOG_EXISTS" != "None" ]; then
        echo "✅ Log group exists: $LOG_GROUP"
    else
        echo "⚠️ Log group not found: $LOG_GROUP"
    fi
done

echo ""
echo "🎯 Health check summary for ${ENVIRONMENT} environment:"
echo "- Stack: $STACK_NAME"
echo "- API URL: $API_URL"
echo "- DynamoDB Table: $TABLE_NAME"
echo "- User Pool: $USER_POOL_ID"
echo ""
echo "✅ Health check completed!"
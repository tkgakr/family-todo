import { beforeAll, afterAll } from 'vitest'
import { DynamoDBClient } from '@aws-sdk/client-dynamodb'
import { DynamoDBDocumentClient } from '@aws-sdk/lib-dynamodb'

// Global test configuration
const config = {
  dynamodbEndpoint: process.env.DYNAMODB_ENDPOINT || 'http://localhost:8000',
  tableName: process.env.TABLE_NAME || 'MainTable',
  region: process.env.AWS_DEFAULT_REGION || 'ap-northeast-1',
}

// Setup DynamoDB client for tests
const client = new DynamoDBClient({
  endpoint: config.dynamodbEndpoint,
  region: config.region,
  credentials: {
    accessKeyId: 'test',
    secretAccessKey: 'test',
  },
})

export const docClient = DynamoDBDocumentClient.from(client)

// Global test setup
beforeAll(async () => {
  console.log('Setting up test environment...')
  
  // Verify DynamoDB connection
  try {
    const { TableNames } = await client.send(new ListTablesCommand({}))
    if (!TableNames.includes(config.tableName)) {
      throw new Error(`Table ${config.tableName} does not exist`)
    }
    console.log(`✓ Connected to DynamoDB, table ${config.tableName} exists`)
  } catch (error) {
    console.error('Failed to connect to DynamoDB:', error.message)
    throw error
  }
})

// Global test cleanup
afterAll(async () => {
  console.log('Cleaning up test environment...')
  // Add cleanup logic if needed
})

export { config }
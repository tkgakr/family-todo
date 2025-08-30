import { describe, it, expect, beforeEach } from 'vitest'
import { docClient, config } from '../setup.js'
import { PutCommand, QueryCommand, GetCommand, DeleteCommand } from '@aws-sdk/lib-dynamodb'

describe('Todo Lifecycle Integration Tests', () => {
  const familyId = 'test-family-001'
  const userId = 'test-user-001'
  let todoId

  beforeEach(async () => {
    // Generate a unique todo ID for each test
    todoId = `01H${Date.now().toString(36).toUpperCase()}TEST`
  })

  it('should create a new todo with event', async () => {
    // Arrange
    const todoEvent = {
      event_type: 'todo_created_v2',
      event_id: `01H${Date.now().toString(36).toUpperCase()}EVT`,
      todo_id: todoId,
      title: '統合テスト用ToDo',
      description: 'これは統合テストで作成されたToDoです',
      tags: ['テスト', 'CI'],
      created_by: userId,
      timestamp: new Date().toISOString(),
    }

    const eventItem = {
      PK: `FAMILY#${familyId}`,
      SK: `EVENT#${todoEvent.event_id}`,
      EntityType: 'Event',
      Data: JSON.stringify(todoEvent),
      CreatedAt: new Date().toISOString(),
      TTL: Math.floor(Date.now() / 1000) + (86400 * 365), // 1 year TTL
    }

    // Act - Save the event
    await docClient.send(new PutCommand({
      TableName: config.tableName,
      Item: eventItem,
    }))

    // Create projection (simulating what EventProcessor would do)
    const todoProjection = {
      PK: `FAMILY#${familyId}`,
      SK: `TODO#CURRENT#${todoId}`,
      EntityType: 'Projection',
      Data: JSON.stringify({
        id: todoId,
        title: todoEvent.title,
        description: todoEvent.description,
        tags: todoEvent.tags,
        status: 'Active',
        created_by: todoEvent.created_by,
        created_at: todoEvent.timestamp,
        version: 1,
      }),
      GSI1PK: `FAMILY#${familyId}#ACTIVE`,
      GSI1SK: todoId,
      Version: 1,
      CreatedAt: new Date().toISOString(),
      UpdatedAt: new Date().toISOString(),
    }

    await docClient.send(new PutCommand({
      TableName: config.tableName,
      Item: todoProjection,
    }))

    // Assert - Verify event was saved
    const eventResult = await docClient.send(new GetCommand({
      TableName: config.tableName,
      Key: {
        PK: `FAMILY#${familyId}`,
        SK: `EVENT#${todoEvent.event_id}`,
      },
    }))

    expect(eventResult.Item).toBeDefined()
    expect(eventResult.Item.EntityType).toBe('Event')

    const savedEvent = JSON.parse(eventResult.Item.Data)
    expect(savedEvent.title).toBe('統合テスト用ToDo')
    expect(savedEvent.todo_id).toBe(todoId)

    // Assert - Verify projection was created
    const projectionResult = await docClient.send(new GetCommand({
      TableName: config.tableName,
      Key: {
        PK: `FAMILY#${familyId}`,
        SK: `TODO#CURRENT#${todoId}`,
      },
    }))

    expect(projectionResult.Item).toBeDefined()
    expect(projectionResult.Item.EntityType).toBe('Projection')

    const savedTodo = JSON.parse(projectionResult.Item.Data)
    expect(savedTodo.title).toBe('統合テスト用ToDo')
    expect(savedTodo.status).toBe('Active')
  })

  it('should update todo and create update event', async () => {
    // Arrange - First create a todo
    const createEvent = {
      event_type: 'todo_created_v2',
      event_id: `01H${Date.now().toString(36).toUpperCase()}CR8`,
      todo_id: todoId,
      title: '元のタイトル',
      description: null,
      tags: [],
      created_by: userId,
      timestamp: new Date().toISOString(),
    }

    // Save creation event and projection
    await docClient.send(new PutCommand({
      TableName: config.tableName,
      Item: {
        PK: `FAMILY#${familyId}`,
        SK: `EVENT#${createEvent.event_id}`,
        EntityType: 'Event',
        Data: JSON.stringify(createEvent),
        CreatedAt: new Date().toISOString(),
      },
    }))

    await docClient.send(new PutCommand({
      TableName: config.tableName,
      Item: {
        PK: `FAMILY#${familyId}`,
        SK: `TODO#CURRENT#${todoId}`,
        EntityType: 'Projection',
        Data: JSON.stringify({
          id: todoId,
          title: createEvent.title,
          status: 'Active',
          created_by: createEvent.created_by,
          created_at: createEvent.timestamp,
          version: 1,
        }),
        Version: 1,
        CreatedAt: new Date().toISOString(),
      },
    }))

    // Act - Create update event
    const updateEvent = {
      event_type: 'todo_updated_v1',
      event_id: `01H${Date.now().toString(36).toUpperCase()}UPD`,
      todo_id: todoId,
      title: '更新されたタイトル',
      description: '説明が追加されました',
      updated_by: userId,
      timestamp: new Date().toISOString(),
    }

    await docClient.send(new PutCommand({
      TableName: config.tableName,
      Item: {
        PK: `FAMILY#${familyId}`,
        SK: `EVENT#${updateEvent.event_id}`,
        EntityType: 'Event',
        Data: JSON.stringify(updateEvent),
        CreatedAt: new Date().toISOString(),
      },
    }))

    // Update projection
    await docClient.send(new PutCommand({
      TableName: config.tableName,
      Item: {
        PK: `FAMILY#${familyId}`,
        SK: `TODO#CURRENT#${todoId}`,
        EntityType: 'Projection',
        Data: JSON.stringify({
          id: todoId,
          title: updateEvent.title,
          description: updateEvent.description,
          status: 'Active',
          created_by: createEvent.created_by,
          created_at: createEvent.timestamp,
          updated_at: updateEvent.timestamp,
          version: 2,
        }),
        Version: 2,
        UpdatedAt: updateEvent.timestamp,
      },
    }))

    // Assert
    const result = await docClient.send(new GetCommand({
      TableName: config.tableName,
      Key: {
        PK: `FAMILY#${familyId}`,
        SK: `TODO#CURRENT#${todoId}`,
      },
    }))

    const updatedTodo = JSON.parse(result.Item.Data)
    expect(updatedTodo.title).toBe('更新されたタイトル')
    expect(updatedTodo.description).toBe('説明が追加されました')
    expect(updatedTodo.version).toBe(2)
  })

  it('should complete todo and update status', async () => {
    // Arrange - Create a todo first
    const createEvent = {
      event_type: 'todo_created_v2',
      event_id: `01H${Date.now().toString(36).toUpperCase()}CR8`,
      todo_id: todoId,
      title: '完了テスト用ToDo',
      created_by: userId,
      timestamp: new Date().toISOString(),
    }

    await docClient.send(new PutCommand({
      TableName: config.tableName,
      Item: {
        PK: `FAMILY#${familyId}`,
        SK: `TODO#CURRENT#${todoId}`,
        EntityType: 'Projection',
        Data: JSON.stringify({
          id: todoId,
          title: createEvent.title,
          status: 'Active',
          created_by: createEvent.created_by,
          created_at: createEvent.timestamp,
          version: 1,
        }),
        GSI1PK: `FAMILY#${familyId}#ACTIVE`,
        GSI1SK: todoId,
        Version: 1,
      },
    }))

    // Act - Complete the todo
    const completeEvent = {
      event_type: 'todo_completed_v1',
      event_id: `01H${Date.now().toString(36).toUpperCase()}CMP`,
      todo_id: todoId,
      completed_by: userId,
      timestamp: new Date().toISOString(),
    }

    await docClient.send(new PutCommand({
      TableName: config.tableName,
      Item: {
        PK: `FAMILY#${familyId}`,
        SK: `EVENT#${completeEvent.event_id}`,
        EntityType: 'Event',
        Data: JSON.stringify(completeEvent),
        CreatedAt: new Date().toISOString(),
      },
    }))

    // Update projection - remove from GSI1 (active todos)
    await docClient.send(new PutCommand({
      TableName: config.tableName,
      Item: {
        PK: `FAMILY#${familyId}`,
        SK: `TODO#CURRENT#${todoId}`,
        EntityType: 'Projection',
        Data: JSON.stringify({
          id: todoId,
          title: createEvent.title,
          status: 'Completed',
          created_by: createEvent.created_by,
          created_at: createEvent.timestamp,
          completed_at: completeEvent.timestamp,
          version: 2,
        }),
        Version: 2,
      },
    }))

    // Assert
    const result = await docClient.send(new GetCommand({
      TableName: config.tableName,
      Key: {
        PK: `FAMILY#${familyId}`,
        SK: `TODO#CURRENT#${todoId}`,
      },
    }))

    const completedTodo = JSON.parse(result.Item.Data)
    expect(completedTodo.status).toBe('Completed')
    expect(completedTodo.completed_at).toBeDefined()
    expect(result.Item.GSI1PK).toBeUndefined() // Should not be in active index
  })

  it('should retrieve todo event history', async () => {
    // Arrange - Create multiple events for the same todo
    const events = [
      {
        event_type: 'todo_created_v2',
        event_id: `01H${Date.now().toString(36).toUpperCase()}001`,
        todo_id: todoId,
        title: 'テストToDo',
        created_by: userId,
        timestamp: new Date().toISOString(),
      },
      {
        event_type: 'todo_updated_v1',
        event_id: `01H${Date.now().toString(36).toUpperCase()}002`,
        todo_id: todoId,
        title: '更新されたToDo',
        updated_by: userId,
        timestamp: new Date(Date.now() + 1000).toISOString(),
      },
      {
        event_type: 'todo_completed_v1',
        event_id: `01H${Date.now().toString(36).toUpperCase()}003`,
        todo_id: todoId,
        completed_by: userId,
        timestamp: new Date(Date.now() + 2000).toISOString(),
      },
    ]

    // Save all events
    for (const event of events) {
      await docClient.send(new PutCommand({
        TableName: config.tableName,
        Item: {
          PK: `FAMILY#${familyId}`,
          SK: `TODO#EVENT#${todoId}#${event.event_id}`,
          EntityType: 'Event',
          Data: JSON.stringify(event),
          CreatedAt: new Date().toISOString(),
        },
      }))
    }

    // Act - Retrieve event history
    const result = await docClient.send(new QueryCommand({
      TableName: config.tableName,
      KeyConditionExpression: 'PK = :pk AND begins_with(SK, :sk_prefix)',
      ExpressionAttributeValues: {
        ':pk': `FAMILY#${familyId}`,
        ':sk_prefix': `TODO#EVENT#${todoId}#`,
      },
    }))

    // Assert
    expect(result.Items).toHaveLength(3)
    
    const eventHistory = result.Items
      .map(item => JSON.parse(item.Data))
      .sort((a, b) => new Date(a.timestamp) - new Date(b.timestamp))

    expect(eventHistory[0].event_type).toBe('todo_created_v2')
    expect(eventHistory[1].event_type).toBe('todo_updated_v1')
    expect(eventHistory[2].event_type).toBe('todo_completed_v1')
  })

  // Cleanup after each test
  afterEach(async () => {
    try {
      // Clean up test data
      const itemsToDelete = [
        { PK: `FAMILY#${familyId}`, SK: `TODO#CURRENT#${todoId}` },
      ]

      // Query for events to delete
      const eventQuery = await docClient.send(new QueryCommand({
        TableName: config.tableName,
        KeyConditionExpression: 'PK = :pk AND begins_with(SK, :sk_prefix)',
        ExpressionAttributeValues: {
          ':pk': `FAMILY#${familyId}`,
          ':sk_prefix': 'EVENT#',
        },
      }))

      if (eventQuery.Items) {
        for (const item of eventQuery.Items) {
          itemsToDelete.push({ PK: item.PK, SK: item.SK })
        }
      }

      // Delete all test items
      for (const key of itemsToDelete) {
        await docClient.send(new DeleteCommand({
          TableName: config.tableName,
          Key: key,
        }))
      }
    } catch (error) {
      console.warn('Cleanup failed:', error.message)
    }
  })
})
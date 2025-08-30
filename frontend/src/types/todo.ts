export interface Todo {
  id: string
  title: string
  description?: string
  tags: string[]
  status: TodoStatus
  created_by: string
  created_at: string
  updated_at?: string
  completed_at?: string
  version: number
}

export enum TodoStatus {
  Active = 'Active',
  Completed = 'Completed',
  Deleted = 'Deleted',
}

export interface CreateTodoRequest {
  title: string
  description?: string
  tags?: string[]
}

export interface UpdateTodoRequest {
  title?: string
  description?: string
  tags?: string[]
}

export interface TodoListResponse {
  todos: Todo[]
  total_count?: number
  has_more: boolean
}

export interface TodoEvent {
  event_id: string
  todo_id: string
  event_type: string
  timestamp: string
  [key: string]: unknown
}

export interface TodoHistoryResponse {
  todo_id: string
  events: TodoEvent[]
  total_count: number
}
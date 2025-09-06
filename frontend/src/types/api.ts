// API エンドポイント関連の型定義
import type { PaginatedResponse, Todo, TodoEvent } from './index'

// コマンド API レスポンス
export interface CreateTodoResponse {
  todoId: string
  message: string
}

export interface UpdateTodoResponse {
  message: string
}

export interface CompleteTodoResponse {
  message: string
}

export interface DeleteTodoResponse {
  message: string
}

// クエリ API レスポンス
export interface GetTodosResponse extends PaginatedResponse<Todo> {
  totalCount: number
}

export interface GetTodoResponse {
  todo: Todo
}

export interface GetTodoHistoryResponse extends PaginatedResponse<TodoEvent> {
  todoId: string
}

// API エラーレスポンス
export interface ApiErrorResponse {
  error: string
  message: string
  code?: string
  details?: Record<string, unknown>
}

// API リクエストオプション
export interface ApiRequestOptions {
  timeout?: number
  retries?: number
  headers?: Record<string, string>
}

// フィルタリング・ソート用の型
export interface TodoFilters {
  status?: 'active' | 'completed' | 'all'
  tags?: string[]
  createdBy?: string
  dateRange?: {
    start: string
    end: string
  }
}

export interface TodoSortOptions {
  field: 'createdAt' | 'updatedAt' | 'title'
  direction: 'asc' | 'desc'
}

export interface GetTodosParams {
  filters?: TodoFilters
  sort?: TodoSortOptions
  limit?: number
  nextToken?: string
}

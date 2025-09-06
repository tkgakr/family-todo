// 共通型定義
export interface Todo {
  id: string
  title: string
  description?: string
  tags: string[]
  completed: boolean
  createdBy: string
  createdAt: string
  updatedAt: string
  version: number
}

export interface CreateTodoRequest {
  title: string
  description?: string
  tags: string[]
}

export interface UpdateTodoRequest {
  title?: string
  description?: string
  tags?: string[]
}

export interface TodoEvent {
  eventId: string
  todoId: string
  eventType: 'todo_created_v2' | 'todo_updated_v1' | 'todo_completed_v1' | 'todo_deleted_v1'
  timestamp: string
  userId: string
  data: Record<string, unknown>
}

export interface User {
  id: string
  email: string
  familyId: string
  role: 'admin' | 'member'
}

export interface Family {
  id: string
  name: string
  members: User[]
  createdAt: string
}

// API レスポンス型
export interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
  message?: string
}

export interface PaginatedResponse<T> {
  items: T[]
  nextToken?: string
  hasMore: boolean
}

// 認証関連型
export interface AuthState {
  isAuthenticated: boolean
  user?: User
  loading: boolean
  error?: string
}

export interface LoginCredentials {
  email: string
}

// エラー型
export interface AppError {
  code: string
  message: string
  details?: Record<string, unknown>
}

// フォーム状態型
export interface FormState<T> {
  data: T
  errors: Record<keyof T, string>
  isSubmitting: boolean
  isDirty: boolean
}

// 環境変数型
export interface EnvironmentConfig {
  apiUrl: string
  cognitoUserPoolId: string
  cognitoClientId: string
  region: string
}

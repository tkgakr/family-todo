// 環境変数の型安全な管理
import type { EnvironmentConfig } from '../types'

// 環境変数の検証とデフォルト値設定
function getEnvironmentVariable(key: string, defaultValue?: string): string {
  const value = import.meta.env[key] || defaultValue
  if (!value) {
    // テスト環境では警告のみ出力
    if (import.meta.env.MODE === 'test') {
      console.warn(`環境変数 ${key} が設定されていません。デフォルト値を使用します。`)
      return defaultValue || ''
    }
    throw new Error(`環境変数 ${key} が設定されていません`)
  }
  return value
}

// 環境設定の作成
export const environment: EnvironmentConfig = {
  apiUrl: getEnvironmentVariable('VITE_API_URL', 'http://localhost:8080'),
  cognitoUserPoolId: getEnvironmentVariable('VITE_COGNITO_USER_POOL_ID', ''),
  cognitoClientId: getEnvironmentVariable('VITE_COGNITO_CLIENT_ID', ''),
  region: getEnvironmentVariable('VITE_AWS_REGION', 'ap-northeast-1'),
}

// 開発環境かどうかの判定
export const isDevelopment = import.meta.env.DEV
export const isProduction = import.meta.env.PROD

// API エンドポイントの構築
export const apiEndpoints = {
  commands: {
    createTodo: `${environment.apiUrl}/commands/todos`,
    updateTodo: (todoId: string) => `${environment.apiUrl}/commands/todos/${todoId}`,
    completeTodo: (todoId: string) => `${environment.apiUrl}/commands/todos/${todoId}/complete`,
    deleteTodo: (todoId: string) => `${environment.apiUrl}/commands/todos/${todoId}`,
  },
  queries: {
    getTodos: `${environment.apiUrl}/queries/todos`,
    getTodo: (todoId: string) => `${environment.apiUrl}/queries/todos/${todoId}`,
    getTodoHistory: (todoId: string) => `${environment.apiUrl}/queries/todos/${todoId}/history`,
  },
} as const

// ログレベルの設定
export const logLevel = getEnvironmentVariable('VITE_LOG_LEVEL', 'info')

// 機能フラグ
export const features = {
  enablePasskey: getEnvironmentVariable('VITE_ENABLE_PASSKEY', 'true') === 'true',
  enableRealTimeUpdates: getEnvironmentVariable('VITE_ENABLE_REALTIME', 'false') === 'true',
  enableAnalytics: getEnvironmentVariable('VITE_ENABLE_ANALYTICS', 'false') === 'true',
} as const

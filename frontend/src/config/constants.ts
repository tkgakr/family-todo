// アプリケーション定数
export const APP_CONFIG = {
  name: '家族用 ToDo アプリ',
  version: '1.0.0',
  description: 'イベントソーシングベースの家族用タスク管理アプリ',
} as const

// UI 関連定数
export const UI_CONFIG = {
  maxTitleLength: 200,
  maxDescriptionLength: 1000,
  maxTagsCount: 10,
  maxTagLength: 50,
  defaultPageSize: 20,
  maxPageSize: 100,
} as const

// API 関連定数
export const API_CONFIG = {
  timeout: 30000, // 30秒
  retryAttempts: 3,
  retryDelay: 1000, // 1秒
  maxRetryDelay: 5000, // 5秒
} as const

// 認証関連定数
export const AUTH_CONFIG = {
  tokenRefreshThreshold: 300000, // 5分前にリフレッシュ
  sessionTimeout: 3600000, // 1時間
  passkeyTimeout: 60000, // 1分
} as const

// バリデーション関連定数
export const VALIDATION = {
  todo: {
    title: {
      minLength: 1,
      maxLength: UI_CONFIG.maxTitleLength,
      required: true,
    },
    description: {
      maxLength: UI_CONFIG.maxDescriptionLength,
      required: false,
    },
    tags: {
      maxCount: UI_CONFIG.maxTagsCount,
      maxLength: UI_CONFIG.maxTagLength,
    },
  },
  user: {
    email: {
      pattern: /^[^\s@]+@[^\s@]+\.[^\s@]+$/,
      required: true,
    },
  },
} as const

// エラーメッセージ
export const ERROR_MESSAGES = {
  network: 'ネットワークエラーが発生しました',
  unauthorized: '認証が必要です',
  forbidden: 'アクセス権限がありません',
  notFound: 'リソースが見つかりません',
  validation: 'バリデーションエラーが発生しました',
  server: 'サーバーエラーが発生しました',
  unknown: '予期しないエラーが発生しました',
} as const

// 成功メッセージ
export const SUCCESS_MESSAGES = {
  todoCreated: 'ToDo を作成しました',
  todoUpdated: 'ToDo を更新しました',
  todoCompleted: 'ToDo を完了しました',
  todoDeleted: 'ToDo を削除しました',
  loginSuccess: 'ログインしました',
  logoutSuccess: 'ログアウトしました',
} as const

// ローカルストレージキー
export const STORAGE_KEYS = {
  authTokens: 'family-todo-auth-tokens',
  userPreferences: 'family-todo-user-preferences',
  todoFilters: 'family-todo-filters',
} as const

// ルート定数
export const ROUTES = {
  home: '/',
  login: '/login',
  todos: '/todos',
  todoDetail: '/todos/:id',
  profile: '/profile',
  family: '/family',
  settings: '/settings',
} as const

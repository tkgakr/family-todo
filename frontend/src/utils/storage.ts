// ローカルストレージ関連のユーティリティ
import { STORAGE_KEYS } from '@/config/constants'

// 型安全なローカルストレージアクセス
export const Storage = {
  // データの保存
  set<T>(key: string, value: T): void {
    try {
      const serializedValue = JSON.stringify(value)
      localStorage.setItem(key, serializedValue)
    } catch (error) {
      console.error('ローカルストレージへの保存に失敗しました:', error)
    }
  },

  // データの取得
  get<T>(key: string): T | null {
    try {
      const item = localStorage.getItem(key)
      if (item === null) {
        return null
      }
      return JSON.parse(item) as T
    } catch (error) {
      console.error('ローカルストレージからの取得に失敗しました:', error)
      return null
    }
  },

  // データの削除
  remove(key: string): void {
    try {
      localStorage.removeItem(key)
    } catch (error) {
      console.error('ローカルストレージからの削除に失敗しました:', error)
    }
  },

  // すべてのデータをクリア
  clear(): void {
    try {
      localStorage.clear()
    } catch (error) {
      console.error('ローカルストレージのクリアに失敗しました:', error)
    }
  },

  // キーの存在確認
  has(key: string): boolean {
    return localStorage.getItem(key) !== null
  },
} as const

// 認証トークンの管理
export interface AuthTokens {
  accessToken: string
  idToken: string
  refreshToken: string
  expiresAt: number
}

export const AuthStorage = {
  setTokens(tokens: AuthTokens): void {
    Storage.set(STORAGE_KEYS.authTokens, tokens)
  },

  getTokens(): AuthTokens | null {
    return Storage.get<AuthTokens>(STORAGE_KEYS.authTokens)
  },

  removeTokens(): void {
    Storage.remove(STORAGE_KEYS.authTokens)
  },

  hasValidTokens(): boolean {
    const tokens = AuthStorage.getTokens()
    if (!tokens) {
      return false
    }

    // トークンの有効期限をチェック
    return tokens.expiresAt > Date.now()
  },
} as const

// ユーザー設定の管理
export interface UserPreferences {
  theme: 'light' | 'dark' | 'system'
  language: 'ja' | 'en'
  todoSortBy: 'createdAt' | 'updatedAt' | 'title'
  todoSortOrder: 'asc' | 'desc'
  showCompletedTodos: boolean
  pageSize: number
}

const defaultPreferences: UserPreferences = {
  theme: 'system',
  language: 'ja',
  todoSortBy: 'createdAt',
  todoSortOrder: 'desc',
  showCompletedTodos: false,
  pageSize: 20,
}

export const PreferencesStorage = {
  setPreferences(preferences: Partial<UserPreferences>): void {
    const current = PreferencesStorage.getPreferences()
    const updated = { ...current, ...preferences }
    Storage.set(STORAGE_KEYS.userPreferences, updated)
  },

  getPreferences(): UserPreferences {
    const stored = Storage.get<UserPreferences>(STORAGE_KEYS.userPreferences)
    return { ...defaultPreferences, ...stored }
  },

  resetPreferences(): void {
    Storage.set(STORAGE_KEYS.userPreferences, defaultPreferences)
  },
} as const

// ToDo フィルターの管理
export interface TodoFilters {
  status: 'all' | 'active' | 'completed'
  tags: string[]
  search: string
  dateRange?: {
    start: string
    end: string
  }
}

const defaultFilters: TodoFilters = {
  status: 'active',
  tags: [],
  search: '',
}

export const FilterStorage = {
  setFilters(filters: Partial<TodoFilters>): void {
    const current = FilterStorage.getFilters()
    const updated = { ...current, ...filters }
    Storage.set(STORAGE_KEYS.todoFilters, updated)
  },

  getFilters(): TodoFilters {
    const stored = Storage.get<TodoFilters>(STORAGE_KEYS.todoFilters)
    return { ...defaultFilters, ...stored }
  },

  resetFilters(): void {
    Storage.set(STORAGE_KEYS.todoFilters, defaultFilters)
  },
} as const

// セッションストレージのユーティリティ
export const SessionStorage = {
  set<T>(key: string, value: T): void {
    try {
      const serializedValue = JSON.stringify(value)
      sessionStorage.setItem(key, serializedValue)
    } catch (error) {
      console.error('セッションストレージへの保存に失敗しました:', error)
    }
  },

  get<T>(key: string): T | null {
    try {
      const item = sessionStorage.getItem(key)
      if (item === null) {
        return null
      }
      return JSON.parse(item) as T
    } catch (error) {
      console.error('セッションストレージからの取得に失敗しました:', error)
      return null
    }
  },

  remove(key: string): void {
    try {
      sessionStorage.removeItem(key)
    } catch (error) {
      console.error('セッションストレージからの削除に失敗しました:', error)
    }
  },

  clear(): void {
    try {
      sessionStorage.clear()
    } catch (error) {
      console.error('セッションストレージのクリアに失敗しました:', error)
    }
  },
} as const

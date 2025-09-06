// API 関連のユーティリティ
import { API_CONFIG, ERROR_MESSAGES } from '@/config/constants'
import type { ApiErrorResponse, ApiRequestOptions } from '@/types/api'

// API エラークラス
export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
    public details?: Record<string, unknown>
  ) {
    super(message)
    this.name = 'ApiError'
  }

  static fromResponse(response: Response, data?: ApiErrorResponse): ApiError {
    const message = data?.message || this.getDefaultErrorMessage(response.status)
    const code = data?.code || response.status.toString()

    return new ApiError(response.status, code, message, data?.details)
  }

  private static getDefaultErrorMessage(status: number): string {
    switch (status) {
      case 400:
        return ERROR_MESSAGES.validation
      case 401:
        return ERROR_MESSAGES.unauthorized
      case 403:
        return ERROR_MESSAGES.forbidden
      case 404:
        return ERROR_MESSAGES.notFound
      case 500:
        return ERROR_MESSAGES.server
      default:
        return ERROR_MESSAGES.unknown
    }
  }
}

// HTTP クライアントクラス
export class HttpClient {
  private baseUrl: string
  private defaultHeaders: Record<string, string>

  constructor(baseUrl: string, defaultHeaders: Record<string, string> = {}) {
    this.baseUrl = baseUrl.replace(/\/$/, '') // 末尾のスラッシュを削除
    this.defaultHeaders = {
      'Content-Type': 'application/json',
      ...defaultHeaders,
    }
  }

  // GET リクエスト
  async get<T>(endpoint: string, options: ApiRequestOptions = {}): Promise<T> {
    return this.request<T>('GET', endpoint, undefined, options)
  }

  // POST リクエスト
  async post<T>(endpoint: string, data?: unknown, options: ApiRequestOptions = {}): Promise<T> {
    return this.request<T>('POST', endpoint, data, options)
  }

  // PUT リクエスト
  async put<T>(endpoint: string, data?: unknown, options: ApiRequestOptions = {}): Promise<T> {
    return this.request<T>('PUT', endpoint, data, options)
  }

  // PATCH リクエスト
  async patch<T>(endpoint: string, data?: unknown, options: ApiRequestOptions = {}): Promise<T> {
    return this.request<T>('PATCH', endpoint, data, options)
  }

  // DELETE リクエスト
  async delete<T>(endpoint: string, options: ApiRequestOptions = {}): Promise<T> {
    return this.request<T>('DELETE', endpoint, undefined, options)
  }

  // 基本的なリクエストメソッド
  private async request<T>(
    method: string,
    endpoint: string,
    data?: unknown,
    options: ApiRequestOptions = {}
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`
    const headers = { ...this.defaultHeaders, ...options.headers }
    const timeout = options.timeout || API_CONFIG.timeout
    const retries = options.retries || API_CONFIG.retryAttempts

    const requestOptions: RequestInit = {
      method,
      headers,
      body: data ? JSON.stringify(data) : null,
    }

    return this.executeWithRetry<T>(url, requestOptions, timeout, retries)
  }

  // リトライ機能付きリクエスト実行
  private async executeWithRetry<T>(
    url: string,
    options: RequestInit,
    timeout: number,
    retries: number
  ): Promise<T> {
    let lastError: Error | undefined

    for (let attempt = 0; attempt <= retries; attempt++) {
      try {
        const controller = new AbortController()
        const timeoutId = setTimeout(() => controller.abort(), timeout)

        const response = await fetch(url, {
          ...options,
          signal: controller.signal,
        })

        clearTimeout(timeoutId)

        if (!response.ok) {
          const errorData = await this.parseErrorResponse(response)
          throw ApiError.fromResponse(response, errorData)
        }

        // レスポンスが空の場合の処理
        const contentType = response.headers.get('content-type')
        if (!contentType || !contentType.includes('application/json')) {
          return {} as T
        }

        return await response.json()
      } catch (error) {
        lastError = error as Error

        // リトライ不可能なエラーの場合は即座に投げる
        if (error instanceof ApiError && !this.isRetryableError(error)) {
          throw error
        }

        // 最後の試行の場合はエラーを投げる
        if (attempt === retries) {
          throw lastError
        }

        // リトライ前の待機
        await this.delay(this.calculateRetryDelay(attempt))
      }
    }

    throw lastError || new Error('Unknown error occurred')
  }

  // エラーレスポンスのパース
  private async parseErrorResponse(response: Response): Promise<ApiErrorResponse | undefined> {
    try {
      const contentType = response.headers.get('content-type')
      if (contentType && contentType.includes('application/json')) {
        return await response.json()
      }
    } catch {
      // JSON パースに失敗した場合は undefined を返す
    }
    return undefined
  }

  // リトライ可能なエラーかどうかの判定
  private isRetryableError(error: ApiError): boolean {
    // 5xx エラーまたは特定の 4xx エラーはリトライ可能
    return (
      error.status >= 500 ||
      error.status === 408 || // Request Timeout
      error.status === 429 // Too Many Requests
    )
  }

  // リトライ遅延の計算（指数バックオフ）
  private calculateRetryDelay(attempt: number): number {
    const baseDelay = API_CONFIG.retryDelay
    const maxDelay = API_CONFIG.maxRetryDelay
    const delay = Math.min(baseDelay * Math.pow(2, attempt), maxDelay)

    // ジッターを追加してサーバーへの負荷を分散
    return delay + Math.random() * 1000
  }

  // 遅延ユーティリティ
  private delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms))
  }

  // 認証ヘッダーの設定
  setAuthToken(token: string): void {
    this.defaultHeaders['Authorization'] = `Bearer ${token}`
  }

  // 認証ヘッダーの削除
  removeAuthToken(): void {
    delete this.defaultHeaders['Authorization']
  }
}

// クエリパラメータの構築
export function buildQueryParams(params: Record<string, unknown>): string {
  const searchParams = new URLSearchParams()

  for (const [key, value] of Object.entries(params)) {
    if (value !== null && value !== undefined && value !== '') {
      if (Array.isArray(value)) {
        for (const item of value) {
          searchParams.append(key, String(item))
        }
      } else {
        searchParams.append(key, String(value))
      }
    }
  }

  const queryString = searchParams.toString()
  return queryString ? `?${queryString}` : ''
}

// URL の構築
export function buildUrl(baseUrl: string, path: string, params?: Record<string, unknown>): string {
  const cleanBaseUrl = baseUrl.replace(/\/$/, '')
  const cleanPath = path.replace(/^\//, '')
  const queryString = params ? buildQueryParams(params) : ''

  return `${cleanBaseUrl}/${cleanPath}${queryString}`
}

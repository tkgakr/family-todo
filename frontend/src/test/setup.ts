import '@testing-library/jest-dom'
import { vi } from 'vitest'

// Mock AWS Amplify
vi.mock('aws-amplify', () => ({
  Amplify: {
    configure: vi.fn(),
  },
}))

vi.mock('@aws-amplify/auth', () => ({
  signIn: vi.fn(),
  signOut: vi.fn(),
  getCurrentUser: vi.fn(),
  fetchAuthSession: vi.fn(),
}))

vi.mock('@aws-amplify/core', () => ({
  Amplify: {
    configure: vi.fn(),
  },
}))

// Mock environment variables
Object.defineProperty(import.meta, 'env', {
  value: {
    VITE_API_URL: 'http://localhost:8080',
    VITE_COGNITO_USER_POOL_ID: 'test-pool-id',
    VITE_COGNITO_CLIENT_ID: 'test-client-id',
    VITE_AWS_REGION: 'ap-northeast-1',
    VITE_LOG_LEVEL: 'debug',
    VITE_ENABLE_PASSKEY: 'true',
    VITE_ENABLE_REALTIME: 'false',
    VITE_ENABLE_ANALYTICS: 'false',
    DEV: true,
    PROD: false,
  },
  writable: true,
})

// Mock window location
Object.defineProperty(window, 'location', {
  value: {
    href: 'http://localhost:3000',
    origin: 'http://localhost:3000',
    pathname: '/',
    search: '',
    hash: '',
  },
  writable: true,
})

// Mock localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
}
Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
})

// Mock sessionStorage
const sessionStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
}
Object.defineProperty(window, 'sessionStorage', {
  value: sessionStorageMock,
})

// Mock fetch
;(globalThis as any).fetch = vi.fn()

// Mock console methods for cleaner test output
;(globalThis as any).console = {
  ...console,
  log: vi.fn(),
  debug: vi.fn(),
  info: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
}

// Mock WebAuthn API
Object.defineProperty(window, 'PublicKeyCredential', {
  value: {
    isUserVerifyingPlatformAuthenticatorAvailable: vi.fn().mockResolvedValue(true),
    isConditionalMediationAvailable: vi.fn().mockResolvedValue(true),
  },
  writable: true,
})

Object.defineProperty(navigator, 'credentials', {
  value: {
    create: vi.fn(),
    get: vi.fn(),
  },
  writable: true,
})

// Reset all mocks before each test
;(globalThis as any).beforeEach(() => {
  vi.clearAllMocks()
  localStorageMock.getItem.mockClear()
  localStorageMock.setItem.mockClear()
  localStorageMock.removeItem.mockClear()
  localStorageMock.clear.mockClear()
  sessionStorageMock.getItem.mockClear()
  sessionStorageMock.setItem.mockClear()
  sessionStorageMock.removeItem.mockClear()
  sessionStorageMock.clear.mockClear()
})

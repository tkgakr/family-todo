import axios from 'axios'
import { getCurrentUser, fetchAuthSession } from 'aws-amplify/auth'

const API_BASE_URL = import.meta.env.VITE_API_ENDPOINT || 'http://localhost:3001'

export const apiClient = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
})

// Request interceptor to add auth headers
apiClient.interceptors.request.use(
  async (config) => {
    try {
      const session = await fetchAuthSession()
      const user = await getCurrentUser()
      
      if (session.tokens?.idToken) {
        config.headers.Authorization = `Bearer ${session.tokens.idToken.toString()}`
      }
      
      if (user.userId) {
        config.headers['X-User-Id'] = user.userId
      }
      
      // Add family ID from user attributes if available
      const familyId = user.signInDetails?.loginId || 'default-family'
      config.headers['X-Family-Id'] = familyId
      
    } catch (error) {
      console.warn('Failed to add auth headers:', error)
    }
    
    return config
  },
  (error) => {
    return Promise.reject(error)
  }
)

// Response interceptor for error handling
apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      // Handle unauthorized - redirect to login
      window.location.href = '/login'
    }
    return Promise.reject(error)
  }
)
import type { ResourcesConfig } from 'aws-amplify'

export const amplifyConfig: ResourcesConfig = {
  Auth: {
    Cognito: {
      userPoolId: import.meta.env.VITE_USER_POOL_ID || '',
      userPoolClientId: import.meta.env.VITE_USER_POOL_CLIENT_ID || '',
      identityPoolId: import.meta.env.VITE_IDENTITY_POOL_ID || '',
      loginWith: {
        oauth: {
          domain: import.meta.env.VITE_OAUTH_DOMAIN || '',
          scopes: ['openid', 'email', 'profile'],
          redirectSignIn: import.meta.env.VITE_OAUTH_REDIRECT_SIGN_IN || 'http://localhost:3000/',
          redirectSignOut: import.meta.env.VITE_OAUTH_REDIRECT_SIGN_OUT || 'http://localhost:3000/',
          responseType: 'code',
        },
        username: true,
        email: true,
      },
    },
  },
  API: {
    REST: {
      'todo-api': {
        endpoint: import.meta.env.VITE_API_ENDPOINT || 'http://localhost:3001',
        region: import.meta.env.VITE_AWS_REGION || 'ap-northeast-1',
      },
    },
  },
}
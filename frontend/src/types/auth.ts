// 認証関連の型定義
export interface CognitoUser {
  username: string
  attributes: {
    email: string
    email_verified: boolean
    family_id?: string
    given_name?: string
    family_name?: string
  }
}

export interface AuthTokens {
  accessToken: string
  idToken: string
  refreshToken: string
  expiresAt: number
}

export interface PasskeyCredential {
  id: string
  rawId: ArrayBuffer
  response: AuthenticatorAssertionResponse
  type: 'public-key'
}

export interface PasskeyRegistrationOptions {
  challenge: string
  rp: {
    name: string
    id: string
  }
  user: {
    id: string
    name: string
    displayName: string
  }
  pubKeyCredParams: Array<{
    type: 'public-key'
    alg: number
  }>
  authenticatorSelection?: {
    authenticatorAttachment?: 'platform' | 'cross-platform'
    userVerification?: 'required' | 'preferred' | 'discouraged'
  }
  timeout?: number
}

export interface PasskeyAuthenticationOptions {
  challenge: string
  allowCredentials?: Array<{
    type: 'public-key'
    id: string
  }>
  userVerification?: 'required' | 'preferred' | 'discouraged'
  timeout?: number
}

export interface AuthError {
  name: string
  message: string
  code?: string
}

export interface InvitationToken {
  token: string
  familyId: string
  invitedBy: string
  email: string
  expiresAt: string
  used: boolean
}

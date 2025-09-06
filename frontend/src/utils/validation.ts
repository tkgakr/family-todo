// バリデーション関連のユーティリティ
import { VALIDATION } from '@/config/constants'

export interface ValidationResult {
  isValid: boolean
  errors: string[]
}

// ToDo タイトルのバリデーション
export function validateTodoTitle(title: string): ValidationResult {
  const errors: string[] = []

  if (!title || title.trim().length === 0) {
    errors.push('タイトルは必須です')
  } else if (title.length > VALIDATION.todo.title.maxLength) {
    errors.push(`タイトルは${VALIDATION.todo.title.maxLength}文字以内で入力してください`)
  }

  return {
    isValid: errors.length === 0,
    errors,
  }
}

// ToDo 説明のバリデーション
export function validateTodoDescription(description?: string): ValidationResult {
  const errors: string[] = []

  if (description && description.length > VALIDATION.todo.description.maxLength) {
    errors.push(`説明は${VALIDATION.todo.description.maxLength}文字以内で入力してください`)
  }

  return {
    isValid: errors.length === 0,
    errors,
  }
}

// タグのバリデーション
export function validateTodoTags(tags: string[]): ValidationResult {
  const errors: string[] = []

  if (tags.length > VALIDATION.todo.tags.maxCount) {
    errors.push(`タグは${VALIDATION.todo.tags.maxCount}個まで設定できます`)
  }

  for (const tag of tags) {
    if (tag.length > VALIDATION.todo.tags.maxLength) {
      errors.push(`タグは${VALIDATION.todo.tags.maxLength}文字以内で入力してください`)
    }
  }

  return {
    isValid: errors.length === 0,
    errors,
  }
}

// メールアドレスのバリデーション
export function validateEmail(email: string): ValidationResult {
  const errors: string[] = []

  if (!email || email.trim().length === 0) {
    errors.push('メールアドレスは必須です')
  } else if (!VALIDATION.user.email.pattern.test(email)) {
    errors.push('有効なメールアドレスを入力してください')
  }

  return {
    isValid: errors.length === 0,
    errors,
  }
}

// 汎用的なバリデーション関数
export function validateRequired(value: unknown, fieldName: string): ValidationResult {
  const errors: string[] = []

  if (value === null || value === undefined || value === '') {
    errors.push(`${fieldName}は必須です`)
  }

  return {
    isValid: errors.length === 0,
    errors,
  }
}

// 文字列長のバリデーション
export function validateLength(
  value: string,
  fieldName: string,
  minLength?: number,
  maxLength?: number
): ValidationResult {
  const errors: string[] = []

  if (minLength !== undefined && value.length < minLength) {
    errors.push(`${fieldName}は${minLength}文字以上で入力してください`)
  }

  if (maxLength !== undefined && value.length > maxLength) {
    errors.push(`${fieldName}は${maxLength}文字以内で入力してください`)
  }

  return {
    isValid: errors.length === 0,
    errors,
  }
}

// 複数のバリデーション結果をマージ
export function mergeValidationResults(...results: ValidationResult[]): ValidationResult {
  const allErrors = results.flatMap((result) => result.errors)

  return {
    isValid: allErrors.length === 0,
    errors: allErrors,
  }
}

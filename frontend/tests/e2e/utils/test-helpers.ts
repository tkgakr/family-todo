import { Page } from '@playwright/test';

/**
 * E2Eテストのヘルパー関数
 */

/**
 * 認証済みユーザーとしてログインする（モック）
 * 実際の認証設定後に実装予定
 */
export async function loginAsTestUser(page: Page) {
  // TODO: 実際のAWS Cognitoテストユーザーでのログイン処理
  // 現在はプレースホルダー
  console.log('Mock login - authentication setup required');
}

/**
 * テストデータの初期化
 */
export async function setupTestData(page: Page) {
  // TODO: テスト用Todoデータの作成
  // バックエンドAPIまたはDynamoDB Localへのデータ挿入
  console.log('Mock test data setup - backend integration required');
}

/**
 * テストデータのクリーンアップ
 */
export async function cleanupTestData(page: Page) {
  // TODO: テスト後のデータクリーンアップ
  console.log('Mock test data cleanup - backend integration required');
}

/**
 * ネットワークリクエストの待機
 */
export async function waitForApiResponse(page: Page, endpoint: string) {
  await page.waitForResponse(response => 
    response.url().includes(endpoint) && response.status() === 200
  );
}

/**
 * エラーハンドリングのテスト用ヘルパー
 */
export async function mockApiError(page: Page, endpoint: string, statusCode: number) {
  await page.route(`**/${endpoint}`, route => {
    route.fulfill({
      status: statusCode,
      contentType: 'application/json',
      body: JSON.stringify({ error: 'Mocked error for testing' })
    });
  });
}
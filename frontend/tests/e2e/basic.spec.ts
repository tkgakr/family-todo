import { test, expect } from '@playwright/test';

test.describe('Basic Application Tests', () => {
  test('should display the application title', async ({ page }) => {
    await page.goto('/');
    
    // Amplify Authenticatorが表示されることを確認
    // 認証が必要なアプリケーションの場合、ログインフォームが表示される
    await expect(page).toHaveTitle(/Family Todo/);
  });

  test('should show authentication form when not logged in', async ({ page }) => {
    await page.goto('/');
    
    // AWS Amplifyの認証フォームが表示されることを確認
    // Note: 実際のAWS Cognitoの設定に依存するため、テスト環境での設定が必要
    const authForm = page.locator('[data-amplify-authenticator]');
    await expect(authForm).toBeVisible({ timeout: 10000 });
  });

  test('should be responsive on mobile viewports', async ({ page }) => {
    // モバイルサイズでの表示確認
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    
    // 基本的な要素が表示されることを確認
    await expect(page).toHaveTitle(/Family Todo/);
  });
});
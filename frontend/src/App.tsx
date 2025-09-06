import { APP_CONFIG } from '@/config/constants'
import { environment, isDevelopment } from '@/config/environment'

function App() {
  return (
    <div className="app">
      <header>
        <h1>{APP_CONFIG.name}</h1>
        {isDevelopment && (
          <div style={{ fontSize: '0.8rem', color: '#666' }}>
            開発モード - API: {environment.apiUrl}
          </div>
        )}
      </header>
      <main>
        <p>React + TypeScript + Vite プロジェクトが構築されました。</p>
        <ul>
          <li>✅ TypeScript 設定完了</li>
          <li>✅ Biome リンティング・フォーマット設定完了</li>
          <li>✅ 型定義ファイル作成完了</li>
          <li>✅ ユーティリティ関数作成完了</li>
          <li>✅ 環境設定完了</li>
          <li>✅ テストセットアップ完了</li>
        </ul>
      </main>
    </div>
  )
}

export default App

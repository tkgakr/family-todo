import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'

// 型安全なルート要素の取得
const rootElement = document.getElementById('root')
if (!rootElement) {
  throw new Error('Root element not found. Make sure there is a div with id="root" in your HTML.')
}

// React アプリケーションのマウント
ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)

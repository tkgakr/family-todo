import { resolve } from 'node:path'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [
    react({
      // React の最適化設定
      babel: {
        plugins: [
          // 必要に応じて Babel プラグインを追加
        ],
      },
    }),
  ],

  // パス解決の設定
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
      '@/types': resolve(__dirname, './src/types'),
      '@/config': resolve(__dirname, './src/config'),
      '@/components': resolve(__dirname, './src/components'),
      '@/hooks': resolve(__dirname, './src/hooks'),
      '@/services': resolve(__dirname, './src/services'),
      '@/utils': resolve(__dirname, './src/utils'),
    },
  },

  // 開発サーバー設定
  server: {
    port: 3000,
    host: true,
    open: false,
    cors: true,
    proxy: {
      // ローカル開発時のAPI プロキシ設定
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, ''),
      },
    },
  },

  // ビルド設定
  build: {
    outDir: 'dist',
    sourcemap: true,
    minify: 'esbuild',
    target: 'es2020',
    rollupOptions: {
      output: {
        manualChunks: {
          // ベンダーライブラリの分割
          vendor: ['react', 'react-dom'],
          aws: ['@aws-amplify/auth', '@aws-amplify/core', 'aws-amplify'],
          router: ['react-router-dom'],
        },
      },
    },
    // バンドルサイズの警告しきい値
    chunkSizeWarningLimit: 1000,
  },

  // 最適化設定
  optimizeDeps: {
    include: ['react', 'react-dom', 'react-router-dom', '@aws-amplify/auth', '@aws-amplify/core'],
  },

  // テスト設定
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: ['node_modules/', 'src/test/', '**/*.d.ts', '**/*.config.*'],
    },
  },

  // 環境変数の設定
  define: {
    __APP_VERSION__: JSON.stringify(process.env.npm_package_version),
  },
})

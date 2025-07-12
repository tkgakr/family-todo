# 家族用TODOアプリ

このリポジトリは、Rust + React/TypeScriptで実装する家族用TODO共有アプリのコードベースです。AWSのサーバーレスアーキテクチャを活用して、低コスト・高セキュリティでデプロイします。

詳細な技術アーキテクチャについては [PLANNING.md](./PLANNING.md) を参照してください。

**現在の開発状況**: マイルストーン0「基盤セットアップ」完了 - CI/CDパイプライン稼働中

## プロジェクト構成

```
/
├── .github/            # GitHub Actions ワークフロー
│   └── workflows/
│       ├── backend.yml     # バックエンドCI/CD
│       └── frontend.yml    # フロントエンドCI/CD
├── infra/              # SAM テンプレート
│   ├── template.yaml   # SAM リソース定義
│   ├── samconfig.toml  # SAM設定ファイル
│   └── events/
│       └── event.json  # ローカルテスト用イベント
├── backend/            # Rust Lambda関数
│   ├── src/            # ソースコード
│   │   ├── main.rs     # エントリーポイント
│   │   └── http_handler.rs # HTTPリクエスト処理
│   ├── tests/         # テストコード
│   │   └── api_integration_tests.rs # API統合テスト
│   └── Cargo.toml     # Rust依存関係
├── frontend/           # React/TS + Biome
│   ├── src/
│   │   ├── App.tsx     # メインコンポーネント
│   │   ├── App.test.tsx # テストファイル
│   │   └── main.tsx    # エントリーポイント
│   ├── package.json    # NPM設定・スクリプト
│   ├── biome.json      # Biome設定（リント・フォーマット）
│   ├── tsconfig.json   # TypeScript設定
│   ├── vite.config.ts  # Vite設定
│   └── index.html      # HTMLテンプレート
├── PLANNING.md         # 詳細な技術アーキテクチャ計画
├── CLAUDE.md           # Claude Code向けガイダンス
└── README.md           # このファイル
```

## 使用技術

- **フロントエンド**: React, TypeScript, Vite, Biome (リント・フォーマット)
- **バックエンド**: Rust, Lambda HTTP, cargo-lambda
- **インフラ**: AWS (Lambda, API Gateway, DynamoDB, Cognito, S3, CloudFront)
- **IaC**: AWS SAM (将来的にTerraform/CDKへ移行予定)
- **CI/CD**: GitHub Actions (backend.yml, frontend.yml)
- **コード品質**: Biome (Frontend), Clippy (Backend)

## 開発環境のセットアップ

### バックエンド (Rust)

1. **Rustのインストール**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **cargo-lambdaのインストール**
   ```bash
   cargo install cargo-lambda
   ```

3. **依存関係のインストール**
   ```bash
   cd backend
   cargo build
   ```

### フロントエンド (React/TypeScript)

1. **Node.js 20のインストール**
   ```bash
   # Node.js 20をインストール（推奨）
   ```

2. **依存関係のインストール**
   ```bash
   cd frontend
   npm ci
   ```

3. **開発サーバー起動**
   ```bash
   npm run dev
   ```

### AWS SAM (インフラ)

1. **AWS SAM CLIのインストール**
   ```bash
   # macOS
   brew install aws-sam-cli
   ```

2. **AWS認証情報の設定**
   ```bash
   aws configure
   ```

## ローカル開発・テスト

### バックエンドテスト

```bash
# テストの実行
cd backend
cargo test

# Clippyによるコード品質チェック
cargo clippy -- -D warnings
```

### フロントエンドテスト

```bash
cd frontend

# テストの実行
npm run test

# リント・フォーマットチェック
npm run lint
npm run format:check

# フォーマット自動修正
npm run format
```

### SAMローカル実行

```bash
cd infra

# ビルド（beta-featuresフラグが必要）
sam build --beta-features

# ローカルAPI起動
sam local start-api

# Lambda関数の単体テスト
sam local invoke todoHandler -e events/event.json
```

### 今後導入予定のツール
- **LocalStack**: Cognito / DynamoDB エミュレーション
- **mkcert**: `https://localhost` でパスキー動作確認
- **DynamoDB Local**: オフラインDBテスト
- **VS Code Dev Container**: 開発環境の標準化

## ライセンス

(準備中)

## プロジェクト進捗

### マイルストーン0: 基盤セットアップ ✅ 完了
- [x] プロジェクト初期設定
- [x] バックエンド基盤構築（Rust Lambda）
- [x] インフラ基盤構築（AWS SAM）
- [x] バックエンドテスト実装
- [x] フロントエンド基盤構築（React + TypeScript + Biome）
- [x] CI/CDパイプライン構築（GitHub Actions）

### マイルストーン1: イベントストア実装 🚧 次の目標
- [ ] DynamoDB統合（イベントストア・プロジェクションテーブル）
- [ ] ULID実装
- [ ] CommandHandlerでのイベント保存
- [ ] DynamoDB Streams設定

### 将来のマイルストーン
- [ ] イベントプロセッサー実装
- [ ] 認証機能（Cognito + Passkey）
- [ ] クエリAPI実装
- [ ] フロントエンドUI開発
- [ ] 統合テスト・E2Eテスト

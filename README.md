# 家族用 ToDo アプリ - AWS サーバーレス版

イベントソーシング + CQRS アーキテクチャを採用したサーバーレス家族用 ToDo アプリです。

## アーキテクチャ

- **バックエンド**: Rust + AWS Lambda
- **フロントエンド**: React + TypeScript + Vite
- **データベース**: DynamoDB (Single Table Design)
- **認証**: Amazon Cognito (Passkey 対応)
- **API**: API Gateway
- **監視**: CloudWatch + X-Ray
- **CI/CD**: GitHub Actions + AWS SAM

## 開発環境のセットアップ

### 前提条件

- Rust (最新安定版)
- Node.js 20+
- AWS CLI
- SAM CLI
- cargo-lambda

### バックエンド開発

```bash
# 依存関係のインストール
cargo build

# テスト実行
cargo test

# Lambda関数のビルド
cargo lambda build --release

# ローカルでのAPI起動
sam local start-api
```

### フロントエンド開発

```bash
cd frontend

# 依存関係のインストール
npm install

# 開発サーバー起動
npm run dev

# テスト実行
npm run test

# ビルド
npm run build
```

## デプロイ

### 開発環境

```bash
sam deploy --config-env default
```

### 本番環境

```bash
sam deploy --config-env production
```

## プロジェクト構造

```
├── crates/                 # Rustワークスペース
│   ├── domain/            # ドメインロジック
│   ├── infrastructure/    # インフラストラクチャ層
│   ├── command-handler/   # コマンドハンドラー Lambda
│   ├── query-handler/     # クエリハンドラー Lambda
│   ├── event-processor/   # イベントプロセッサー Lambda
│   ├── snapshot-manager/  # スナップショット管理 Lambda
│   └── shared/           # 共通ライブラリ
├── frontend/              # React フロントエンド
├── template.yaml          # SAM テンプレート
├── .github/workflows/     # CI/CD パイプライン
└── docs/                 # ドキュメント
```

## 機能

- [ ] ToDo の作成・編集・完了・削除
- [ ] 家族メンバー管理
- [ ] Passkey 認証
- [ ] リアルタイム同期
- [ ] 履歴管理（イベントソーシング）
- [ ] GDPR 対応（データ削除）

## 開発ガイドライン

- イベントソーシングパターンに従う
- テスト駆動開発（TDD）を実践
- セキュリティベストプラクティスを遵守
- パフォーマンス要件を満たす（要件書参照）

## ライセンス

MIT License

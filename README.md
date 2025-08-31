# Family Todo App

AWS サーバーレス環境で動作する家族向け ToDo 共有アプリケーション

## 概要

このプロジェクトは、Rust + React/TypeScript を使用して構築された、イベントソーシングアーキテクチャを採用した家族向け ToDo 管理アプリケーションです。AWS サーバーレスサービスを活用して、低コスト・高セキュリティ・高可用性を実現しています。

## 主な技術要素

- **バックエンド**: Rust + AWS Lambda
- **フロントエンド**: React/TypeScript + Vite + Tailwind CSS + Biome
- **データベース**: Amazon DynamoDB (Single Table Design)
- **認証**: Amazon Cognito User Pool + JWT
- **インフラ**: AWS SAM
- **CI/CD**: GitHub Actions (準備中)
- **アーキテクチャ**: イベントソーシング + CQRS
- **識別子**: ULID (時系列ソート可能)
- **監視**: AWS X-Ray + CloudWatch

## プロジェクト構成

```
family-todo-claude/
├── backend/               # Rust Lambda 関数群
│   ├── shared/           # 共通ドメインモデル・インフラ層
│   ├── command-handler/  # 書き込み処理 Lambda
│   ├── query-handler/    # 読み取り処理 Lambda
│   └── event-processor/  # イベント処理 Lambda
├── frontend/             # React/TypeScript SPA
├── infra/                # AWS SAM テンプレート
├── tests/                # 統合テスト (準備中)
├── docs/                 # ドキュメント
│   ├── api/              # API ドキュメント
│   ├── architecture/     # アーキテクチャ設計書
│   ├── DEVELOPMENT.md    # 開発ガイド
│   └── DEPLOYMENT.md     # デプロイガイド
└── env.json              # ローカル環境変数
```

## セットアップ

### 前提条件

- Rust 1.75+
- Node.js 20+
- AWS CLI
- SAM CLI
- Docker (ローカル開発用)

### ローカル開発環境

1. **リポジトリのクローン**
```bash
git clone https://github.com/your-org/family-todo-claude.git
cd family-todo-claude
```

2. **バックエンドの起動**
```bash
# SAM でローカル API を起動
sam build --use-container
sam local start-api --warm-containers EAGER --port 3001 --env-vars env.json
```

3. **フロントエンドの起動**
```bash
cd frontend
npm install
npm run dev
```

4. **DynamoDB Local の起動** (オプション)
```bash
docker run -p 8000:8000 amazon/dynamodb-local:latest \
  -jar DynamoDBLocal.jar -sharedDb -inMemory
```

### AWS デプロイ

1. **初回デプロイ**
```bash
cd infra
sam deploy --guided
```

2. **更新デプロイ**
```bash
sam deploy
```

詳細は [デプロイガイド](docs/DEPLOYMENT.md) を参照してください。

## 主な機能

### 実装済み
- ✅ ToDo の作成・更新・完了・削除
- ✅ 家族間でのToDo共有
- ✅ イベント履歴の追跡と再生
- ✅ ULID ベースの識別子管理
- ✅ イベントソーシング + CQRS アーキテクチャ
- ✅ DynamoDB Single Table Design
- ✅ レスポンシブな UI (React + Tailwind CSS)
- ✅ AWS SAM によるインフラ管理
- ✅ CloudWatch + X-Ray 監視

### 開発予定
- ⬜ WebAuthn (Passkey) 認証
- ⬜ リアルタイム同期 (WebSocket)
- ⬜ プッシュ通知
- ⬜ ファイル添付機能
- ⬜ 楽観的ロック実装
- ⬜ スナップショット機能

## API仕様

### エンドポイント

| メソッド | パス | 説明 |
|---------|------|------|
| GET | `/todos` | ToDo一覧取得 |
| GET | `/todos/{id}` | ToDo詳細取得 |
| POST | `/todos` | ToDo作成 |
| PUT | `/todos/{id}` | ToDo更新 |
| POST | `/todos/{id}/complete` | ToDo完了 |
| DELETE | `/todos/{id}` | ToDo削除 |
| GET | `/todos/{id}/history` | ToDo履歴取得 |

### 認証

現在は開発用にヘッダーベース認証を使用：
- `X-Family-Id`: 家族ID
- `X-User-Id`: ユーザーID

本番環境では Amazon Cognito による JWT 認証を使用します。

## テスト

### Rust (バックエンド)
```bash
cd backend
cargo test
```

### TypeScript (フロントエンド)
```bash
cd frontend
npm test
```

### 統合テスト (準備中)
```bash
cd tests
npm run test:integration
```

## アーキテクチャの特徴

### イベントソーシング

すべての変更がイベントとして記録され、現在の状態はイベントの再生によって構築されます：

- **TodoCreatedV2**: ToDo作成イベント
- **TodoUpdatedV1**: ToDo更新イベント  
- **TodoCompletedV1**: ToDo完了イベント
- **TodoDeletedV1**: ToDo削除イベント

### CQRS分離

読み取りと書き込みを分離：

- **Command Handler**: 書き込み処理（イベント生成）
- **Query Handler**: 読み取り処理（プロジェクション参照）
- **Event Processor**: イベントからプロジェクション更新

### Single Table Design

DynamoDB の効率的な利用のため、単一テーブルに全データを格納：

```
PK                    | SK                    | 用途
FAMILY#{familyId}     | EVENT#{eventId}       | イベント
FAMILY#{familyId}     | TODO#CURRENT#{todoId} | 現在のToDo状態
FAMILY#{familyId}#ACTIVE | {todoId}           | アクティブToDo一覧(GSI1)
```

## 監視・可観測性

- **AWS X-Ray**: 分散トレーシング
- **CloudWatch**: ログ・メトリクス
- **OpenTelemetry**: 構造化ログ出力
- **CloudWatch Alarms**: エラー率・レイテンシ監視

## セキュリティ

- **最小権限IAMロール**: 各Lambda関数に必要最小限の権限
- **VPC不要**: サーバーレスサービスのみ使用
- **データ暗号化**: DynamoDB暗号化、転送時TLS
- **認証**: Amazon Cognito + JWT

## 開発コマンド

### Rust (バックエンド)
```bash
cd backend

# フォーマット
cargo fmt

# リンター
cargo clippy -- -D warnings

# セキュリティ監査
cargo audit

# テスト
cargo test
```

### TypeScript (フロントエンド)
```bash
cd frontend

# Biome によるフォーマット・リント
npm run format
npm run lint

# テスト
npm test

# ビルド
npm run build
```

## ライセンス

MIT License

## 貢献

プルリクエストやイシュー報告を歓迎します。

## ドキュメント

- 📖 **[開発ガイド](docs/DEVELOPMENT.md)**: 開発環境のセットアップと実装方法
- 🚀 **[デプロイガイド](docs/DEPLOYMENT.md)**: AWS へのデプロイ手順と運用方法
- 🏗️ **[アーキテクチャ概要](docs/architecture/OVERVIEW.md)**: システム全体の設計思想
- 📋 **[詳細設計書](docs/architecture/PLANNING.md)**: 包括的なアーキテクチャ仕様
- 🔌 **[API ドキュメント](docs/api/README.md)**: REST API の仕様書

## 学習目的

このプロジェクトは以下の技術要素の学習を目的としています：

- **イベントソーシング + CQRS**: 大規模システムでの状態管理パターン
- **AWS サーバーレス**: Lambda, DynamoDB, API Gateway の実践的活用
- **Rust**: システムプログラミングでの型安全性とパフォーマンス
- **Single Table Design**: NoSQL データベースの効率的な設計
- **ULID**: 分散システムでの識別子管理
- **可観測性**: CloudWatch + X-Ray による包括的な監視
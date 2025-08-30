# Family Todo App

AWS サーバーレス環境で動作する家族向け ToDo 共有アプリケーション

## 概要

このプロジェクトは、Rust + React/TypeScript を使用して構築された、イベントソーシングアーキテクチャを採用した家族向け ToDo 管理アプリケーションです。AWS サーバーレスサービスを活用して、低コスト・高セキュリティ・高可用性を実現しています。

## 主な技術要素

- **バックエンド**: Rust + AWS Lambda
- **フロントエンド**: React/TypeScript + Vite + Tailwind CSS
- **データベース**: Amazon DynamoDB (Single Table Design)
- **認証**: Amazon Cognito (Passkey対応予定)
- **インフラ**: AWS SAM
- **CI/CD**: GitHub Actions
- **アーキテクチャ**: イベントソーシング + CQRS

## プロジェクト構成

```
/
├── backend/               # Rust Lambda 関数群
│   ├── shared/           # 共通ドメインモデル
│   ├── command-handler/  # 書き込み処理
│   ├── query-handler/    # 読み取り処理
│   └── event-processor/  # イベントストリーム処理
├── frontend/             # React/TypeScript SPA
├── infra/                # SAM テンプレート
├── tests/                # 統合テスト
└── docs/                 # ドキュメント
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
git clone <repository-url>
cd family-todo-claude
```

2. **ローカル開発環境の起動**
```bash
make dev-up
```

これにより以下が起動されます：
- DynamoDB Local
- LocalStack (Cognito, SES, SQS)
- フロントエンド開発サーバー
- バックエンドのウォッチモード

3. **開発環境の停止**
```bash
make dev-down
```

### AWS デプロイ

1. **依存関係のインストールとビルド**
```bash
make build
```

2. **AWSへのデプロイ**
```bash
cd infra
sam deploy --guided
```

## 主な機能

- ✅ ToDo の作成・更新・完了・削除
- ✅ 家族間でのToDo共有
- ✅ イベント履歴の追跡
- ✅ リアルタイムな状態同期
- ✅ モバイルフレンドリーなUI
- ⬜ Passkey認証 (実装予定)
- ⬜ プッシュ通知 (実装予定)

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

### 単体テスト
```bash
make test-unit
```

### 統合テスト
```bash
make test-integration
```

### スモークテスト
```bash
cd tests
npm run smoke-test
```

### 負荷テスト
```bash
cd tests/load
k6 run k6-script.js
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

```bash
# コードフォーマット
make fmt

# リンター実行
make lint

# 全テスト実行
make test

# ローカルAPIサーバー起動
make deploy-local

# ヘルプ表示
make help
```

## ライセンス

MIT License

## 貢献

プルリクエストやイシュー報告を歓迎します。

## 設計ドキュメント

詳細なアーキテクチャ設計は [docs/architecture/PLANNING.md](docs/architecture/PLANNING.md) を参照してください。
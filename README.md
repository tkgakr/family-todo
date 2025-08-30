# 家族用 ToDo アプリ - AWS サーバーレス版

イベントソーシング + CQRS アーキテクチャを採用したサーバーレス家族用 ToDo アプリです。Rust + React/TypeScript で構築され、AWS サーバーレスインフラストラクチャにデプロイされます。

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

### クイックスタート

```bash
# cargo-lambda のインストール
cargo install cargo-lambda

# プロジェクト依存関係のインストール
make install-deps

# ローカル開発環境の起動（DynamoDB Local含む）
make local-dev

# 別ターミナルでAPIサーバー起動
make sam-local

# 別ターミナルでフロントエンド起動
make frontend-dev
```

### 開発用 URL

- **フロントエンド**: http://localhost:3000
- **API Gateway**: http://localhost:8080
- **DynamoDB Local**: http://localhost:8000
- **DynamoDB Admin**: http://localhost:8001

### 主要な開発コマンド

```bash
# ビルドとテスト
make build                     # 全Rustクレートのビルド
make test                      # 全テストの実行
make lint                      # リンティング実行
make format                    # コードフォーマット

# ローカル開発
make local-dev                 # ローカルサービス起動
make sam-local                 # ローカルAPI起動
make frontend-dev              # フロントエンド開発サーバー起動

# デプロイ
make deploy-dev                # 開発環境へのデプロイ
make deploy-prod               # 本番環境へのデプロイ

# ユーティリティ
make logs-dev                  # 開発環境ログの確認
make validate-template         # SAMテンプレートの検証
make clean                     # ビルド成果物のクリーンアップ
```

## インフラストラクチャ

### AWS リソース

- **API Gateway**: Cognito JWT 認証付き HTTP API
- **Lambda 関数**: Rust ベースのサーバーレス関数
- **DynamoDB**: GSI 付きシングルテーブル設計
- **Cognito**: Passkey 対応ユーザー認証
- **CloudWatch**: ログ、メトリクス、アラーム
- **X-Ray**: 分散トレーシング
- **SQS**: 失敗イベント用デッドレターキュー

### 監視とアラーム

包括的な監視機能を含みます：

- API Gateway 4xx/5xx エラーとレイテンシ
- Lambda 関数エラーと実行時間
- DynamoDB スロットリングと容量メトリクス
- カスタム CloudWatch ダッシュボード

## デプロイメント

### 環境

- **開発環境**: `develop` ブランチから自動デプロイ
- **本番環境**: `main` ブランチから自動デプロイ

### CI/CD パイプライン

1. **コード品質**: フォーマット、リンティング、セキュリティスキャン
2. **テスト**: ユニット、統合、フロントエンドテスト
3. **ビルド**: Lambda 関数とフロントエンドアセット
4. **デプロイ**: SAM ベースのインフラストラクチャデプロイ
5. **スモークテスト**: 基本的なヘルスチェック

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

## イベントソーシング設計

### イベントタイプ

- `TodoCreatedV2`: メタデータ付き ToDo 作成
- `TodoUpdatedV1`: ToDo 変更
- `TodoCompletedV1`: ToDo 完了
- `TodoDeletedV1`: ToDo 削除（論理削除）

### データフロー

1. **コマンド** → CommandHandler → イベント → DynamoDB
2. **イベント** → DynamoDB Streams → EventProcessor → プロジェクション
3. **クエリ** → QueryHandler → プロジェクション → レスポンス

### 楽観的ロック

バージョンベースの楽観的ロックと自動リトライロジックで同時変更を処理します。

## セキュリティ機能

- **Passkey 認証**: WebAuthn ベースのパスワードレスログイン
- **JWT 認可**: API Gateway と Cognito の統合
- **家族ベースアクセス制御**: マルチテナントデータ分離
- **監査ログ**: GDPR 対応の完全なイベント履歴
- **セキュリティスキャン**: CI/CD での自動脆弱性チェック

## パフォーマンス目標

- **API レスポンス時間**: <200ms (p95)
- **DynamoDB 操作**: 書き込み<20ms、読み取り<10ms
- **Lambda コールドスタート**: <500ms (p99)
- **エラー率**: <0.1%
- **可用性**: 99.9%

## 設定

### 環境変数

`.env.example` を `.env.local` にコピーして設定：

```bash
# AWS設定
AWS_REGION=ap-northeast-1
AWS_PROFILE=default

# アプリケーション設定
ENVIRONMENT=dev
LOG_LEVEL=info

# フロントエンド設定
VITE_API_URL=http://localhost:8080
VITE_USER_POOL_ID=your-user-pool-id
VITE_USER_POOL_CLIENT_ID=your-user-pool-client-id
```

## ドキュメント

- [アーキテクチャ計画](docs/architecture/PLANNING.md)
- [要件定義](/.kiro/specs/family-todo-serverless/requirements.md)
- [設計書](/.kiro/specs/family-todo-serverless/design.md)
- [実装タスク](/.kiro/specs/family-todo-serverless/tasks.md)

## ライセンス

MIT License

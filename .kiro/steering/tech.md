# 技術スタック & ビルドシステム

## バックエンドスタック

- **言語**: Rust（最新安定版）
- **ランタイム**: AWS Lambda with `provided.al2` runtime
- **ビルドツール**: `cargo-lambda` for Lambda 最適化ビルド
- **アーキテクチャ**: イベントソーシング + CQRS パターン
- **データベース**: DynamoDB（シングルテーブル設計）
- **認証**: Amazon Cognito（Passkey 対応）
- **API**: AWS API Gateway（HTTP API）
- **監視**: CloudWatch + X-Ray トレーシング

## フロントエンドスタック

- **フレームワーク**: React 18 + TypeScript
- **ビルドツール**: Vite 5
- **リンティング/フォーマット**: Biome（ESLint + Prettier の代替）
- **テスト**: Vitest + Testing Library
- **認証**: AWS Amplify Auth
- **ルーティング**: React Router v6

## インフラストラクチャ

- **IaC**: AWS SAM（Serverless Application Model）
- **CI/CD**: GitHub Actions
- **ローカル開発**: Docker Compose（DynamoDB Local）
- **デプロイ**: SAM CLI（環境固有設定）

## 主要依存関係

### Rust ワークスペース依存関係

- `tokio`: フル機能付き非同期ランタイム
- `serde`: derive マクロ付きシリアライゼーション
- `aws-sdk-dynamodb`: AWS DynamoDB クライアント
- `lambda_runtime`: AWS Lambda ランタイム
- `ulid`: 辞書順ソート可能なユニーク ID
- `chrono`: serde 対応の日時処理
- `tracing`: 構造化ログと可観測性

### フロントエンド依存関係

- `@aws-amplify/auth`: Cognito 認証
- `react-router-dom`: クライアントサイドルーティング
- `@biomejs/biome`: 高速リンティング・フォーマット

## 共通コマンド

### 開発環境セットアップ

```bash
make setup                    # 完全な環境セットアップ
make install-deps            # すべての依存関係をインストール
make local-dev               # ローカルサービス開始（DynamoDB Local）
make sam-local               # APIサーバーをローカルで開始
make frontend-dev            # フロントエンド開発サーバー開始
```

### ビルド & テスト

```bash
make build                   # すべてのRustクレートをビルド
make test                    # すべてのテストを実行（Rust + フロントエンド）
make lint                    # リンティング実行（cargo fmt, clippy, biome）
make format                  # すべてのコードをフォーマット
make lambda-build            # デプロイ用Lambda関数をビルド
```

### デプロイ

```bash
make deploy-dev              # 開発環境にデプロイ
make deploy-prod             # 本番環境にデプロイ
make validate-template       # SAMテンプレートを検証
make health-check-dev        # デプロイヘルスチェック
```

### ローカル開発 URL

- フロントエンド: http://localhost:3000
- API Gateway: http://localhost:8080
- DynamoDB Local: http://localhost:8000
- DynamoDB Admin: http://localhost:8001

## ビルド設定

### Rust リリースプロファイル

- より小さなバイナリのための LTO 有効
- 最適化のための単一コードジェンユニット
- Lambda 互換性のためのパニックアボート
- デバッグシンボル削除

### Lambda アーキテクチャ

- x86_64 アーキテクチャ
- 256MB メモリ割り当て
- 30 秒タイムアウト
- X-Ray トレーシング有効
- 設定用環境変数

## 言語・コミュニケーション設定

- **チャット応答**: 日本語で回答
- **コードコメント**: 日本語
- **ドキュメント**: 日本語
- **エラーメッセージ**: 可能な限り日本語で説明

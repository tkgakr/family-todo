# 開発ガイド

## 概要

このドキュメントでは、Family Todo App の開発環境のセットアップから実装方法まで、開発に必要な情報を提供します。

## 前提条件

### 必須ソフトウェア

- **Rust** 1.75 以上
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup target add aarch64-unknown-linux-musl
  ```

- **Node.js** 20 以上
  ```bash
  # nodenv を使用している場合
  nodenv install 20.0.0
  nodenv global 20.0.0
  
  # nvm を使用している場合
  nvm install 20
  nvm use 20
  ```

- **AWS CLI**
  ```bash
  # macOS
  brew install awscli
  
  # その他のOS
  pip install awscli
  ```

- **AWS SAM CLI**
  ```bash
  # macOS
  brew install aws-sam-cli
  
  # その他のOS
  pip install aws-sam-cli
  ```

- **Docker Desktop**
  - [公式サイト](https://www.docker.com/products/docker-desktop)からダウンロード

### 推奨ツール

- **cargo-watch**: Rustファイルの変更を監視
  ```bash
  cargo install cargo-watch
  ```

- **cargo-audit**: セキュリティ監査
  ```bash
  cargo install cargo-audit
  ```

## プロジェクト構成

```
family-todo-claude/
├── backend/                # Rust Lambda 関数群
│   ├── shared/            # 共通ドメインモデル・インフラ層
│   ├── command-handler/   # 書き込み処理 Lambda
│   ├── query-handler/     # 読み取り処理 Lambda
│   ├── event-processor/   # イベント処理 Lambda
│   └── snapshot-manager/  # スナップショット管理 Lambda
├── frontend/              # React/TypeScript SPA
├── infra/                 # AWS SAM テンプレート
├── tests/                 # 統合テスト
├── docs/                  # ドキュメント
├── .github/workflows/     # CI/CDパイプライン
├── Makefile               # 統合開発コマンド
├── docker-compose.yml     # ローカル開発環境
├── env.json               # ローカル環境変数
└── .env.example           # 環境変数テンプレート
```

## 開発環境のセットアップ

### 1. リポジトリのクローン

```bash
git clone https://github.com/your-org/family-todo-claude.git
cd family-todo-claude
```

### 2. 環境変数の設定

```bash
# 環境変数テンプレートをコピー
cp .env.example .env
# 必要に応じて .env を編集

# env.json は既に設定済みのため通常は変更不要
```

### 3. 統合開発環境の起動（推奨）

```bash
# ローカル開発環境（Docker Compose）を起動
make dev-up

# 開発サーバーを起動（別ターミナル）
make dev-servers

# 開発環境の状態確認
make dev-status
```

### 4. 個別セットアップ（オプション）

#### バックエンド
```bash
cd backend
cargo build
```

#### フロントエンド
```bash
cd frontend
npm install
```

#### テスト
```bash
cd tests
npm install
```

## ローカル開発

### 推奨: 統合開発コマンド

```bash
# 開発環境管理
make dev-up           # 開発環境起動（Docker Compose）
make dev-down         # 開発環境停止
make dev-status       # サービス状態確認
make dev-servers      # 開発サーバー起動

# ビルド・デプロイ  
make build            # Lambda関数ビルド
make build-frontend   # フロントエンドビルド
make deploy-local     # ローカルAPIサーバー起動

# データベース管理
make db-setup         # DB初期化
make db-reset         # DBリセット
make db-seed          # テストデータ生成
```

### 個別起動（オプション）

#### API サーバー
```bash
# SAM でローカル API を起動
make deploy-local
# または
cd infra && sam local start-api --port 3001 --env-vars ../env.json
```

#### フロントエンド開発サーバー
```bash
cd frontend && npm run dev
```

ブラウザで `http://localhost:5173` にアクセス

#### サービス起動確認
```bash
# 利用可能なサービス
# - DynamoDB Local: http://localhost:8000
# - LocalStack: http://localhost:4566  
# - Redis: http://localhost:6379
# - API: http://localhost:3001
# - Frontend: http://localhost:5173
```

## テスト

### 推奨: 統合テストコマンド

```bash
# 全テスト実行
make test

# 個別テスト実行
make test-unit         # ユニットテスト
make test-integration  # 統合テスト
make test-e2e          # E2Eテスト

# コード品質チェック
make fmt               # フォーマット（Rust + TypeScript）
make lint              # リンター（Rust + TypeScript）
make typecheck         # 型チェック（Rust + TypeScript）
```

### 個別テスト実行

#### Rust (バックエンド)
```bash
cd backend
cargo test                    # 単体テスト
cargo test --test '*'         # 統合テスト
cargo fmt                     # フォーマット
cargo clippy -- -D warnings  # リンター
```

#### TypeScript (フロントエンド)
```bash
cd frontend
npm run test:unit      # 単体テスト
npm run test:e2e       # E2Eテスト
npm run format         # フォーマット
npm run lint           # リンター
npm run typecheck      # 型チェック
```

### テストスイート（準備中）

統合テスト・E2Eテスト・負荷テストの実装予定：

```bash
cd tests/load
k6 run k6-script.js
```

## コード品質

### リンター・フォーマッター

#### Rust
```bash
cd backend

# フォーマット
cargo fmt

# リンター
cargo clippy -- -D warnings

# セキュリティ監査
cargo audit
```

#### TypeScript
```bash
cd frontend

# Biome によるフォーマット・リント
npm run format
npm run lint
```

### 推奨 VS Code 拡張機能

```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "ms-vscode.vscode-typescript-next",
    "biomejs.biome",
    "aws-amplify.aws-amplify-vscode",
    "ms-vscode.vscode-json"
  ]
}
```

## アーキテクチャ理解

### イベントソーシング

すべての状態変更は「イベント」として記録されます：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum TodoEvent {
    TodoCreatedV2 { /* ... */ },
    TodoUpdatedV1 { /* ... */ },
    TodoCompletedV1 { /* ... */ },
    TodoDeletedV1 { /* ... */ },
}
```

### CQRS 分離

- **Command Handler**: 書き込み操作（イベント生成）
- **Query Handler**: 読み取り操作（プロジェクション参照）
- **Event Processor**: イベントからプロジェクション更新

### DynamoDB Single Table Design

```
PK                     | SK                     | 用途
-----------------------|------------------------|------------------
FAMILY#{family_id}     | EVENT#{event_id}       | イベント格納
FAMILY#{family_id}     | TODO#CURRENT#{todo_id} | 現在のTodo状態
FAMILY#{family_id}     | FAMILY#META           | 家族メタデータ
```

## 実装ガイドライン

### 新機能の追加手順

1. **ドメインモデルの更新**
   ```bash
   # 新しいイベントタイプを追加
   vim backend/shared/src/domain/events.rs
   ```

2. **コマンドハンドラーの実装**
   ```bash
   vim backend/command-handler/src/handlers.rs
   ```

3. **クエリハンドラーの実装**
   ```bash
   vim backend/query-handler/src/handlers.rs
   ```

4. **イベントプロセッサーの更新**
   ```bash
   vim backend/event-processor/src/processor.rs
   ```

5. **フロントエンドの実装**
   ```bash
   vim frontend/src/api/todos.ts
   vim frontend/src/components/
   ```

6. **テストの追加**

### エラーハンドリング

```rust
// ドメインエラーの定義
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Invalid todo ID: {0}")]
    InvalidTodoId(String),
    
    #[error("Todo not found")]
    TodoNotFound,
    
    #[error("Validation error: {0}")]
    ValidationError(String),
}

// レスポンスでのエラーハンドリング
match result {
    Ok(todo) => Ok(ApiResponse::success(todo)),
    Err(DomainError::TodoNotFound) => {
        Ok(ApiResponse::error(404, "Todo not found"))
    },
    Err(e) => {
        tracing::error!("Unexpected error: {}", e);
        Ok(ApiResponse::error(500, "Internal server error"))
    }
}
```

### ログとメトリクス

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(client))]
pub async fn create_todo(
    client: &aws_sdk_dynamodb::Client,
    command: CreateTodoCommand,
) -> Result<Todo, DomainError> {
    info!("Creating todo with title: {}", command.title);
    
    // 実装...
    
    info!(todo_id = %todo.id, "Todo created successfully");
    Ok(todo)
}
```

### パフォーマンスの考慮事項

1. **Lambda コールドスタート対策**
   - 静的変数でクライアント初期化
   - ARM64 アーキテクチャ使用
   - 適切なメモリ設定 (512MB)

2. **DynamoDB 最適化**
   - Single Table Design
   - 効率的なクエリパターン
   - GSI の活用

3. **フロントエンド最適化**
   - React.memo の活用
   - useMemo/useCallback の適切な使用
   - Bundle サイズの管理

## デバッグ

### Lambda ログの確認

```bash
# CloudWatch Logs
sam logs -n TodoCommandHandler --start-time 2024-01-15T10:00:00 --tail

# ローカルログ
sam local start-api --debug
```

### DynamoDB データの確認

```bash
# ローカル DynamoDB
aws dynamodb scan \
  --table-name MainTable \
  --endpoint-url http://localhost:8000

# AWS DynamoDB
aws dynamodb scan --table-name {stack-name}-MainTable
```

### フロントエンド デバッグ

```javascript
// Redux DevTools の使用
// Browser DevTools の Network タブ
// React DevTools の使用
```

## 本番環境との相違点

### ローカル開発環境
- DynamoDB Local 使用
- 認証機能無効化（ヘッダーベース）
- ホットリロード有効

### 本番環境
- AWS DynamoDB
- Cognito 認証
- CloudWatch 監視

## トラブルシューティング

### よくある問題

1. **Rust コンパイルエラー**
   ```bash
   # キャッシュクリア
   cargo clean
   cargo build
   ```

2. **DynamoDB アクセスエラー**
   ```bash
   # ローカルDynamoDBが起動しているか確認
   curl http://localhost:8000
   ```

3. **CORS エラー**
   ```bash
   # API Gateway の CORS 設定確認
   # フロントエンドのプロキシ設定確認
   ```

4. **Lambda タイムアウト**
   - 環境変数 `RUST_LOG=debug` でログ確認
   - 処理時間の測定

## パフォーマンス計測

### ベンチマークの実行

```bash
# Rust ベンチマーク
cd backend
cargo bench

# フロントエンド パフォーマンス
cd frontend
npm run build:analyze
```

### メトリクス監視

- CloudWatch メトリクス
- X-Ray トレーシング
- カスタムメトリクス

## 貢献ガイドライン

1. **ブランチ戦略**: GitHub Flow
2. **コミットメッセージ**: [Conventional Commits](https://www.conventionalcommits.org/)
3. **プルリクエスト**: テンプレート使用
4. **コードレビュー**: 最低1人の承認必須

## 参考資料

- [Rust Book](https://doc.rust-lang.org/book/)
- [AWS SAM ドキュメント](https://docs.aws.amazon.com/serverless-application-model/)
- [React ドキュメント](https://react.dev/)
- [DynamoDB ベストプラクティス](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/best-practices.html)
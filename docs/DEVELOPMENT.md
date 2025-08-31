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
│   └── event-processor/   # イベント処理 Lambda
├── frontend/              # React/TypeScript SPA
├── infra/                 # AWS SAM テンプレート
├── tests/                 # 統合テスト
├── docs/                  # ドキュメント
└── env.json              # ローカル環境変数
```

## 開発環境のセットアップ

### 1. リポジトリのクローン

```bash
git clone https://github.com/your-org/family-todo-claude.git
cd family-todo-claude
```

### 2. 環境変数の設定

```bash
cp env.json.example env.json
# 必要に応じて環境変数を調整
```

### 3. 依存関係のインストール

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

### SAM Local での API サーバー起動

```bash
# ビルド
sam build --use-container

# ローカルAPI起動
sam local start-api \
  --warm-containers EAGER \
  --port 3001 \
  --env-vars env.json
```

### フロントエンド開発サーバー起動

```bash
cd frontend
npm run dev
```

ブラウザで `http://localhost:3000` にアクセス

### DynamoDB Local の起動

```bash
docker run -p 8000:8000 amazon/dynamodb-local:latest \
  -jar DynamoDBLocal.jar -sharedDb -inMemory
```

### 統合開発環境

すべてを一度に起動する場合：

```bash
# 並行実行でバックエンドとフロントエンドを起動
npm run dev
```

## テスト

### 単体テスト

#### Rust (バックエンド)
```bash
cd backend
cargo test
```

#### TypeScript (フロントエンド)
```bash
cd frontend
npm test
```

### 統合テスト

```bash
cd tests
npm run test:integration
```

### 負荷テスト

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
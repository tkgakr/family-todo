# プロジェクト構造 & 組織化

## ルートディレクトリレイアウト

```text
├── crates/                 # Rustワークスペース - すべてのバックエンドコード
├── frontend/              # Reactフロントエンドアプリケーション
├── .github/workflows/     # CI/CDパイプライン定義
├── docs/                  # プロジェクトドキュメント
├── scripts/               # 開発・デプロイスクリプト
├── .kiro/                 # Kiro AIアシスタント設定
├── template.yaml          # SAMインフラストラクチャテンプレート
├── docker-compose.yml     # ローカル開発サービス
├── Makefile              # 開発ワークフローコマンド
└── README.md             # プロジェクト概要とセットアップ
```

## Rust ワークスペース構造

バックエンドは関心の分離を明確にしたドメイン駆動設計に従います：

### コアクレート

- **`crates/domain/`**: 純粋なドメインロジック、イベント、エンティティ

  - シリアライゼーション以外の外部依存関係なし
  - `Todo`、`TodoId`、`TodoEvent`型を含む
  - ビジネスルールとドメイン検証

- **`crates/shared/`**: すべてのサービス間で共通のユーティリティ
  - 認証ヘルパー
  - 設定管理
  - トレーシングとログ設定
  - 横断的関心事

### インフラストラクチャ層

- **`crates/infrastructure/`**: データアクセスと外部統合
  - DynamoDB リポジトリとクライアント
  - イベントストア実装
  - AWS サービス統合

### アプリケーションサービス（Lambda 関数）

- **`crates/command-handler/`**: 書き込み操作（コマンド）

  - todo 作成、更新、完了、削除を処理
  - コマンドを検証してイベントを生成
  - ルート: `/commands/*`

- **`crates/query-handler/`**: 読み取り操作（クエリ）

  - todo リストと個別 todo 詳細を提供
  - 最適化された読み取りモデルとプロジェクション
  - ルート: `/queries/*`

- **`crates/event-processor/`**: イベントストリーム処理

  - DynamoDB Stream イベントを処理
  - 読み取りモデルとプロジェクションを更新
  - DynamoDB Streams によってトリガー

- **`crates/snapshot-manager/`**: スナップショット作成と管理
  - パフォーマンス向上のための定期スナップショット作成
  - CloudWatch Events によるスケジュール実行
  - データ最適化のため日次実行

## フロントエンド構造

```text
frontend/
├── src/
│   ├── components/        # 再利用可能なUIコンポーネント
│   ├── pages/            # ルートレベルコンポーネント
│   ├── hooks/            # カスタムReactフック
│   ├── services/         # APIクライアントとビジネスロジック
│   ├── types/            # TypeScript型定義
│   ├── utils/            # ヘルパー関数
│   └── test/             # テストユーティリティとセットアップ
├── public/               # 静的アセット
└── dist/                 # ビルド出力（生成）
```

## 設定ファイル

### ビルド & 開発

- `Cargo.toml`: Rust ワークスペース設定
- `package.json`: フロントエンド依存関係とスクリプト
- `vite.config.ts`: フロントエンドビルド設定
- `biome.json`: コードフォーマットとリンティングルール
- `samconfig.toml`: SAM デプロイ設定

### インフラストラクチャ

- `template.yaml`: 完全な AWS インフラストラクチャ定義
- `local-env.json`: ローカル開発用環境変数
- `.env.example`: 環境設定テンプレート

### CI/CD

- `.github/workflows/ci-cd.yml`: 完全なデプロイパイプライン
- `scripts/setup-dev.sh`: 開発環境セットアップ
- `scripts/health-check.sh`: デプロイ検証

## 命名規則

### Rust コード

- **クレート**: kebab-case（`command-handler`、`event-processor`）
- **モジュール**: snake_case（`todo.rs`、`events.rs`）
- **型**: PascalCase（`TodoId`、`TodoEvent`）
- **関数**: snake_case（`create_todo`、`handle_command`）
- **定数**: SCREAMING_SNAKE_CASE（`MAX_RETRY_ATTEMPTS`）

### フロントエンドコード

- **コンポーネント**: PascalCase（`TodoList.tsx`、`CreateTodo.tsx`）
- **フック**: camelCase で`use`プレフィックス（`useTodos`、`useAuth`）
- **サービス**: camelCase（`todoService`、`authService`）
- **型**: PascalCase（`Todo`、`CreateTodoRequest`）

### インフラストラクチャ

- **リソース**: PascalCase（`FamilyTodoTable`、`CommandHandlerFunction`）
- **パラメータ**: PascalCase（`Environment`、`FamilyTodoTableName`）
- **出力**: PascalCase（`ApiGatewayUrl`、`UserPoolId`）

## ファイル組織原則

### ドメイン駆動設計

- ドメインロジックは`crates/domain/`に分離
- インフラストラクチャ関心事をビジネスロジックから分離
- 読み取りと書き込み操作間の明確な境界

### 単一責任

- 各クレートは焦点を絞った目的を持つ
- Lambda 関数は特定の操作タイプを処理
- コマンドとクエリの明確な分離

### 依存関係の方向

- ドメイン層は外部依存関係なし
- インフラストラクチャはドメインに依存
- アプリケーションサービスは両方に依存
- フロントエンドは API コントラクトのみに依存

### イベントソーシング構造

- イベントは不変でバージョン管理（`TodoCreatedV2`）
- DynamoDB のストリーム付きイベントストア
- 非同期でプロジェクション更新
- パフォーマンス最適化のためのスナップショット

## 言語・コミュニケーション設定

- **チャット応答**: 日本語で回答
- **コードコメント**: 日本語
- **ドキュメント**: 日本語
- **変数名・関数名**: 英語（技術的慣例に従う）
- **エラーメッセージ**: 可能な限り日本語で説明

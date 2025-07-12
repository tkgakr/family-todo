# CLAUDE.md

このファイルは、このリポジトリでコードを操作する際の Claude Code (claude.ai/code) 向けのガイダンスを提供します。

## プロジェクト概要

Rustバックエンド（AWS Lambda）と予定されているReact/TypeScriptフロントエンドで構築された家族用TODO共有アプリケーションです。アーキテクチャはイベントソーシングとCQRSパターンを使用したAWSサーバーレスサービスを活用しています。プロジェクトは現在初期開発段階で、バックエンドインフラストラクチャの基盤が完成しています。

## 開発コマンド

### バックエンド（Rust Lambda）
```bash
# テスト実行
cd backend && cargo test

# clippyリンター実行
cd backend && cargo clippy -- -D warnings

# 開発用ビルド
cd backend && cargo build

# 統合テスト実行
cd backend && cargo test --test api_integration_tests
```

### インフラストラクチャ（AWS SAM）
```bash
# SAMテンプレートビルド
cd infra && sam build

# AWSへデプロイ
cd infra && sam deploy

# ローカルAPI開発
cd infra && sam local start-api

# イベントファイルでLambda関数をローカルテスト
cd infra && sam local invoke todoHandler -e events/event.json
```

## アーキテクチャ

### 現在の状態
- **バックエンド**: 基本的なHTTP処理を行う単一のRust Lambda関数（`todoHandler`）
- **インフラストラクチャ**: Lambda、API Gateway、IAMロールを定義するAWS SAMテンプレート
- **構造**: `backend/`（Rust）と`infra/`（SAMテンプレート）を含むモノレポ

### 計画されたアーキテクチャ（planning.mdに基づく）
プロジェクトはイベントソーシングとCQRSパターンを中心に設計されています：

- **イベントストア**: ULID識別子で不変イベントを保存するDynamoDBテーブル
- **プロジェクションストア**: 読み取り最適化されたビュー用DynamoDBテーブル
- **Lambda関数**:
  - `TodoCommandHandler`: 書き込み操作（イベント作成）
  - `TodoEventProcessor`: DynamoDB Streamsを処理してプロジェクションを更新
  - `TodoQueryHandler`: プロジェクションからの読み取り操作
- **識別子**: 自然な時系列ソートと分散ID生成のためのULID形式

### 主要な依存関係
- `lambda_http`: LambdaでのHTTP処理
- `serde`/`serde_json`: JSONシリアライゼーション
- `tokio`: 非同期ランタイム
- 予定: `ulid`, `aws-sdk-dynamodb`, `chrono`

## ファイル構造
```
/
├── backend/           # Rust Lambda関数
│   ├── src/
│   │   ├── main.rs           # Lambdaエントリーポイント
│   │   └── http_handler.rs   # HTTPリクエスト処理
│   ├── tests/
│   │   └── api_integration_tests.rs
│   └── Cargo.toml
├── infra/             # AWS SAMインフラストラクチャ
│   ├── template.yaml         # SAMリソース定義
│   ├── samconfig.toml       # SAM設定
│   └── events/
│       └── event.json       # ローカル開発用テストイベント
├── planning.md        # 詳細な技術アーキテクチャ計画
└── README.md
```

## 主要な実装ノート

### ULID使用法
プロジェクトではUUIDの代わりにULID（26文字Base32）を使用予定：
- DynamoDBでの自然な時系列ソート
- パーティション/ソートキーとしてより効率的な保存
- デバッグ機能（タイムスタンプ抽出）

### DynamoDB設計
イベントソーシングパターン：
- イベントストア: `PK: FAMILY#{familyId}`, `SK: EVENT#{ulid}`
- プロジェクションストア: `PK: FAMILY#{familyId}`, `SK: TODO#{ulid}`
- DynamoDB Streamsがイベント処理をトリガー

### テスト
- `backend/src/`モジュール内のユニットテスト（`#[cfg(test)]`付き）
- `backend/tests/`内の統合テスト
- `infra/events/`のイベントファイルを使用したローカルSAMテスト

## 開発ワークフロー

1. バックエンド変更: `cargo test`でテスト後、`sam build`と`sam local start-api`実行
2. インフラストラクチャ変更: `template.yaml`更新後、`sam build && sam deploy`実行
3. ローカル開発: APIテスト用に`sam local start-api`使用
4. 現在のLambdaは任意のHTTPリクエストに対してシンプルな挨拶メッセージで応答

## 設定

- SAMスタック名: `family-todo-app`
- AWSリージョン: `ap-northeast-1` 
- Lambdaランタイム: `provided.al2023`（Rustカスタムランタイム）
- アーキテクチャ: コスト効率のためのARM64
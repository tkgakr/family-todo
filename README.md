# 家族用TODOアプリ

このリポジトリは、Rust + React/TypeScriptで実装する家族用TODO共有アプリのコードベースです。AWSのサーバーレスアーキテクチャを活用して、低コスト・高セキュリティでデプロイします。

**現在の開発状況**: バックエンドとインフラストラクチャの基盤構築中

## プロジェクト構成

```
/
├── infra/              # SAM テンプレート
│   ├── template.yaml   # SAM リソース定義
│   └── samconfig.toml  # SAM設定ファイル
├── backend/            # Rust Lambda関数
│   ├── src/            # ソースコード
│   │   ├── main.rs     # エントリーポイント
│   │   └── http_handler.rs # HTTPリクエスト処理
│   ├── tests/         # テストコード
│   │   └── api_integration_tests.rs # API統合テスト
│   └── Cargo.toml     # Rust依存関係
├── frontend/           # React/TS (未実装)
│   └── (準備中)
└── docs/               # ADR, API Spec等 (準備中)
```

## 使用技術

- **フロントエンド**: React, TypeScript, Vite
- **バックエンド**: Rust, Lambda HTTP, cargo-lambda
- **インフラ**: AWS (Lambda, API Gateway, DynamoDB, Cognito, S3, CloudFront)
- **IaC**: AWS SAM (将来的にTerraform/CDKへ移行予定)
- **CI/CD**: GitHub Actions

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
```

### SAMローカル実行

```bash
cd infra
sam build
sam local start-api
```

### 今後導入予定のツール
- **LocalStack**: Cognito / DynamoDB エミュレーション
- **mkcert**: `https://localhost` でパスキー動作確認
- **DynamoDB Local**: オフラインDBテスト
- **VS Code Dev Container**: 開発環境の標準化

## ライセンス

(準備中)

## プロジェクト進捗

- [x] プロジェクト初期設定
- [x] バックエンド基盤構築（Rust Lambda）
- [x] インフラ基盤構築（AWS SAM）
- [x] バックエンドテスト実装
- [ ] DynamoDB統合
- [ ] フロントエンド開発開始
- [ ] 認証機能
- [ ] CI/CD設定

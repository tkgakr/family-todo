# 家族用TODOアプリ

このリポジトリは、Rust + React/TypeScriptで実装する家族用TODO共有アプリのコードベースです。AWSのサーバーレスアーキテクチャを活用して、低コスト・高セキュリティでデプロイします。

## プロジェクト構成

```
/
├── infra/              # SAM / Terraform テンプレート
│   ├── template.yaml   # SAM main
│   └── samconfig.toml
├── backend/            # Rust (axum) Lambda
│   ├── src/
│   └── Cargo.toml
├── frontend/           # React/TS (Vite)
│   ├── src/
│   └── package.json
├── .github/
│   └── workflows/
│       ├── backend.yml
│       └── frontend.yml
└── docs/               # ADR, API Spec, etc.
```

## 使用技術

- **フロントエンド**: React, TypeScript, Vite
- **バックエンド**: Rust, axum, cargo-lambda
- **インフラ**: AWS (Lambda, API Gateway, DynamoDB, Cognito, S3, CloudFront)
- **IaC**: AWS SAM (将来的にTerraform/CDKへ移行予定)
- **CI/CD**: GitHub Actions

## 開発環境のセットアップ

(準備中)

## ローカル開発・テスト

- **SAM CLI**: Lambda + API GW模擬
- **LocalStack**: Cognito / DynamoDB エミュレーション
- **mkcert**: `https://localhost` でパスキー動作確認
- **DynamoDB Local**: オフラインDBテスト
- **VS Code Dev Container**: 開発環境

## ライセンス

(準備中)

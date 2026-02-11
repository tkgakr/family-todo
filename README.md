# Family Todo

シンプルで使いやすい家族向けToDo共有アプリケーション。

## アーキテクチャ

- **Frontend**: React + TypeScript (Vite)
- **Backend**: Rust (AWS Lambda)
- **Database**: Amazon DynamoDB
- **Auth**: Amazon Cognito
- **Hosting**: S3 + CloudFront
- **IaC**: AWS SAM

## 開発

```bash
# バックエンドビルド
cd backend && cargo lambda build --release --arm64

# フロントエンドビルド
cd frontend && bun install && bun run build

# SAM デプロイ
sam build && sam deploy
```
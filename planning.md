
# 🗺️ ToDo アプリ ― AWS サーバーレス版 プランニングドキュメント
**目的**: Rust + React/TypeScript で実装する家族用 ToDo 共有アプリを、低コスト・高セキュリティで AWS サーバーレスにデプロイする。ここでは AI エージェントが実装・IaC 自動生成・CI/CD 設計などを行う際の“単一ソース・オブ・トゥルース”として参照できる **構成・命名・手順**をまとめる。

---

## 1. システム全体概要

| 層 | サービス | 主な役割 |
|----|----------|----------|
| **フロント** | **S3** 静的ウェブホスティング／**CloudFront** CDN | React SPA (Vite + TS) 配信 |
| **API** | **API Gateway (HTTP)** | REST エンドポイント・CORS・Cognito JWT 検証 |
|  | **AWS Lambda (Rust)** | ビジネスロジック (`axum` + `cargo-lambda`) |
| **認証** | **Amazon Cognito** (ユーザープール) | Passkey (WebAuthn) + リフレッシュトークン |
| **DB** | **Amazon DynamoDB** | `families` テーブル（ToDo + 履歴） |
| **監視** | **CloudWatch Logs / Metrics** | Lambda 実行ログ・アラーム |
| **CI/CD** | **GitHub Actions + AWS CLI / SAM** | ビルド・テスト・デプロイ自動化 |
| **IaC** | **AWS SAM** (初期) → 将来 **Terraform/CDK** | インフラ定義・再現 |

---

## 2. リポジトリ構成（モノレポ）

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

---

## 3. AWS リソース詳細

| 論理名 | 種別 | 設定の要点 |
|--------|------|-----------|
| `todoUserPool` | Cognito User Pool | Passkey 有効（Authenticator Attachment=platform＋cross‑platform）。RefreshTokenValidity=90 days |
| `todoApi` | API Gateway HTTP API | Lambda Proxy + Cognito Authorizer (JWT) |
| `todoHandler` | Lambda | メモリ 256 MB, 30 sec, **Rust** target `aarch64-unknown-linux-musl` |
| `todoTable` | DynamoDB | PK=`FamilyID` ( S), SK=`ItemID` ( S)；GSI1=`FamilyID#History` sort=`Timestamp` |
| `todoBucket` | S3 | 静的サイト・サーバーアクセスログは別バケット |
| `todoCdn` | CloudFront | オリジン=S3、OAC でバケットアクセス |
| `todoLogs` | CloudWatch Log Group | Retention=1 year (コスト削減) |

### IAM ロール / ポリシー

| ロール | 最小権限ポリシー |
|--------|-----------------|
| `TodoLambdaRole` | `dynamodb:GetItem/PutItem/UpdateItem`, `logs:CreateLog*`, `cognito-idp:GetUser` |
| `GithubDeployRole` | `lambda:UpdateFunctionCode`, `cloudfront:CreateInvalidation`, `s3:Sync*`, `cloudformation:*` (初期) |

---

## 4. デプロイフロー（GitHub Actions）

1. **Backend ワークフロー** (`backend.yml`)

```yaml
steps:
  - uses: actions/checkout@v4
  - uses: actions/setup-node@v4 # SAMはNode.jsに依存
  - uses: aws-actions/setup-sam@v2
  - uses: dtolnay/rust-toolchain@stable
  # 'sam build' はRust (cargo-lambda) もサポート
  - run: sam build --use-container
  - uses: aws-actions/configure-aws-credentials@v4
    with:
      role-to-assume: ${{ secrets.AWS_DEPLOY_ROLE }}
  # 'sam deploy' でコードとインフラを同時にデプロイ
  - run: sam deploy --no-confirm-changeset --no-fail-on-empty-changeset
```

2. **Frontend ワークフロー** (`frontend.yml`)

```yaml
steps:
  - uses: actions/checkout@v4
  - uses: actions/setup-node@v4
  - run: npm ci && npm run build
  - uses: aws-actions/configure-aws-credentials@v4 ...
  - run: aws s3 sync frontend/dist s3://$S3_BUCKET --delete
  - run: aws cloudfront create-invalidation --distribution-id $CF_ID --paths "/*"
```

---

## 5. ローカル開発・テスト

| ツール | 用途 |
|--------|------|
| **SAM CLI** `sam local start-api` | Lambda + API GW 模擬。`--warm-containers EAGER` で高速化 |
| **LocalStack** (optional) | Cognito / DynamoDB エミュレーション |
| **mkcert** | `https://localhost` でパスキー動作確認 |
| **DynamoDB Local** | オフライン DB テスト（LocalStack が重い場合） |
| **VS Code Dev Container** | rustup + Node + AWS CLI プリインストール環境 |

---

## 6. 開発マイルストーン

| Sprint | ゴール | 完了条件 (DoD) |
|--------|--------|----------------|
| **0** | 基盤セットアップ | IaC で空リソース作成・CI 雛形稼働 |
| **1** | 認証 | Passkey 登録・ログイン → JWT 取得確認 |
| **2** | CRUD API | `POST /todos`, `PATCH /todos/{id}` 完了 |
| **3** | 履歴ログ | 操作履歴を DynamoDB GSI1 へ書込／取得 |
| **4** | SPA UI | 家族切り替え・ToDo リスト／履歴表示 |
| **5** | QA & Cost Tune | `ab` / `hey` で 95p レイテンシ < 200 ms；AWS 月額≦$0.5 |

---

## 7. コスト試算（月間 5 家族 × 1 000 req）

| サービス | 無料枠消費 | 課金予測 |
|----------|-----------|---------|
| Lambda | 1 000 req ≈ 0.3 ms・128 MB | **$0** |
| API GW | 1 000 × $1e‑6 | **$0.001** |
| DynamoDB | RCUs 3・WCUs 1 未満 | **$0** |
| Cognito | < 10 MAU | **$0** |
| S3 / CF | 転送 100 MB 未満 | **$0** |

> **総額 ≒ 0 〜 0.01 USD／月**（全て無料枠想定）。

---

## 8. セキュリティ & 運用ポリシー

- **TLS**: ACM + CloudFront, 強制 TLS1.2+
- **CSP**: `default-src 'self'; frame-ancestors 'none';`
- **バックアップ**: DynamoDB PITR 有効、7 日間保持
- **監視**: CloudWatch Metric Alarm (`5xx` > 1 / 5 min) → SNS → Slack
- **キー管理**: Cognito secret hash/use SRP；S3 バケットは SSE‑S3

---

## 9. 共有コンテキスト用 TL;DR

> **Rust Lambda × Cognito Passkey × DynamoDB**。  
> IaC=SAM、CI=GitHub Actions。月額ほぼゼロ。  
> ローカル検証は **SAM CLI** + **LocalStack** で本番互換。

AI エージェントはこのドキュメントを読み取り、

- IaC テンプレート生成
- GitHub Actions ワークフロー自動作成
- API スキーマ定義（OpenAPI 3.1）
- 監視 & アラームの CloudWatch Rule 生成

を順次実施してください。

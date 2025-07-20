# GitHub Actions AWS認証設定手順

このドキュメントでは、GitHub ActionsでAWS認証するための設定手順を説明します。

## 参考ドキュメント 📚

### AWS公式ドキュメント
- [GitHub Actions で OpenID Connect を使用して AWS での認証を設定する](https://docs.aws.amazon.com/ja_jp/IAM/latest/UserGuide/id_roles_providers_create_oidc_verify-thumbprint.html)
- [IAM ロールの作成](https://docs.aws.amazon.com/ja_jp/IAM/latest/UserGuide/id_roles_create.html)
- [IAM ポリシーの作成](https://docs.aws.amazon.com/ja_jp/IAM/latest/UserGuide/access_policies_create.html)
- [AWS Serverless Application Model (SAM)](https://docs.aws.amazon.com/ja_jp/serverless-application-model/)

### GitHub公式ドキュメント
- [GitHub Actions と AWS の認証](https://docs.github.com/ja/actions/deployment/security-hardening-your-deployments/configuring-openid-connect-in-amazon-web-services)
- [暗号化されたシークレット](https://docs.github.com/ja/actions/security-guides/encrypted-secrets)
- [ワークフローの構文](https://docs.github.com/ja/actions/using-workflows/workflow-syntax-for-github-actions)

### aws-actions公式リポジトリ
- [aws-actions/configure-aws-credentials](https://github.com/aws-actions/configure-aws-credentials)
- [aws-actions/setup-sam](https://github.com/aws-actions/setup-sam)

## 作業概要
1. **AWS側設定**: OIDCプロバイダーとIAMロールの作成
2. **GitHub側設定**: Repository Secretsの設定
3. **設定確認**: 動作テスト

---

## ステップ1: AWS側設定 🔧

### 1-1. AWSアカウント情報確認

#### 方法1: AWS CLI使用（ローカルで認証済みの場合）
```bash
# AWSアカウントIDを確認
aws sts get-caller-identity

# GitHubリポジトリ名を確認
git remote get-url origin
```

#### 方法2: AWSコンソールで確認（ローカル未認証の場合）
**AWSコンソール**にSSO等でログインして以下を確認：

1. **アカウントID確認**:
   - コンソール右上のアカウント名をクリック
   - アカウントIDが12桁の数字で表示される（例：123456789012）

2. **GitHubユーザー名確認**:
   ```bash
   # GitHubリポジトリ名を確認
   git remote get-url origin
   # 出力例: https://github.com/YOUR_GITHUB_USERNAME/family-todo.git
   ```

3. **必要な情報をメモ**:
   - `YOUR_ACCOUNT_ID`: 123456789012
   - `YOUR_GITHUB_USERNAME`: あなたのGitHubユーザー名

### 1-2. GitHub OIDCプロバイダー作成 (AWSコンソール)

**AWSコンソール** → **IAM** → **IDプロバイダー** → **プロバイダーを追加**

- **プロバイダーのタイプ**: OpenID Connect
- **プロバイダーのURL**: `https://token.actions.githubusercontent.com`
- **対象者**: `sts.amazonaws.com`

📖 **参考**: [OpenID Connect プロバイダーを作成する](https://docs.aws.amazon.com/ja_jp/IAM/latest/UserGuide/id_roles_providers_create_oidc.html)

### 1-3. カスタムポリシーの作成 (AWSコンソール)

**AWSコンソール** → **IAM** → **ポリシー** → **ポリシーを作成**

📖 **参考**: [IAM ポリシーの作成](https://docs.aws.amazon.com/ja_jp/IAM/latest/UserGuide/access_policies_create-console.html)

**手順**:
1. 「JSON」タブを選択
2. 既存のJSONを削除して、以下のJSONを貼り付け
3. 「次へ」をクリック
4. **ポリシー名**: `GitHubActionsGeneralDeployPolicy`
5. **説明**: `General GitHub Actions deployment policy for serverless apps`
6. 「ポリシーの作成」をクリック

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "CloudFormationAccess",
      "Effect": "Allow",
      "Action": [
        "cloudformation:CreateStack",
        "cloudformation:UpdateStack",
        "cloudformation:DeleteStack",
        "cloudformation:DescribeStacks",
        "cloudformation:DescribeStackEvents",
        "cloudformation:DescribeStackResources",
        "cloudformation:GetTemplate",
        "cloudformation:ValidateTemplate",
        "cloudformation:CreateChangeSet",
        "cloudformation:DescribeChangeSet",
        "cloudformation:ExecuteChangeSet",
        "cloudformation:DeleteChangeSet",
        "cloudformation:ListChangeSets"
      ],
      "Resource": [
        "arn:aws:cloudformation:ap-northeast-1:*:stack/*"
      ]
    },
    {
      "Sid": "LambdaAccess",
      "Effect": "Allow",
      "Action": [
        "lambda:CreateFunction",
        "lambda:UpdateFunctionCode",
        "lambda:UpdateFunctionConfiguration",
        "lambda:DeleteFunction",
        "lambda:GetFunction",
        "lambda:ListTags",
        "lambda:TagResource",
        "lambda:UntagResource",
        "lambda:AddPermission",
        "lambda:RemovePermission"
      ],
      "Resource": [
        "arn:aws:lambda:ap-northeast-1:*:function:*"
      ]
    },
    {
      "Sid": "IAMAccess",
      "Effect": "Allow",
      "Action": [
        "iam:CreateRole",
        "iam:UpdateRole",
        "iam:DeleteRole",
        "iam:GetRole",
        "iam:PassRole",
        "iam:AttachRolePolicy",
        "iam:DetachRolePolicy",
        "iam:PutRolePolicy",
        "iam:DeleteRolePolicy",
        "iam:GetRolePolicy"
      ],
      "Resource": [
        "arn:aws:iam::*:role/*-lambda-role"
      ]
    },
    {
      "Sid": "APIGatewayAccess",
      "Effect": "Allow",
      "Action": [
        "apigateway:GET",
        "apigateway:POST",
        "apigateway:PUT",
        "apigateway:DELETE",
        "apigateway:PATCH"
      ],
      "Resource": [
        "arn:aws:apigateway:ap-northeast-1::/restapis",
        "arn:aws:apigateway:ap-northeast-1::/restapis/*"
      ]
    },
    {
      "Sid": "CloudWatchLogsAccess",
      "Effect": "Allow",
      "Action": [
        "logs:CreateLogGroup",
        "logs:DeleteLogGroup",
        "logs:DescribeLogGroups",
        "logs:PutRetentionPolicy"
      ],
      "Resource": [
        "arn:aws:logs:ap-northeast-1:*:log-group:/aws/lambda/*"
      ]
    },
    {
      "Sid": "S3BackendAccess",
      "Effect": "Allow",
      "Action": [
        "s3:GetObject",
        "s3:PutObject"
      ],
      "Resource": [
        "arn:aws:s3:::aws-sam-cli-managed-default-samclisourcebucket-*/*"
      ]
    },
    {
      "Sid": "S3FrontendAccess",
      "Effect": "Allow",
      "Action": [
        "s3:GetObject",
        "s3:PutObject",
        "s3:DeleteObject",
        "s3:ListBucket"
      ],
      "Resource": [
        "arn:aws:s3:::*",
        "arn:aws:s3:::*/*"
      ]
    },
    {
      "Sid": "CloudFrontAccess",
      "Effect": "Allow",
      "Action": [
        "cloudfront:CreateInvalidation"
      ],
      "Resource": [
        "arn:aws:cloudfront::*:distribution/*"
      ]
    }
  ]
}
```

**📝 汎用ポリシーの設定内容**:

このポリシーは複数のプロジェクト・リポジトリで使用できるよう、以下のように汎用化されています：

- **CloudFormation**: 全てのスタックに対する権限
- **Lambda**: 全ての関数に対する権限  
- **IAM**: `*-lambda-role`パターンのロールに対する権限
- **CloudWatch Logs**: 全てのLambdaログに対する権限
- **S3**: 全てのバケットに対する権限
- **API Gateway**: 全てのAPIに対する権限
- **CloudFront**: 全てのディストリビューションに対する権限

> ⚠️ **セキュリティ考慮**: より厳密な制限が必要な場合は、プロジェクト固有のリソース名パターンを使用してください

> 💡 **使用例**: このポリシーにより `family-todo` 以外のサーバーレスプロジェクトでも同じロールを使用可能

### 1-4. IAMロール作成 (AWSコンソール)

**AWSコンソール** → **IAM** → **ロール** → **ロールを作成**

📖 **参考**: [ウェブアイデンティティ用の IAM ロールを作成する](https://docs.aws.amazon.com/ja_jp/IAM/latest/UserGuide/id_roles_create_for-idp_oidc.html)

#### a) 信頼関係の設定
**信頼されたエンティティタイプ**: ウェブアイデンティティ

**コンソール設定項目**:
1. **アイデンティティプロバイダー**: `token.actions.githubusercontent.com`
2. **Audience**: `sts.amazonaws.com`
3. **GitHub組織またはGitHubユーザー**: `YOUR_GITHUB_USERNAME`
4. **GitHubリポジトリ**: `family-todo`
5. **GitHubブランチ**: `main`

> ⚠️ **注意**: ステップ1-2でOIDCプロバイダーを作成済みの場合のみ、ドロップダウンに `token.actions.githubusercontent.com` が表示されます

**信頼ポリシー**:
```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Federated": "arn:aws:iam::YOUR_ACCOUNT_ID:oidc-provider/token.actions.githubusercontent.com"
      },
      "Action": "sts:AssumeRoleWithWebIdentity",
      "Condition": {
        "StringEquals": {
          "token.actions.githubusercontent.com:aud": "sts.amazonaws.com"
        },
        "StringLike": {
          "token.actions.githubusercontent.com:sub": "repo:YOUR_GITHUB_USERNAME/family-todo:ref:refs/heads/main"
        }
      }
    }
  ]
}
```

**📝 置換必要な値**:
- `YOUR_ACCOUNT_ID` → AWSアカウントID
- `YOUR_GITHUB_USERNAME` → GitHubユーザー名

> 💡 **ヒント**: コンソールのフォームで設定すると、上記のJSONが自動生成されます。手動でJSONを入力する場合は「カスタム信頼ポリシー」を選択してください。

#### b) 権限ポリシーの設定

IAMロール作成画面の「許可ポリシー」で `GitHubActionsGeneralDeployPolicy` を検索して選択します：

1. 「許可ポリシー」セクションで検索ボックスに `GitHubActionsGeneralDeployPolicy` と入力
2. ✅チェックを入れて選択
3. 「次へ」をクリック

#### c) ロール名の設定
- **ロール名**: `family-todo-github-actions-role`

> 💡 **命名理由**: 信頼ポリシーで `family-todo` リポジトリに限定されているため、プロジェクト固有のロール名を使用

---

## ステップ2: GitHub側設定 🐙

### 2-1. Repository Secretsの設定 (GitHub)

**GitHubリポジトリ** → **Settings** → **Secrets and variables** → **Actions** → **New repository secret**

📖 **参考**: [暗号化されたシークレット](https://docs.github.com/ja/actions/security-guides/encrypted-secrets#creating-encrypted-secrets-for-a-repository)

以下のSecretsを追加：

| Secret名 | 値 | 説明 |
|---------|-----|------|
| `AWS_DEPLOY_ROLE` | `arn:aws:iam::YOUR_ACCOUNT_ID:role/family-todo-github-actions-role` | IAMロールのARN |
| `API_ENDPOINT` | `https://xxxxx.execute-api.ap-northeast-1.amazonaws.com/api/` | API Gateway URL（デプロイ後に設定） |
| `USER_POOL_ID` | `ap-northeast-1_xxxxxxxxx` | Cognito User Pool ID（今後追加予定） |
| `USER_POOL_CLIENT_ID` | `xxxxxxxxxxxxxxxxxxxxxxxxxx` | Cognito User Pool Client ID（今後追加予定） |
| `S3_BUCKET` | `your-frontend-bucket` | フロントエンド用S3バケット名（今後追加予定） |
| `CF_DISTRIBUTION_ID` | `EXXXXXXXXXXXXXXXXX` | CloudFront Distribution ID（今後追加予定） |

**📝 置換必要な値**:
- `YOUR_ACCOUNT_ID` → AWSアカウントID

---

## ステップ3: 設定確認 ✅

### 3-1. バックエンドデプロイテスト
```bash
# backendディレクトリに変更があることを確認してプッシュ
git add backend/
git commit -m "test: GitHub Actions設定テスト"
git push origin main
```

### 3-2. GitHub Actionsログ確認
**GitHubリポジトリ** → **Actions** タブでワークフローの実行状況を確認

📖 **参考**: [ワークフロー実行の監視](https://docs.github.com/ja/actions/monitoring-and-troubleshooting-workflows/viewing-workflow-run-history)

### 3-3. エラー発生時の確認ポイント
- [ ] AWSアカウントIDが正しく設定されているか
- [ ] GitHubユーザー名が正しく設定されているか
- [ ] IAMロールの信頼関係が正しく設定されているか
- [ ] GitHub Secretsの値が正しく設定されているか
- [ ] Dockerfileが正しく配置されているか（backend/Dockerfile）
- [ ] SAMテンプレートでPackageType: Imageが設定されているか（Rust使用時）
- [ ] ベータ機能に依存していないか

#### よくあるエラーと解決方法

**最終解決方法: Dockerコンテナイメージによるベータ機能完全回避**

**SAM Build エラー: "rust-cargolambda" is a beta feature**の完全な解決策：

**1. backend/Dockerfile の作成**:
```dockerfile
# AWS Lambda Rust Runtime for ARM64
FROM public.ecr.aws/lambda/provided:al2023-arm64

# Install development tools
RUN dnf update -y && \
    dnf install -y gcc gcc-c++ make && \
    dnf clean all

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Add ARM64 target for cross compilation
RUN rustup target add aarch64-unknown-linux-gnu

# Set working directory
WORKDIR ${LAMBDA_TASK_ROOT}

# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build the application
RUN cargo build --release --target aarch64-unknown-linux-gnu

# Copy the binary to the runtime directory as bootstrap
RUN cp target/aarch64-unknown-linux-gnu/release/backend ${LAMBDA_RUNTIME_DIR}/bootstrap

# Set the CMD to your handler
CMD ["bootstrap"]
```

**2. SAMテンプレート設定**:
```yaml
todoHandler:
  Type: AWS::Serverless::Function
  Properties:
    PackageType: Image
    ImageUri: todo-handler:latest
  Metadata:
    DockerTag: latest
    DockerContext: ../backend/
    Dockerfile: Dockerfile
```

**3. GitHub Actions ワークフロー**:
```yaml
- name: SAM Build (Docker Image, no beta features)
  run: |
    cd infra
    sam build
```

**利点**:
- **ベータ機能完全回避**: rust-cargolambdaを使用しない
- **環境一貫性**: Dockerによる完全な環境制御
- **プロダクション対応**: AWS公式サポートのコンテナイメージ方式
- **再現性**: Dockerfileによる完全な環境定義
- **クロスプラットフォーム**: Linux/macOS/Windowsで同一の出力

**ローカル開発時**: 
```bash
cd infra && sam build
sam local start-api
```

📖 **参考**: 
- [GitHub Actions でのトラブルシューティング](https://docs.github.com/ja/actions/monitoring-and-troubleshooting-workflows/troubleshooting-workflows)
- [AWS SAM CLI Rust サポート](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/building-rust.html)

---

## 補足情報

### 将来追加予定のリソース
- DynamoDB Tables（イベントストア、プロジェクション）
- Amazon Cognito（認証）
- S3 + CloudFront（フロントエンド配信）

これらのリソースが追加された際は、対応する権限をIAMポリシーに追加する必要があります。

### セキュリティのポイント
- 最小権限の原則に基づく権限設定
- リソースARNでの権限制限
- ワイルドカード使用の最小化

📖 **参考**: 
- [GitHub Actions のセキュリティ強化](https://docs.github.com/ja/actions/security-guides/security-hardening-for-github-actions)
- [AWS セキュリティのベストプラクティス](https://docs.aws.amazon.com/ja_jp/IAM/latest/UserGuide/best-practices.html)

---

## 関連ツール・リソース 🛠️

### GitHub Actions公式アクション
- [`actions/checkout`](https://github.com/actions/checkout) - リポジトリのチェックアウト
- [`actions/setup-node`](https://github.com/actions/setup-node) - Node.js環境のセットアップ
- [`dtolnay/rust-toolchain`](https://github.com/dtolnay/rust-toolchain) - Rustツールチェーンのセットアップ

### AWS CLI ドキュメント
- [AWS CLI コマンドリファレンス](https://docs.aws.amazon.com/cli/latest/reference/)
- [AWS SAM CLI ドキュメント](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/serverless-sam-cli-command-reference.html)
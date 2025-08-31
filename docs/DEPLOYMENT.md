# デプロイガイド

## 概要

Family Todo App を AWS にデプロイするためのガイドです。
AWS SAM (Serverless Application Model) を使用してインフラを管理し、GitHub Actions で CI/CD パイプラインを実現しています。

## アーキテクチャ図

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   CloudFront    │────│      S3          │    │   API Gateway   │
│      (CDN)      │    │  (Static Site)   │    │      (REST)     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                          │
                       ┌─────────────────┐              │
                       │    Cognito      │              │
                       │   User Pool     │              │
                       └─────────────────┘              │
                                                          │
┌─────────────────────────────────────────────────────────┼──────────────────────────────────┐
│                           AWS Lambda                     │                                  │
├─────────────────┬─────────────────┬─────────────────────┴──────────────┐                   │
│ Command Handler │ Query Handler   │         Event Processor              │                   │
│   (書き込み)     │   (読み取り)    │        (イベント処理)                │                   │
└─────────────────┴─────────────────┴──────────────────────────────────────┘                   │
                                                          │                                    │
                       ┌─────────────────┐              │                                    │
                       │   DynamoDB      │──────────────┘                                    │
                       │ (Single Table)  │                                                   │
                       │   + Streams     │                                                   │
                       └─────────────────┘                                                   │
                                                                                              │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────────┘
│   CloudWatch    │    │      X-Ray      │    │       SQS       │    │
│  (Logs/Metrics) │    │   (Tracing)     │    │   (Dead Letter  │    │
│                 │    │                 │    │     Queue)      │    │
└─────────────────┘    └─────────────────┘    └─────────────────┘    │
```

## 前提条件

### AWS 環境の準備

1. **AWS アカウント**: 適切な権限を持つアカウント
2. **AWS CLI**: 設定済み
3. **AWS SAM CLI**: インストール済み

### 必要な権限

以下のサービスに対する権限が必要です：
- CloudFormation
- Lambda
- API Gateway  
- DynamoDB
- Cognito
- S3
- CloudWatch
- X-Ray
- SQS
- IAM

### 環境変数の設定

```bash
export AWS_REGION=ap-northeast-1
export AWS_PROFILE=your-profile
export STACK_NAME=family-todo-app
```

## デプロイ手順

### 1. 初回デプロイ

#### バックエンドのデプロイ

```bash
# プロジェクトルートから
cd infra

# ガイド付きデプロイ（初回のみ）
sam deploy --guided
```

ガイドで以下を設定：
- Stack Name: `family-todo-app`
- AWS Region: `ap-northeast-1`
- Environment: `prod`
- Confirm changes before deploy: `y`
- Allow SAM CLI IAM role creation: `y`
- Save parameters to configuration file: `y`
- Configuration file name: `samconfig.toml`
- Configuration environment: `default`

#### フロントエンドのビルドとデプロイ

```bash
# フロントエンドのビルド
cd frontend
npm install
npm run build

# S3 バケットの作成（初回のみ）
aws s3 mb s3://family-todo-app-frontend-prod

# ビルド結果をS3にアップロード
aws s3 sync dist/ s3://family-todo-app-frontend-prod/ --delete

# CloudFront ディストリビューションの作成（オプション）
# 詳細は「CloudFront セットアップ」セクションを参照
```

### 2. 更新デプロイ

#### バックエンド

```bash
cd infra
sam deploy
```

#### フロントエンド

```bash
cd frontend
npm run build
aws s3 sync dist/ s3://family-todo-app-frontend-prod/ --delete
```

### 3. デプロイの確認

```bash
# スタックの状態確認
aws cloudformation describe-stacks --stack-name family-todo-app

# API エンドポイントの確認
aws cloudformation describe-stacks \
  --stack-name family-todo-app \
  --query 'Stacks[0].Outputs[?OutputKey==`ApiEndpoint`].OutputValue' \
  --output text

# Lambda 関数の確認
aws lambda list-functions --query 'Functions[?starts_with(FunctionName,`family-todo-app`)]'
```

## 環境別デプロイ

### 開発環境

```bash
sam deploy --parameter-overrides Environment=dev
```

### ステージング環境

```bash
sam deploy \
  --parameter-overrides Environment=staging \
  --stack-name family-todo-app-staging
```

### 本番環境

```bash
sam deploy \
  --parameter-overrides Environment=prod \
  --stack-name family-todo-app-prod
```

## CloudFront セットアップ

フロントエンドの配信を高速化するため、CloudFront を設定します。

### 1. CloudFront ディストリビューション作成

```json
{
  "CallerReference": "family-todo-app-2024",
  "Comment": "Family Todo App Frontend Distribution",
  "Origins": {
    "Quantity": 2,
    "Items": [
      {
        "Id": "S3Origin",
        "DomainName": "family-todo-app-frontend-prod.s3.ap-northeast-1.amazonaws.com",
        "CustomOriginConfig": {
          "HTTPPort": 80,
          "HTTPSPort": 443,
          "OriginProtocolPolicy": "https-only"
        }
      },
      {
        "Id": "APIOrigin", 
        "DomainName": "your-api-id.execute-api.ap-northeast-1.amazonaws.com",
        "CustomOriginConfig": {
          "HTTPPort": 80,
          "HTTPSPort": 443,
          "OriginProtocolPolicy": "https-only"
        }
      }
    ]
  },
  "DefaultCacheBehavior": {
    "TargetOriginId": "S3Origin",
    "ViewerProtocolPolicy": "redirect-to-https",
    "MinTTL": 0,
    "DefaultTTL": 3600,
    "MaxTTL": 86400
  }
}
```

### 2. キャッシュ無効化

デプロイ後にキャッシュをクリア：

```bash
aws cloudfront create-invalidation \
  --distribution-id E1234567890123 \
  --paths "/*"
```

## 監視とアラートの設定

### CloudWatch アラーム

デプロイ時に以下のアラームが自動作成されます：

- **Command Handler エラー率**: 5分間で5エラー以上
- **Query Handler エラー率**: 5分間で10エラー以上  
- **高レイテンシ**: 平均5秒以上
- **DLQ メッセージ**: メッセージが1件以上

### SNS 通知設定

```bash
# SNS トピックの確認
aws sns list-topics --query 'Topics[?contains(TopicArn,`family-todo-app`)]'

# メール通知の設定
aws sns subscribe \
  --topic-arn arn:aws:sns:ap-northeast-1:123456789012:family-todo-app-Alerts \
  --protocol email \
  --notification-endpoint your-email@example.com
```

## ログとデバッグ

### CloudWatch Logs の確認

```bash
# 最新のログを表示
sam logs -n TodoCommandHandler --start-time '10 minutes ago' --tail

# 特定期間のログを検索
aws logs filter-log-events \
  --log-group-name /aws/lambda/family-todo-app-TodoCommandHandler \
  --start-time 1640995200000 \
  --filter-pattern "ERROR"
```

### X-Ray トレースの確認

```bash
# トレース ID の検索
aws xray get-trace-summaries \
  --time-range-type TimeRangeByStartTime \
  --start-time 2024-01-15T10:00:00 \
  --end-time 2024-01-15T11:00:00
```

## データベースの管理

### DynamoDB テーブル確認

```bash
# テーブル情報
aws dynamodb describe-table --table-name family-todo-app-MainTable

# データの確認（注意：本番環境では実行しないこと）
aws dynamodb scan --table-name family-todo-app-MainTable --max-items 10
```

### バックアップ設定

```bash
# Point-in-Time Recovery の確認
aws dynamodb describe-continuous-backups --table-name family-todo-app-MainTable

# 手動バックアップの作成
aws dynamodb create-backup \
  --table-name family-todo-app-MainTable \
  --backup-name family-todo-app-backup-$(date +%Y%m%d)
```

## セキュリティ設定

### Cognito User Pool 設定

```bash
# User Pool の確認
aws cognito-idp describe-user-pool --user-pool-id your-user-pool-id

# User Pool Client の確認  
aws cognito-idp describe-user-pool-client \
  --user-pool-id your-user-pool-id \
  --client-id your-client-id
```

### IAM ロールの確認

```bash
# Lambda 実行ロールの確認
aws iam get-role --role-name family-todo-app-TodoCommandHandlerRole-xxx

# 付与されているポリシーの確認
aws iam list-attached-role-policies --role-name family-todo-app-TodoCommandHandlerRole-xxx
```

## トラブルシューティング

### よくある問題と解決方法

#### 1. デプロイ失敗

**問題**: CloudFormation スタックの作成が失敗する
```bash
# スタックイベントの確認
aws cloudformation describe-stack-events --stack-name family-todo-app

# 失敗したリソースの詳細確認
aws cloudformation describe-stack-resources --stack-name family-todo-app
```

#### 2. Lambda 関数のタイムアウト

**問題**: Lambda 関数が30秒でタイムアウト
```bash
# CloudWatch Logs でエラー内容確認
# 必要に応じてタイムアウト値を増加
sam deploy --parameter-overrides LambdaTimeout=60
```

#### 3. DynamoDB アクセスエラー

**問題**: Lambda から DynamoDB にアクセスできない
```bash
# IAM ロールの権限確認
# VPC 設定の確認（不要なVPC設定の除去）
# セキュリティグループの確認
```

#### 4. API Gateway CORS エラー

**問題**: フロントエンドから API にアクセスできない
```bash
# CORS 設定の確認
aws apigateway get-method \
  --rest-api-id your-api-id \
  --resource-id your-resource-id \
  --http-method OPTIONS
```

### ログ分析

```bash
# エラーログの抽出
aws logs filter-log-events \
  --log-group-name /aws/lambda/family-todo-app-TodoCommandHandler \
  --filter-pattern "ERROR" \
  --start-time 1640995200000

# パフォーマンス分析
aws logs filter-log-events \
  --log-group-name /aws/lambda/family-todo-app-TodoCommandHandler \
  --filter-pattern "[timestamp, requestId, level=REPORT]"
```

## パフォーマンス最適化

### Lambda の最適化

1. **メモリサイズの調整**
   ```bash
   # 現在の設定確認
   aws lambda get-function --function-name family-todo-app-TodoCommandHandler
   
   # メモリサイズの更新
   aws lambda update-function-configuration \
     --function-name family-todo-app-TodoCommandHandler \
     --memory-size 1024
   ```

2. **コールドスタートの削減**
   - Provisioned Concurrency の設定
   - ARM64 アーキテクチャの使用
   - 依存関係の最適化

### DynamoDB の最適化

1. **キャパシティの監視**
   ```bash
   # メトリクスの確認
   aws cloudwatch get-metric-statistics \
     --namespace AWS/DynamoDB \
     --metric-name ConsumedReadCapacityUnits \
     --dimensions Name=TableName,Value=family-todo-app-MainTable \
     --start-time 2024-01-15T00:00:00Z \
     --end-time 2024-01-15T23:59:59Z \
     --period 3600 \
     --statistics Sum
   ```

## コスト管理

### コスト分析

```bash
# 月別コスト確認
aws ce get-cost-and-usage \
  --time-period Start=2024-01-01,End=2024-01-31 \
  --granularity MONTHLY \
  --metrics BlendedCost \
  --group-by Type=DIMENSION,Key=SERVICE
```

### コスト最適化のポイント

1. **Lambda**: 適切なメモリサイズと実行時間の設定
2. **DynamoDB**: On-Demand から Provisioned への変更検討
3. **CloudWatch**: ログ保持期間の設定
4. **S3**: ライフサイクルポリシーの設定

## 災害復旧

### バックアップ戦略

1. **DynamoDB**: Point-in-Time Recovery + 手動バックアップ
2. **Lambda**: SAM テンプレートによるコード管理
3. **S3**: Cross-Region Replication

### 復旧手順

```bash
# 別リージョンでの復旧
sam deploy --region us-west-2 --stack-name family-todo-app-dr

# DynamoDB テーブルの復元
aws dynamodb restore-table-from-backup \
  --target-table-name family-todo-app-MainTable \
  --backup-arn arn:aws:dynamodb:region:account:table/MainTable/backup/01640995200000-abcd1234
```
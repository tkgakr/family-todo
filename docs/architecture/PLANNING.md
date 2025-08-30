# ToDo アプリ ― AWS サーバーレス版 プランニングドキュメント

**目的**: Rust + React/TypeScript で実装する家族用 ToDo 共有アプリを、低コスト・高セキュリティで AWS サーバーレスにデプロイする。イベントソーシングアーキテクチャを採用し、シンプルな UI/UX を実現する。

---

## 1. システム全体概要

| 層             | サービス                                          | 主な役割                                            |
| -------------- | ------------------------------------------------- | --------------------------------------------------- |
| **フロント**   | **S3** 静的ウェブホスティング／**CloudFront** CDN | React SPA (Vite + TS + Biome) 配信                  |
| **API**        | **API Gateway (HTTP)**                            | REST エンドポイント・CORS・Cognito JWT 検証         |
|                | **AWS Lambda (Rust)**                             | コマンド/クエリハンドラー・イベントプロセッサー     |
| **認証**       | **Amazon Cognito** (ユーザープール)               | Passkey (WebAuthn) + リフレッシュトークン           |
| **DB**         | **Amazon DynamoDB**                               | Single Table Design (イベント+プロジェクション)     |
| **ストリーム** | **DynamoDB Streams**                              | イベント駆動での読み取りモデル更新                  |
| **監視**       | **CloudWatch / X-Ray**                            | ログ・メトリクス・分散トレーシング                  |
| **エラー処理** | **SQS DLQ**                                       | 処理失敗イベントの退避                              |
| **CI/CD**      | **GitHub Actions + AWS CLI / SAM**                | ビルド・テスト・デプロイ自動化                      |
| **IaC**        | **AWS SAM** (初期) → 将来 **Terraform/CDK**       | インフラ定義・再現                                  |

---

## 2. リポジトリ構成（モノレポ）

```text
/
├── infra/                  # SAM / Terraform テンプレート
│   ├── template.yaml       # SAM main
│   ├── samconfig.toml
│   └── modules/           # 再利用可能なモジュール
├── backend/               # Rust Lambda 関数群
│   ├── command-handler/   # 書き込み側
│   ├── query-handler/     # 読み取り側
│   ├── event-processor/   # ストリーム処理
│   ├── snapshot-manager/  # スナップショット生成
│   └── shared/           # 共通ドメインモデル
│       ├── domain/       # ドメインモデル・イベント定義
│       ├── infra/        # AWS SDK ラッパー
│       └── telemetry/    # OpenTelemetry設定
├── frontend/              # React/TS (Vite + Biome)
│   ├── src/
│   ├── package.json
│   ├── biome.json        # Biome設定ファイル
│   └── tests/
│       ├── unit/
│       └── e2e/
├── .github/
│   └── workflows/
│       ├── backend.yml
│       ├── frontend.yml
│       └── integration.yml
├── tests/                 # 統合テスト
│   ├── scenarios/
│   └── load/             # 負荷テスト
└── docs/                  # ADR, API Spec, etc.
    ├── adr/              # Architecture Decision Records
    └── api/              # OpenAPI仕様
```

---

## 3. 識別子設計

### TodoId に ULID を採用

- **形式**: 26 文字の Base32 エンコード（例: `01ARZ3NDEKTSV4RRFFQ69G5FAV`）
- **利点**:
  - タイムスタンプ付きで自然にソート可能
  - UUID より短く、DynamoDB のキーとして効率的
  - ID から作成時刻を推測可能（デバッグ性向上）
  - Rust の `ulid` クレートで簡単に実装

### 実装詳細

```rust
use ulid::Ulid;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TodoId(String);

impl TodoId {
    pub fn new() -> Self {
        Self(Ulid::new().to_string())
    }

    pub fn from_string(s: String) -> Result<Self, TodoIdError> {
        Ulid::from_string(&s)?;
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn timestamp_ms(&self) -> Option<u64> {
        Ulid::from_string(&self.0)
            .ok()
            .map(|ulid| ulid.timestamp_ms())
    }
    
    pub fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.timestamp_ms()
            .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64))
    }
}

impl fmt::Display for TodoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

---

## 4. DynamoDB 設計（Single Table Design）

### 統合テーブル構造

| 属性          | 型   | 説明                                              |
| ------------- | ---- | ------------------------------------------------- |
| **PK**        | S    | パーティションキー                                 |
| **SK**        | S    | ソートキー                                        |
| EntityType    | S    | `Event`, `Projection`, `Snapshot`, `Family`       |
| GSI1PK        | S    | GSI1 パーティションキー（アクティブToDo用）         |
| GSI1SK        | S    | GSI1 ソートキー                                   |
| Data          | M    | エンティティ固有のデータ                           |
| Version       | N    | 楽観的ロックバージョン                            |
| TTL           | N    | 有効期限（スナップショット管理用）                 |
| CreatedAt     | S    | ISO8601形式の作成日時                             |
| UpdatedAt     | S    | ISO8601形式の更新日時                             |

### アクセスパターン別のキー設計

| アクセスパターン           | PK                           | SK                           | 備考                        |
| ------------------------- | ---------------------------- | ---------------------------- | --------------------------- |
| イベント保存              | `FAMILY#${familyId}`          | `EVENT#${ulid}`              | ULIDで時系列順              |
| 現在のToDo取得            | `FAMILY#${familyId}`          | `TODO#CURRENT#${todoId}`     | 最新状態                    |
| ToDo履歴取得              | `FAMILY#${familyId}`          | `TODO#EVENT#${todoId}#${ulid}` | 特定ToDoのイベント履歴    |
| スナップショット          | `FAMILY#${familyId}`          | `TODO#SNAPSHOT#${todoId}#${ulid}` | 定期スナップショット    |
| アクティブToDo一覧（GSI1） | `FAMILY#${familyId}#ACTIVE`   | `${ulid}`                    | 未完了のToDo               |
| 家族メタデータ            | `FAMILY#${familyId}`          | `FAMILY#META`                | 家族設定・メンバー情報      |

### GSI 設計

#### GSI1: アクティブToDo インデックス
- **目的**: 未完了ToDoの効率的な取得
- **Projection**: ALL
- **条件**: EntityType = 'Projection' AND Completed = false

#### GSI2: ユーザー別ToDo インデックス（将来拡張用）
- **PK**: `USER#${userId}`
- **SK**: `${ulid}`
- **Projection**: KEYS_ONLY

### イベントスキーマ

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum TodoEvent {
    #[serde(rename = "todo_created_v2")]
    TodoCreatedV2 {
        event_id: String,  // ULID
        todo_id: TodoId,
        title: String,
        description: Option<String>,
        tags: Vec<String>,
        created_by: UserId,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    #[serde(rename = "todo_updated_v1")]
    TodoUpdatedV1 {
        event_id: String,
        todo_id: TodoId,
        title: Option<String>,
        description: Option<String>,
        updated_by: UserId,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    #[serde(rename = "todo_completed_v1")]
    TodoCompletedV1 {
        event_id: String,
        todo_id: TodoId,
        completed_by: UserId,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    #[serde(rename = "todo_deleted_v1")]
    TodoDeletedV1 {
        event_id: String,
        todo_id: TodoId,
        deleted_by: UserId,
        reason: Option<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

// イベントバージョニング・アップキャスト
impl TodoEvent {
    pub fn upcast(self) -> TodoEvent {
        match self {
            // 古いバージョンを新しいバージョンに変換
            // 例: V1からV2への変換ロジック
            _ => self,
        }
    }
}
```

---

## 5. Lambda 関数アーキテクチャ

| 関数名                   | 役割                           | トリガー                      | 同時実行数制限 |
| ----------------------- | ------------------------------ | ----------------------------- | ------------- |
| `TodoCommandHandler`     | 書き込み処理（イベント生成）     | API Gateway (POST/PUT/DELETE) | 10            |
| `TodoEventProcessor`     | イベントからプロジェクション更新 | DynamoDB Streams              | 5             |
| `TodoQueryHandler`       | 読み取り処理                   | API Gateway (GET)             | 20            |
| `TodoSnapshotManager`    | スナップショット生成・削除      | EventBridge (定期実行)         | 1             |

### IAM ロール設計（最小権限の原則）

```yaml
CommandHandlerRole:
  Policies:
    - DynamoDBWritePolicy:
        TableName: !Ref MainTable
        Actions:
          - dynamodb:PutItem
          - dynamodb:ConditionCheckItem
    - CloudWatchLogsPolicy
    - XRayTracingPolicy

EventProcessorRole:
  Policies:
    - DynamoDBStreamReadPolicy:
        StreamArn: !GetAtt MainTable.StreamArn
    - DynamoDBCrudPolicy:
        TableName: !Ref MainTable
        Actions:
          - dynamodb:GetItem
          - dynamodb:Query
          - dynamodb:PutItem
          - dynamodb:UpdateItem
    - SQSSendMessagePolicy:
        QueueName: !Ref DeadLetterQueue
    - CloudWatchLogsPolicy
    - XRayTracingPolicy

QueryHandlerRole:
  Policies:
    - DynamoDBReadOnlyPolicy:
        TableName: !Ref MainTable
        IndexName: GSI1
    - CloudWatchLogsPolicy
    - XRayTracingPolicy

SnapshotManagerRole:
  Policies:
    - DynamoDBCrudPolicy:
        TableName: !Ref MainTable
    - CloudWatchLogsPolicy
    - XRayTracingPolicy
```

---

## 6. エラーハンドリングとリトライ戦略

### DynamoDB Streams エラー処理

```yaml
EventProcessor:
  Type: AWS::Serverless::Function
  Properties:
    Runtime: provided.al2
    Handler: bootstrap
    EventInvokeConfig:
      MaximumEventAge: 3600  # 1時間
      MaximumRetryAttempts: 3
      DestinationConfig:
        OnFailure:
          Type: SQS
          Destination: !GetAtt DeadLetterQueue.Arn
    Events:
      Stream:
        Type: DynamoDB
        Properties:
          Stream: !GetAtt MainTable.StreamArn
          StartingPosition: TRIM_HORIZON
          MaximumBatchingWindowInSeconds: 5
          ParallelizationFactor: 2
          BisectBatchOnFunctionError: true
          ReportBatchItemFailures: true
          FilterCriteria:
            Filters:
              - Pattern: '{"dynamodb": {"NewImage": {"EntityType": {"S": ["Event"]}}}}'
```

### Rust実装でのエラーハンドリング

```rust
use aws_lambda_events::dynamodb::Event as DynamoDbEvent;
use lambda_runtime::{Error, LambdaEvent};
use serde_json::json;

pub async fn handle_stream_event(
    event: LambdaEvent<DynamoDbEvent>,
) -> Result<BatchItemFailures, Error> {
    let mut failures = Vec::new();
    
    for record in event.payload.records {
        let sequence_number = record.event_sequence_number.clone();
        
        match process_record(&record).await {
            Ok(_) => {
                tracing::info!(
                    sequence_number = %sequence_number,
                    "Successfully processed record"
                );
            }
            Err(e) if is_retryable(&e) => {
                tracing::warn!(
                    sequence_number = %sequence_number,
                    error = %e,
                    "Retryable error occurred"
                );
                failures.push(BatchItemFailure {
                    item_identifier: sequence_number,
                });
            }
            Err(e) => {
                tracing::error!(
                    sequence_number = %sequence_number,
                    error = %e,
                    "Non-retryable error occurred, sending to DLQ"
                );
                send_to_dlq(&record, &e).await?;
            }
        }
    }
    
    Ok(BatchItemFailures {
        batch_item_failures: failures,
    })
}

fn is_retryable(error: &ProcessError) -> bool {
    matches!(
        error,
        ProcessError::TemporaryFailure(_) |
        ProcessError::ThrottlingException(_) |
        ProcessError::ServiceUnavailable(_)
    )
}
```

---

## 7. スナップショット戦略

### スナップショット生成ロジック

```rust
const SNAPSHOT_EVENT_THRESHOLD: usize = 100;
const SNAPSHOT_AGE_THRESHOLD_DAYS: i64 = 7;

pub async fn create_snapshot_if_needed(
    todo_id: &TodoId,
    event_count: usize,
    last_snapshot_date: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<(), Error> {
    let should_create_snapshot = event_count >= SNAPSHOT_EVENT_THRESHOLD ||
        last_snapshot_date.map_or(true, |date| {
            chrono::Utc::now().signed_duration_since(date).num_days() >= SNAPSHOT_AGE_THRESHOLD_DAYS
        });
    
    if should_create_snapshot {
        let snapshot = build_snapshot(todo_id).await?;
        save_snapshot(snapshot).await?;
        
        // 古いスナップショットにTTLを設定
        set_old_snapshots_ttl(todo_id).await?;
    }
    
    Ok(())
}

pub async fn rebuild_with_snapshot(todo_id: &TodoId) -> Result<Todo, Error> {
    // 最新のスナップショットを取得
    let snapshot = get_latest_snapshot(todo_id).await?;
    
    // スナップショット以降のイベントを取得
    let events = if let Some(ref snap) = snapshot {
        get_events_after(todo_id, &snap.last_event_id).await?
    } else {
        get_all_events(todo_id).await?
    };
    
    // アグリゲートを再構築
    let mut todo = snapshot.map(|s| s.state).unwrap_or_default();
    let event_count = events.len();
    
    for event in events {
        todo.apply(event.upcast());
    }
    
    // 必要に応じて新しいスナップショットを作成（非同期）
    if event_count >= SNAPSHOT_EVENT_THRESHOLD {
        tokio::spawn(async move {
            if let Err(e) = create_snapshot_if_needed(todo_id, event_count, None).await {
                tracing::error!(error = %e, "Failed to create snapshot");
            }
        });
    }
    
    Ok(todo)
}
```

---

## 8. API エンドポイント設計

| メソッド | パス                       | 説明                | ハンドラー       | レート制限 |
| -------- | ------------------------- | ------------------- | --------------- | --------- |
| POST     | `/todos`                  | ToDo 作成           | CommandHandler  | 10/分     |
| PUT      | `/todos/{id}`             | ToDo 更新           | CommandHandler  | 20/分     |
| POST     | `/todos/{id}/complete`    | ToDo 完了           | CommandHandler  | 20/分     |
| DELETE   | `/todos/{id}`             | ToDo 削除           | CommandHandler  | 5/分      |
| GET      | `/todos`                  | ToDo 一覧取得       | QueryHandler    | 60/分     |
| GET      | `/todos/{id}`             | ToDo 詳細取得       | QueryHandler    | 60/分     |
| GET      | `/todos/{id}/history`     | 履歴取得            | QueryHandler    | 10/分     |
| GET      | `/family/members`         | 家族メンバー一覧     | QueryHandler    | 10/分     |

### OpenAPI仕様（抜粋）

```yaml
openapi: 3.0.3
info:
  title: Family Todo API
  version: 2.0.0
paths:
  /todos:
    post:
      summary: Create a new todo
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [title]
              properties:
                title:
                  type: string
                  minLength: 1
                  maxLength: 200
                description:
                  type: string
                  maxLength: 1000
                tags:
                  type: array
                  items:
                    type: string
                  maxItems: 10
      responses:
        '201':
          description: Todo created successfully
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Todo'
        '400':
          $ref: '#/components/responses/BadRequest'
        '401':
          $ref: '#/components/responses/Unauthorized'
        '429':
          $ref: '#/components/responses/TooManyRequests'
```

---

## 9. 楽観的ロック実装

```rust
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::error::SdkError;

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("Concurrent modification detected")]
    ConcurrentModification,
    #[error("Todo not found")]
    NotFound,
    #[error("DynamoDB error: {0}")]
    DynamoDb(String),
}

pub async fn update_todo_with_lock(
    client: &aws_sdk_dynamodb::Client,
    family_id: &str,
    todo: &Todo,
    updates: TodoUpdates,
) -> Result<Todo, UpdateError> {
    let pk = format!("FAMILY#{}", family_id);
    let sk = format!("TODO#CURRENT#{}", todo.id);
    
    let result = client
        .update_item()
        .table_name("MainTable")
        .key("PK", AttributeValue::S(pk.clone()))
        .key("SK", AttributeValue::S(sk.clone()))
        .update_expression(build_update_expression(&updates))
        .condition_expression("attribute_exists(PK) AND Version = :current_version")
        .expression_attribute_values(":current_version", AttributeValue::N(todo.version.to_string()))
        .expression_attribute_values(":new_version", AttributeValue::N((todo.version + 1).to_string()))
        .expression_attribute_values(":updated_at", AttributeValue::S(chrono::Utc::now().to_rfc3339()))
        .return_values(aws_sdk_dynamodb::types::ReturnValue::AllNew)
        .send()
        .await;
    
    match result {
        Ok(output) => {
            let item = output.attributes.ok_or(UpdateError::NotFound)?;
            Todo::from_dynamodb_item(item).map_err(|e| UpdateError::DynamoDb(e.to_string()))
        }
        Err(SdkError::ServiceError(err)) => {
            if err.err().is_conditional_check_failed_exception() {
                Err(UpdateError::ConcurrentModification)
            } else {
                Err(UpdateError::DynamoDb(err.to_string()))
            }
        }
        Err(e) => Err(UpdateError::DynamoDb(e.to_string())),
    }
}

// リトライロジック
pub async fn update_with_retry(
    client: &aws_sdk_dynamodb::Client,
    family_id: &str,
    todo_id: &TodoId,
    updates: TodoUpdates,
    max_retries: u32,
) -> Result<Todo, UpdateError> {
    let mut retries = 0;
    
    loop {
        // 最新の状態を取得
        let todo = get_todo(client, family_id, todo_id).await?;
        
        match update_todo_with_lock(client, family_id, &todo, updates.clone()).await {
            Ok(updated) => return Ok(updated),
            Err(UpdateError::ConcurrentModification) if retries < max_retries => {
                retries += 1;
                tracing::warn!(retries, "Concurrent modification detected, retrying");
                tokio::time::sleep(tokio::time::Duration::from_millis(100 * (1 << retries))).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

---

## 10. 監視・可観測性

### OpenTelemetry 統合

```rust
use opentelemetry::{global, sdk::trace as sdktrace, trace::Tracer};
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    // OTLP エクスポーター設定
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("https://otel-collector.amazonaws.com");
    
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            sdktrace::config()
                .with_resource(sdktrace::Resource::new(vec![
                    opentelemetry::KeyValue::new("service.name", "todo-backend"),
                    opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ]))
                .with_sampler(sdktrace::Sampler::AlwaysOn),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;
    
    // Tracing subscriber 設定
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().json())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    Ok(())
}

// Lambda ハンドラーでの使用例
#[tracing::instrument(skip(event, context))]
pub async fn handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
    context: Context,
) -> Result<ApiGatewayProxyResponse, Error> {
    let span = tracing::Span::current();
    span.record("request_id", &context.request_id);
    span.record("function_name", &context.env_config.function_name);
    
    // ビジネスロジック
    process_request(event.payload).await
}
```

### CloudWatch アラーム設定

```yaml
Alarms:
  CommandHandlerErrorAlarm:
    Type: AWS::CloudWatch::Alarm
    Properties:
      MetricName: Errors
      Namespace: AWS/Lambda
      Statistic: Sum
      Period: 300
      EvaluationPeriods: 1
      Threshold: 5
      ComparisonOperator: GreaterThanThreshold
      Dimensions:
        - Name: FunctionName
          Value: !Ref TodoCommandHandler
      AlarmActions:
        - !Ref AlertTopic

  HighLatencyAlarm:
    Type: AWS::CloudWatch::Alarm
    Properties:
      MetricName: Duration
      Namespace: AWS/Lambda
      Statistic: Average
      Period: 300
      EvaluationPeriods: 2
      Threshold: 200  # 200ms
      ComparisonOperator: GreaterThanThreshold
      TreatMissingData: notBreaching
```

---

## 11. フロントエンド設定

### Biome 設定 (`biome.json`)

```json
{
  "$schema": "https://biomejs.dev/schemas/1.8.3/schema.json",
  "organizeImports": {
    "enabled": true
  },
  "formatter": {
    "enabled": true,
    "formatWithErrors": false,
    "indentStyle": "space",
    "indentWidth": 2,
    "lineWidth": 100,
    "lineEnding": "lf"
  },
  "linter": {
    "enabled": true,
    "rules": {
      "recommended": true,
      "complexity": {
        "noForEach": "warn",
        "useOptionalChain": "error",
        "useLiteralKeys": "error",
        "noUselessCatch": "error"
      },
      "style": {
        "useConst": "error",
        "useTemplate": "error",
        "noVar": "error",
        "useNodejsImportProtocol": "off"
      },
      "correctness": {
        "noUnusedVariables": "error",
        "useExhaustiveDependencies": "warn"
      },
      "security": {
        "noDangerouslySetInnerHtml": "error"
      },
      "performance": {
        "noAccumulatingSpread": "error"
      }
    }
  },
  "javascript": {
    "formatter": {
      "jsxQuoteStyle": "double",
      "quoteProperties": "asNeeded",
      "trailingComma": "all",
      "semicolons": "asNeeded",
      "arrowParentheses": "always",
      "bracketSpacing": true
    }
  },
  "files": {
    "ignore": ["node_modules", "dist", "build", "coverage", ".next", ".turbo"]
  }
}
```

---

## 12. テスト戦略

### ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    
    #[tokio::test]
    async fn test_todo_creation_with_ulid() {
        // Given
        let family_id = "test-family";
        let user_id = UserId::new();
        let title = "テストToDo".to_string();
        
        // When
        let event = TodoEvent::TodoCreatedV2 {
            event_id: Ulid::new().to_string(),
            todo_id: TodoId::new(),
            title: title.clone(),
            description: None,
            tags: vec![],
            created_by: user_id.clone(),
            timestamp: chrono::Utc::now(),
        };
        
        let mut todo = Todo::default();
        todo.apply(event.clone());
        
        // Then
        assert_eq!(todo.title, title);
        assert_eq!(todo.created_by, user_id);
        assert!(todo.id.timestamp_ms().is_some());
    }
    
    #[tokio::test]
    async fn test_optimistic_locking() {
        let client = create_test_client().await;
        let family_id = "test-family";
        let todo = create_test_todo().await;
        
        // 同時更新をシミュレート
        let update1 = TodoUpdates { title: Some("更新1".to_string()), ..Default::default() };
        let update2 = TodoUpdates { title: Some("更新2".to_string()), ..Default::default() };
        
        // 両方の更新を同時に実行
        let (result1, result2) = tokio::join!(
            update_todo_with_lock(&client, family_id, &todo, update1),
            update_todo_with_lock(&client, family_id, &todo, update2)
        );
        
        // 一方は成功、もう一方は失敗するはず
        assert!(result1.is_ok() ^ result2.is_ok());
        
        let error = result1.err().or(result2.err()).unwrap();
        assert!(matches!(error, UpdateError::ConcurrentModification));
    }
    
    #[tokio::test]
    async fn test_event_ordering_with_ulid() {
        let mut events = vec![];
        
        for i in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            events.push(TodoEvent::TodoUpdatedV1 {
                event_id: Ulid::new().to_string(),
                todo_id: TodoId::new(),
                title: Some(format!("Title {}", i)),
                description: None,
                updated_by: UserId::new(),
                timestamp: chrono::Utc::now(),
            });
        }
        
        // ULIDによって自然に時系列順になることを確認
        let sorted_events = events.clone();
        let mut events_by_id = events.clone();
        events_by_id.sort_by(|a, b| {
            let a_id = match a {
                TodoEvent::TodoUpdatedV1 { event_id, .. } => event_id,
                _ => panic!("Unexpected event type"),
            };
            let b_id = match b {
                TodoEvent::TodoUpdatedV1 { event_id, .. } => event_id,
                _ => panic!("Unexpected event type"),
            };
            a_id.cmp(b_id)
        });
        
        assert_eq!(sorted_events, events_by_id);
    }
}
```

### 統合テスト

```rust
// tests/integration/todo_lifecycle.rs
use aws_sdk_dynamodb::Client;
use lambda_runtime::LambdaEvent;

#[tokio::test]
async fn test_todo_lifecycle_e2e() {
    let client = setup_test_dynamodb().await;
    let family_id = "test-family-001";
    
    // 1. ToDo作成
    let create_event = create_api_gateway_event(
        "POST",
        "/todos",
        json!({
            "title": "買い物リスト",
            "description": "週末の買い出し",
            "tags": ["家事", "急ぎ"]
        }),
    );
    
    let response = command_handler(LambdaEvent::new(create_event, Context::default())).await.unwrap();
    assert_eq!(response.status_code, 201);
    
    let created_todo: Todo = serde_json::from_str(&response.body).unwrap();
    let todo_id = created_todo.id.clone();
    
    // 2. イベントプロセッサーが実行されるまで待機
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // 3. ToDo取得
    let get_event = create_api_gateway_event("GET", &format!("/todos/{}", todo_id), json!({}));
    let response = query_handler(LambdaEvent::new(get_event, Context::default())).await.unwrap();
    assert_eq!(response.status_code, 200);
    
    // 4. ToDo更新
    let update_event = create_api_gateway_event(
        "PUT",
        &format!("/todos/{}", todo_id),
        json!({ "title": "買い物リスト（更新）" }),
    );
    let response = command_handler(LambdaEvent::new(update_event, Context::default())).await.unwrap();
    assert_eq!(response.status_code, 200);
    
    // 5. 履歴確認
    let history_event = create_api_gateway_event(
        "GET",
        &format!("/todos/{}/history", todo_id),
        json!({}),
    );
    let response = query_handler(LambdaEvent::new(history_event, Context::default())).await.unwrap();
    let history: Vec<TodoEvent> = serde_json::from_str(&response.body).unwrap();
    
    assert_eq!(history.len(), 2); // Created + Updated
    
    // 6. ToDo完了
    let complete_event = create_api_gateway_event(
        "POST",
        &format!("/todos/{}/complete", todo_id),
        json!({}),
    );
    let response = command_handler(LambdaEvent::new(complete_event, Context::default())).await.unwrap();
    assert_eq!(response.status_code, 200);
    
    // 7. アクティブToDo一覧から消えていることを確認
    let list_event = create_api_gateway_event("GET", "/todos?status=active", json!({}));
    let response = query_handler(LambdaEvent::new(list_event, Context::default())).await.unwrap();
    let todos: Vec<Todo> = serde_json::from_str(&response.body).unwrap();
    
    assert!(!todos.iter().any(|t| t.id == todo_id));
}
```

### 負荷テスト

```yaml
# tests/load/k6-script.js
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

const errorRate = new Rate('errors');

export const options = {
  stages: [
    { duration: '2m', target: 100 }, // ランプアップ
    { duration: '5m', target: 100 }, // 維持
    { duration: '2m', target: 0 },   // ランプダウン
  ],
  thresholds: {
    http_req_duration: ['p(95)<200'], // 95%が200ms以内
    errors: ['rate<0.01'],             // エラー率1%未満
  },
};

export default function () {
  const url = `${__ENV.API_ENDPOINT}/todos`;
  const payload = JSON.stringify({
    title: `Todo ${Date.now()}`,
    description: 'Load test todo',
  });
  
  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${__ENV.AUTH_TOKEN}`,
    },
  };
  
  const response = http.post(url, payload, params);
  
  const success = check(response, {
    'status is 201': (r) => r.status === 201,
    'response time < 200ms': (r) => r.timings.duration < 200,
  });
  
  errorRate.add(!success);
  sleep(1);
}
```

---

## 13. 家族メンバー管理

### Cognito グループによる家族管理

```yaml
CognitoUserPool:
  Type: AWS::Cognito::UserPool
  Properties:
    UserPoolName: TodoAppUserPool
    MfaConfiguration: OPTIONAL
    EnabledMfas:
      - SOFTWARE_TOKEN_MFA
    Schema:
      - Name: family_id
        AttributeDataType: String
        Mutable: false
        Required: false
    Policies:
      PasswordPolicy:
        MinimumLength: 12
        RequireUppercase: true
        RequireLowercase: true
        RequireNumbers: true
        RequireSymbols: true

FamilyAdminGroup:
  Type: AWS::Cognito::UserPoolGroup
  Properties:
    GroupName: FamilyAdmins
    Description: Family administrators who can manage members
    UserPoolId: !Ref CognitoUserPool
    RoleArn: !GetAtt FamilyAdminRole.Arn
```

### 家族メンバー招待フロー

```rust
pub async fn invite_family_member(
    client: &aws_sdk_cognitoidp::Client,
    dynamo_client: &aws_sdk_dynamodb::Client,
    family_id: &str,
    inviter_id: &UserId,
    invitee_email: &str,
) -> Result<InvitationToken, Error> {
    // 1. 招待者の権限確認
    verify_family_admin(client, family_id, inviter_id).await?;
    
    // 2. 招待トークン生成
    let invitation_token = InvitationToken::new();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    
    // 3. DynamoDBに招待情報を保存
    dynamo_client
        .put_item()
        .table_name("MainTable")
        .item("PK", AttributeValue::S(format!("FAMILY#{}", family_id)))
        .item("SK", AttributeValue::S(format!("INVITATION#{}", invitation_token)))
        .item("EntityType", AttributeValue::S("Invitation".to_string()))
        .item("InviteeEmail", AttributeValue::S(invitee_email.to_string()))
        .item("InviterId", AttributeValue::S(inviter_id.to_string()))
        .item("ExpiresAt", AttributeValue::S(expires_at.to_rfc3339()))
        .item("TTL", AttributeValue::N(expires_at.timestamp().to_string()))
        .send()
        .await?;
    
    // 4. 招待メール送信（SES使用）
    send_invitation_email(invitee_email, &invitation_token).await?;
    
    Ok(invitation_token)
}
```

---

## 14. GDPR対応とデータ削除

### 論理削除と物理削除の併用

```rust
pub enum DeletionType {
    Soft,  // 通常の削除（イベントとして記録）
    Hard,  // GDPR要求による完全削除
}

pub async fn delete_todo(
    client: &aws_sdk_dynamodb::Client,
    family_id: &str,
    todo_id: &TodoId,
    deletion_type: DeletionType,
    reason: Option<String>,
) -> Result<(), Error> {
    match deletion_type {
        DeletionType::Soft => {
            // イベントとして削除を記録
            let event = TodoEvent::TodoDeletedV1 {
                event_id: Ulid::new().to_string(),
                todo_id: todo_id.clone(),
                deleted_by: get_current_user_id(),
                reason,
                timestamp: chrono::Utc::now(),
            };
            save_event(client, family_id, event).await?;
        }
        DeletionType::Hard => {
            // GDPR対応：全データを物理削除
            // 1. 全イベントを削除
            let events = get_all_events_for_todo(client, family_id, todo_id).await?;
            for event in events {
                delete_item(client, &event.pk, &event.sk).await?;
            }
            
            // 2. プロジェクションを削除
            delete_item(
                client,
                &format!("FAMILY#{}", family_id),
                &format!("TODO#CURRENT#{}", todo_id),
            ).await?;
            
            // 3. スナップショットを削除
            let snapshots = get_all_snapshots(client, family_id, todo_id).await?;
            for snapshot in snapshots {
                delete_item(client, &snapshot.pk, &snapshot.sk).await?;
            }
            
            // 4. 監査ログに記録（匿名化）
            log_gdpr_deletion(todo_id, reason).await?;
        }
    }
    
    Ok(())
}
```

---

## 15. パフォーマンス目標と測定

### SLO (Service Level Objectives)

| メトリクス                 | 目標値      | 測定方法                           |
| ------------------------- | ----------- | ---------------------------------- |
| API レスポンスタイム (p50) | < 50ms      | CloudWatch Metrics                |
| API レスポンスタイム (p95) | < 200ms     | CloudWatch Metrics (ウォーム時)    |
| API レスポンスタイム (p99) | < 500ms     | CloudWatch Metrics (コールド含む)  |
| エラー率                   | < 0.1%      | CloudWatch Metrics                |
| 可用性                     | > 99.9%     | CloudWatch Synthetics              |
| DynamoDB読み取り遅延       | < 10ms      | X-Ray トレース                     |
| DynamoDB書き込み遅延       | < 20ms      | X-Ray トレース                     |

### コールドスタート最適化

```rust
// Lambda関数の最適化
use once_cell::sync::Lazy;
use aws_config::BehaviorVersion;

// グローバルでクライアントを初期化（コールドスタート時1回のみ）
static DYNAMODB_CLIENT: Lazy<aws_sdk_dynamodb::Client> = Lazy::new(|| {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .load()
            .await;
        aws_sdk_dynamodb::Client::new(&config)
    })
});

// ARM64アーキテクチャを使用（Graviton2）
// メモリ: 512MB（コストとパフォーマンスのバランス）
// 予約済み同時実行数: 各関数に設定
```

---

## 16. CI/CD パイプライン詳細

### Backend デプロイワークフロー

```yaml
name: Deploy Backend
on:
  push:
    branches: [main]
    paths:
      - "backend/**"
      - "infra/**"
  pull_request:
    branches: [main]
    paths:
      - "backend/**"

env:
  RUST_VERSION: "1.75"
  AWS_REGION: "ap-northeast-1"

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: ${{ env.RUST_VERSION }}
          targets: aarch64-unknown-linux-musl
          components: rustfmt, clippy
      
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: "backend -> target"
      
      - name: Run tests
        run: |
          cd backend
          cargo fmt -- --check
          cargo clippy -- -D warnings
          cargo test --all-features
      
      - name: Security audit
        run: |
          cargo install cargo-audit
          cargo audit

  deploy:
    needs: test
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    permissions:
      id-token: write
      contents: read
    
    steps:
      - uses: actions/checkout@v4
      
      - uses: aws-actions/setup-sam@v2
        with:
          use-installer: true
      
      - uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: ${{ secrets.AWS_DEPLOY_ROLE }}
          aws-region: ${{ env.AWS_REGION }}
      
      - name: Build with SAM
        run: |
          sam build \
            --use-container \
            --parallel \
            --cached
      
      - name: Deploy to AWS
        run: |
          sam deploy \
            --no-confirm-changeset \
            --no-fail-on-empty-changeset \
            --stack-name todo-app-backend \
            --s3-bucket ${{ secrets.SAM_BUCKET }} \
            --capabilities CAPABILITY_IAM \
            --parameter-overrides \
              Environment=${{ github.ref == 'refs/heads/main' && 'prod' || 'dev' }}
      
      - name: Run smoke tests
        run: |
          cd tests
          npm run smoke-test
```

---

## 17. ローカル開発環境

### Docker Compose 設定

```yaml
version: "3.9"

services:
  dynamodb-local:
    image: amazon/dynamodb-local:latest
    container_name: todo-dynamodb
    ports:
      - "8000:8000"
    command: "-jar DynamoDBLocal.jar -sharedDb -inMemory"
    networks:
      - todo-network

  localstack:
    image: localstack/localstack:3.0
    container_name: todo-localstack
    ports:
      - "4566:4566"
      - "4571:4571"
    environment:
      - SERVICES=cognito-idp,ses,sqs
      - DEBUG=1
      - DATA_DIR=/tmp/localstack/data
      - LAMBDA_EXECUTOR=docker
      - DOCKER_HOST=unix:///var/run/docker.sock
    volumes:
      - "${TMPDIR:-/tmp}/localstack:/tmp/localstack"
      - "/var/run/docker.sock:/var/run/docker.sock"
    networks:
      - todo-network

  redis:
    image: redis:7-alpine
    container_name: todo-redis
    ports:
      - "6379:6379"
    networks:
      - todo-network

networks:
  todo-network:
    driver: bridge

volumes:
  localstack-data:
```

### 開発用 Makefile

```makefile
.PHONY: help
help:
	@echo "Available commands:"
	@echo "  make dev-up        - Start local development environment"
	@echo "  make dev-down      - Stop local development environment"
	@echo "  make test          - Run all tests"
	@echo "  make test-unit     - Run unit tests"
	@echo "  make test-integration - Run integration tests"
	@echo "  make fmt           - Format code"
	@echo "  make lint          - Run linters"
	@echo "  make build         - Build all Lambda functions"
	@echo "  make deploy-local  - Deploy to local environment"

.PHONY: dev-up
dev-up:
	docker-compose up -d
	./scripts/setup-local-db.sh
	cd frontend && npm run dev &
	cd backend && cargo watch -x test -x run

.PHONY: dev-down
dev-down:
	docker-compose down
	pkill -f "npm run dev" || true
	pkill -f "cargo watch" || true

.PHONY: test
test: test-unit test-integration

.PHONY: test-unit
test-unit:
	cd backend && cargo test --lib
	cd frontend && npm test

.PHONY: test-integration
test-integration:
	cd backend && cargo test --test '*'
	cd frontend && npm run test:e2e

.PHONY: fmt
fmt:
	cd backend && cargo fmt
	cd frontend && npm run format

.PHONY: lint
lint:
	cd backend && cargo clippy -- -D warnings
	cd frontend && npm run lint

.PHONY: build
build:
	sam build --use-container --parallel

.PHONY: deploy-local
deploy-local: build
	sam local start-api \
		--warm-containers EAGER \
		--port 3001 \
		--env-vars env.json
```

---

## 18. コスト試算（詳細版）

### 月間使用量予測（5家族、各家族20人まで）

| サービス              | 想定使用量                          | 月額費用     |
| -------------------- | ----------------------------------- | ------------ |
| **Lambda**           |                                     |              |
| - 実行回数           | 30,000回（1家族6,000回/月）         | $0.00        |
| - 実行時間           | 50ms平均 × 30,000 = 1,500秒         | $0.00        |
| - メモリ使用         | 512MB                               | $0.00        |
| **API Gateway**      | 30,000 リクエスト                    | $0.03        |
| **DynamoDB**         |                                     |              |
| - 書き込み           | 10,000 WCU                          | $0.00        |
| - 読み取り           | 20,000 RCU                          | $0.00        |
| - ストレージ         | 100MB                               | $0.00        |
| - Streams            | 10,000 レコード                      | $0.00        |
| **Cognito**          | 100 MAU                             | $0.00        |
| **S3**               | 10MB (フロントエンド)                | $0.00        |
| **CloudFront**       | 1GB 転送                            | $0.00        |
| **CloudWatch**       |                                     |              |
| - ログ保存           | 1GB                                 | $0.50        |
| - メトリクス         | 10 カスタムメトリクス                | $0.00        |
| **X-Ray**            | 100,000 トレース                    | $0.00        |
| **合計**             |                                     | **約$0.53/月** |

※ AWS Free Tier適用後の金額

---

## 19. 今後の拡張計画

### Phase 1 (3ヶ月)
- ✅ 基本的なCRUD機能
- ✅ 家族間でのToDo共有
- ✅ Passkey認証
- ⬜ プッシュ通知（期限接近）
- ⬜ ファイル添付機能

### Phase 2 (6ヶ月)
- ⬜ リアルタイム同期（WebSocket API）
- ⬜ 定期的なToDo（繰り返しタスク）
- ⬜ ToDoテンプレート
- ⬜ 統計・分析ダッシュボード
- ⬜ モバイルアプリ（React Native）

### Phase 3 (12ヶ月)
- ⬜ AI による ToDo 提案
- ⬜ 音声入力対応
- ⬜ カレンダー連携
- ⬜ 外部サービス連携（IFTTT等）
- ⬜ マルチテナント対応

---

## まとめ

本プロジェクトは、シンプルなUIを保ちながら、以下の高度な技術要素を実装する学習プロジェクトです：

1. **イベントソーシング + CQRS** による完全な監査証跡と柔軟な読み取りモデル
2. **ULID** による効率的な分散ID管理とソート可能性
3. **Single Table Design** によるDynamoDBの最適化
4. **楽観的ロック** による同時実行制御
5. **OpenTelemetry** による包括的な可観測性
6. **スナップショット戦略** による長期運用への対応
7. **GDPR対応** のデータ削除メカニズム

これらの実装を通じて、プロダクションレベルのサーバーレスアプリケーション開発スキルを習得できます。
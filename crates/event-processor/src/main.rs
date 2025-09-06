use aws_lambda_events::event::dynamodb::{Event as DynamoDbEvent, EventRecord};

#[cfg(test)]
use infrastructure::DynamoDbClient;
use infrastructure::ProjectionRepository;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use shared::{trace_lambda_handler, tracing::init_tracing};
use tracing::{debug, error, info, warn};

/// DynamoDB Streams イベント処理のエラー型
#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
    #[error("設定エラー: {0}")]
    Config(String),

    #[error("DynamoDB エラー: {0}")]
    DynamoDb(String),

    #[error("イベント解析エラー: {0}")]
    EventParsing(String),

    #[error("プロジェクション更新エラー: {0}")]
    ProjectionUpdate(String),

    #[error("リトライ可能エラー: {0}")]
    Retryable(String),

    #[error("内部エラー: {0}")]
    Internal(String),
}

/// バッチ処理の失敗アイテム
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchItemFailure {
    #[serde(rename = "itemIdentifier")]
    pub item_identifier: Option<String>,
}

/// バッチ処理の失敗レスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchItemFailures {
    #[serde(rename = "batchItemFailures")]
    pub batch_item_failures: Vec<BatchItemFailure>,
}

/// EventProcessor のメイン処理
pub struct EventProcessor {
    #[allow(dead_code)]
    projection_repo: ProjectionRepository,
}

impl EventProcessor {
    pub fn new(projection_repo: ProjectionRepository) -> Self {
        Self { projection_repo }
    }

    /// DynamoDB Streams イベントを処理
    pub async fn process_stream_event(
        &self,
        event: DynamoDbEvent,
    ) -> Result<BatchItemFailures, ProcessorError> {
        info!(
            "DynamoDB Streams イベント処理開始: {} レコード",
            event.records.len()
        );

        let mut failures = Vec::new();

        for record in event.records {
            match self.process_record(&record).await {
                Ok(_) => {
                    debug!("レコード処理成功: {:?}", record.event_id);
                }
                Err(ProcessorError::Retryable(msg)) => {
                    warn!("リトライ可能エラー: {}", msg);
                    failures.push(BatchItemFailure {
                        item_identifier: Some(record.event_id.clone()),
                    });
                }
                Err(e) => {
                    error!("レコード処理失敗: {}", e);
                    // リトライ不可エラーはDLQに送信（実装は後で追加）
                    self.send_to_dlq(&record, &e).await;
                }
            }
        }

        info!("DynamoDB Streams イベント処理完了: {} 失敗", failures.len());

        Ok(BatchItemFailures {
            batch_item_failures: failures,
        })
    }

    /// 個別レコードの処理（プレースホルダー実装）
    async fn process_record(&self, record: &EventRecord) -> Result<(), ProcessorError> {
        debug!("レコード処理開始: {:?}", record.event_name);

        // プレースホルダー実装
        // 実際の実装では、DynamoDB Streams のレコードを解析して
        // イベントを抽出し、プロジェクションを更新する

        match record.event_name.as_str() {
            "INSERT" | "MODIFY" => {
                info!("イベント挿入/更新を検出: {}", record.event_id);
                // TODO: 実際のイベント解析とプロジェクション更新
                self.process_event_change(record).await?;
            }
            "REMOVE" => {
                info!("イベント削除を検出: {}", record.event_id);
                // TODO: 削除処理
                self.process_event_removal(record).await?;
            }
            _ => {
                debug!("処理対象外のイベント: {:?}", record.event_name);
            }
        }

        Ok(())
    }

    /// イベント変更の処理（プレースホルダー）
    async fn process_event_change(&self, _record: &EventRecord) -> Result<(), ProcessorError> {
        // プレースホルダー実装
        // 実際の実装では：
        // 1. DynamoDB レコードからイベントデータを抽出
        // 2. TodoEvent にデシリアライズ
        // 3. プロジェクションを更新
        // 4. 必要に応じてスナップショット作成をトリガー

        debug!("イベント変更処理（プレースホルダー）");
        Ok(())
    }

    /// イベント削除の処理（プレースホルダー）
    async fn process_event_removal(&self, _record: &EventRecord) -> Result<(), ProcessorError> {
        // プレースホルダー実装
        // 実際の実装では：
        // 1. 削除されたプロジェクションの処理
        // 2. 関連するインデックスの更新

        debug!("イベント削除処理（プレースホルダー）");
        Ok(())
    }

    /// DLQへの送信（プレースホルダー実装）
    async fn send_to_dlq(&self, record: &EventRecord, error: &ProcessorError) {
        error!("DLQに送信: record={:?}, error={}", record.event_id, error);
        // 実際の実装では、SQS DLQにメッセージを送信
        // ここではログ出力のみ
    }
}

/// Lambda関数のエントリーポイント
async fn function_handler(event: LambdaEvent<DynamoDbEvent>) -> Result<BatchItemFailures, Error> {
    let (payload, context) = event.into_parts();

    // トレーシングでラップされたハンドラー実行
    trace_lambda_handler!(
        "event-processor",
        payload,
        context,
        |payload: DynamoDbEvent, _context| async move {
            info!("EventProcessor Lambda関数開始");

            // 設定を読み込み
            let config = shared::Config::from_env().map_err(|e| {
                error!("設定読み込みエラー: {}", e);
                Error::from(format!("設定エラー: {e}"))
            })?;

            // DynamoDBクライアントを初期化
            let db_client = infrastructure::DynamoDbClient::new(&config)
                .await
                .map_err(|e| {
                    error!("DynamoDBクライアント初期化エラー: {}", e);
                    Error::from(format!("DynamoDBエラー: {e}"))
                })?;

            // プロジェクションリポジトリを初期化
            let projection_repo = ProjectionRepository::new(db_client);

            // EventProcessorを初期化
            let processor = EventProcessor::new(projection_repo);

            // イベントを処理
            match processor.process_stream_event(payload).await {
                Ok(result) => {
                    info!("EventProcessor Lambda関数完了");
                    Ok(result)
                }
                Err(e) => {
                    error!("EventProcessor処理エラー: {}", e);
                    Err(Error::from(format!("処理エラー: {e}")))
                }
            }
        }
    )
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // OpenTelemetry トレーシングを初期化
    if let Err(e) = init_tracing() {
        eprintln!("トレーシング初期化エラー: {e}");
        // トレーシング初期化に失敗してもアプリケーションは継続
    }

    let result = run(service_fn(function_handler)).await;

    // Lambda 終了時にトレーサーをシャットダウン
    shared::tracing::shutdown_telemetry();

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::Config;

    #[tokio::test]
    async fn test_event_processor_creation() {
        // テスト用の設定
        let config = Config {
            dynamodb_table: "test-table".to_string(),
            environment: "test".to_string(),
            aws_region: "ap-northeast-1".to_string(),
            dynamodb_endpoint: Some("http://localhost:8000".to_string()),
            retry_max_attempts: 2,
            retry_initial_delay_ms: 10,
        };

        // DynamoDBクライアントが利用できない場合はスキップ
        if let Ok(db_client) = DynamoDbClient::new(&config).await {
            let projection_repo = ProjectionRepository::new(db_client);
            let processor = EventProcessor::new(projection_repo);

            // プロセッサーが正常に作成されることを確認
            assert!(!(std::ptr::addr_of!(processor) as *const u8).is_null());
        }
    }

    #[test]
    fn test_batch_item_failures_serialization() {
        let failures = BatchItemFailures {
            batch_item_failures: vec![
                BatchItemFailure {
                    item_identifier: Some("1".to_string()),
                },
                BatchItemFailure {
                    item_identifier: Some("2".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&failures).unwrap();
        assert!(json.contains("batchItemFailures"));
        assert!(json.contains("itemIdentifier"));
    }

    #[test]
    fn test_processor_error_display() {
        let error = ProcessorError::EventParsing("テストエラー".to_string());
        assert!(error.to_string().contains("イベント解析エラー"));
        assert!(error.to_string().contains("テストエラー"));
    }
}

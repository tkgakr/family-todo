use aws_sdk_cloudwatch::types::{Dimension, MetricDatum, StandardUnit};
use aws_sdk_cloudwatch::Client as CloudWatchClient;
use aws_smithy_types::DateTime as AwsDateTime;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{error, info, instrument};

/// CloudWatch カスタムメトリクス送信クライアント
#[derive(Clone)]
pub struct MetricsClient {
    client: CloudWatchClient,
    namespace: String,
    default_dimensions: Vec<Dimension>,
}

impl MetricsClient {
    /// 新しいメトリクスクライアントを作成
    pub fn new(client: CloudWatchClient, namespace: String, environment: String) -> Self {
        let default_dimensions = vec![Dimension::builder()
            .name("Environment")
            .value(environment)
            .build()];

        Self {
            client,
            namespace,
            default_dimensions,
        }
    }

    /// カウンターメトリクスを送信
    #[instrument(skip(self))]
    pub async fn put_count_metric(
        &self,
        metric_name: &str,
        value: f64,
        dimensions: Option<HashMap<String, String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.put_metric(metric_name, value, StandardUnit::Count, dimensions)
            .await
    }

    /// 時間メトリクス（ミリ秒）を送信
    #[instrument(skip(self))]
    pub async fn put_duration_metric(
        &self,
        metric_name: &str,
        duration_ms: f64,
        dimensions: Option<HashMap<String, String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.put_metric(
            metric_name,
            duration_ms,
            StandardUnit::Milliseconds,
            dimensions,
        )
        .await
    }

    /// パーセンテージメトリクスを送信
    #[instrument(skip(self))]
    pub async fn put_percentage_metric(
        &self,
        metric_name: &str,
        percentage: f64,
        dimensions: Option<HashMap<String, String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.put_metric(metric_name, percentage, StandardUnit::Percent, dimensions)
            .await
    }

    /// 汎用メトリクス送信
    #[instrument(skip(self))]
    pub async fn put_metric(
        &self,
        metric_name: &str,
        value: f64,
        unit: StandardUnit,
        dimensions: Option<HashMap<String, String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut metric_dimensions = self.default_dimensions.clone();

        // 追加のディメンションを設定
        if let Some(dims) = dimensions {
            for (key, value) in dims {
                metric_dimensions.push(Dimension::builder().name(key).value(value).build());
            }
        }

        let metric_datum = MetricDatum::builder()
            .metric_name(metric_name)
            .value(value)
            .unit(unit)
            .timestamp(AwsDateTime::from_secs(Utc::now().timestamp()))
            .set_dimensions(Some(metric_dimensions))
            .build();

        let result = self
            .client
            .put_metric_data()
            .namespace(&self.namespace)
            .metric_data(metric_datum)
            .send()
            .await;

        match &result {
            Ok(_) => {
                info!(
                    metric_name = metric_name,
                    value = value,
                    "Custom metric sent successfully"
                );
            }
            Err(e) => {
                error!(
                    metric_name = metric_name,
                    error = %e,
                    "Failed to send custom metric"
                );
            }
        }

        result
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            .map(|_| ())
    }

    /// バッチでメトリクスを送信（最大20個まで）
    #[instrument(skip(self, metrics))]
    pub async fn put_metrics_batch(
        &self,
        metrics: Vec<CustomMetric>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if metrics.is_empty() {
            return Ok(());
        }

        // CloudWatch API の制限により、一度に最大20個のメトリクスまで送信可能
        const BATCH_SIZE: usize = 20;

        for chunk in metrics.chunks(BATCH_SIZE) {
            let mut metric_data = Vec::new();

            for metric in chunk {
                let mut metric_dimensions = self.default_dimensions.clone();

                // 追加のディメンションを設定
                for (key, value) in &metric.dimensions {
                    metric_dimensions.push(Dimension::builder().name(key).value(value).build());
                }

                let metric_datum = MetricDatum::builder()
                    .metric_name(&metric.name)
                    .value(metric.value)
                    .unit(metric.unit.clone())
                    .timestamp(AwsDateTime::from_secs(metric.timestamp.timestamp()))
                    .set_dimensions(Some(metric_dimensions))
                    .build();

                metric_data.push(metric_datum);
            }

            let result = self
                .client
                .put_metric_data()
                .namespace(&self.namespace)
                .set_metric_data(Some(metric_data))
                .send()
                .await;

            if let Err(e) = result {
                error!(
                    batch_size = chunk.len(),
                    error = %e,
                    "Failed to send metrics batch"
                );
                return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
            } else {
                info!(batch_size = chunk.len(), "Metrics batch sent successfully");
            }
        }

        Ok(())
    }
}

/// カスタムメトリクス定義
#[derive(Debug, Clone)]
pub struct CustomMetric {
    pub name: String,
    pub value: f64,
    pub unit: StandardUnit,
    pub dimensions: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

impl CustomMetric {
    /// 新しいカスタムメトリクスを作成
    pub fn new(
        name: String,
        value: f64,
        unit: StandardUnit,
        dimensions: HashMap<String, String>,
    ) -> Self {
        Self {
            name,
            value,
            unit,
            dimensions,
            timestamp: Utc::now(),
        }
    }

    /// カウンターメトリクスを作成
    pub fn count(name: String, value: f64, dimensions: HashMap<String, String>) -> Self {
        Self::new(name, value, StandardUnit::Count, dimensions)
    }

    /// 時間メトリクス（ミリ秒）を作成
    pub fn duration_ms(
        name: String,
        duration_ms: f64,
        dimensions: HashMap<String, String>,
    ) -> Self {
        Self::new(name, duration_ms, StandardUnit::Milliseconds, dimensions)
    }

    /// パーセンテージメトリクスを作成
    pub fn percentage(name: String, percentage: f64, dimensions: HashMap<String, String>) -> Self {
        Self::new(name, percentage, StandardUnit::Percent, dimensions)
    }
}

/// ビジネスメトリクス収集用のヘルパー関数群
pub struct BusinessMetrics;

impl BusinessMetrics {
    /// ToDo 作成メトリクス
    pub fn todo_created(family_id: &str, user_id: &str) -> CustomMetric {
        let mut dimensions = HashMap::new();
        dimensions.insert("FamilyId".to_string(), family_id.to_string());
        dimensions.insert("UserId".to_string(), user_id.to_string());
        dimensions.insert("Operation".to_string(), "CreateTodo".to_string());

        CustomMetric::count("TodoOperations".to_string(), 1.0, dimensions)
    }

    /// ToDo 更新メトリクス
    pub fn todo_updated(family_id: &str, user_id: &str) -> CustomMetric {
        let mut dimensions = HashMap::new();
        dimensions.insert("FamilyId".to_string(), family_id.to_string());
        dimensions.insert("UserId".to_string(), user_id.to_string());
        dimensions.insert("Operation".to_string(), "UpdateTodo".to_string());

        CustomMetric::count("TodoOperations".to_string(), 1.0, dimensions)
    }

    /// ToDo 完了メトリクス
    pub fn todo_completed(family_id: &str, user_id: &str) -> CustomMetric {
        let mut dimensions = HashMap::new();
        dimensions.insert("FamilyId".to_string(), family_id.to_string());
        dimensions.insert("UserId".to_string(), user_id.to_string());
        dimensions.insert("Operation".to_string(), "CompleteTodo".to_string());

        CustomMetric::count("TodoOperations".to_string(), 1.0, dimensions)
    }

    /// ToDo 削除メトリクス
    pub fn todo_deleted(family_id: &str, user_id: &str) -> CustomMetric {
        let mut dimensions = HashMap::new();
        dimensions.insert("FamilyId".to_string(), family_id.to_string());
        dimensions.insert("UserId".to_string(), user_id.to_string());
        dimensions.insert("Operation".to_string(), "DeleteTodo".to_string());

        CustomMetric::count("TodoOperations".to_string(), 1.0, dimensions)
    }

    /// DynamoDB 操作時間メトリクス
    pub fn dynamodb_operation_duration(
        table_name: &str,
        operation: &str,
        duration_ms: f64,
    ) -> CustomMetric {
        let mut dimensions = HashMap::new();
        dimensions.insert("TableName".to_string(), table_name.to_string());
        dimensions.insert("Operation".to_string(), operation.to_string());

        CustomMetric::duration_ms(
            "DynamoDBOperationDuration".to_string(),
            duration_ms,
            dimensions,
        )
    }

    /// API レスポンス時間メトリクス
    pub fn api_response_time(
        method: &str,
        path: &str,
        status_code: u16,
        duration_ms: f64,
    ) -> CustomMetric {
        let mut dimensions = HashMap::new();
        dimensions.insert("Method".to_string(), method.to_string());
        dimensions.insert("Path".to_string(), path.to_string());
        dimensions.insert("StatusCode".to_string(), status_code.to_string());

        CustomMetric::duration_ms("ApiResponseTime".to_string(), duration_ms, dimensions)
    }

    /// エラー率メトリクス
    pub fn error_rate(
        service: &str,
        error_type: &str,
        error_count: f64,
        total_count: f64,
    ) -> CustomMetric {
        let error_rate = if total_count > 0.0 {
            (error_count / total_count) * 100.0
        } else {
            0.0
        };

        let mut dimensions = HashMap::new();
        dimensions.insert("Service".to_string(), service.to_string());
        dimensions.insert("ErrorType".to_string(), error_type.to_string());

        CustomMetric::percentage("ErrorRate".to_string(), error_rate, dimensions)
    }

    /// 楽観的ロック競合メトリクス
    pub fn optimistic_lock_conflict(family_id: &str, todo_id: &str) -> CustomMetric {
        let mut dimensions = HashMap::new();
        dimensions.insert("FamilyId".to_string(), family_id.to_string());
        dimensions.insert("TodoId".to_string(), todo_id.to_string());

        CustomMetric::count("OptimisticLockConflicts".to_string(), 1.0, dimensions)
    }

    /// イベント処理メトリクス
    pub fn event_processed(event_type: &str, processing_duration_ms: f64) -> CustomMetric {
        let mut dimensions = HashMap::new();
        dimensions.insert("EventType".to_string(), event_type.to_string());

        CustomMetric::duration_ms(
            "EventProcessingDuration".to_string(),
            processing_duration_ms,
            dimensions,
        )
    }

    /// スナップショット作成メトリクス
    pub fn snapshot_created(family_id: &str, todo_count: f64) -> CustomMetric {
        let mut dimensions = HashMap::new();
        dimensions.insert("FamilyId".to_string(), family_id.to_string());

        CustomMetric::count("SnapshotsCreated".to_string(), todo_count, dimensions)
    }
}

/// メトリクス送信のマクロ
#[macro_export]
macro_rules! send_metric {
    ($metrics_client:expr, $metric:expr) => {{
        use tracing::error;

        if let Err(e) = $metrics_client.put_metrics_batch(vec![$metric]).await {
            error!(error = %e, "Failed to send metric");
        }
    }};
}

/// 複数メトリクス送信のマクロ
#[macro_export]
macro_rules! send_metrics {
    ($metrics_client:expr, $metrics:expr) => {{
        use tracing::error;

        if let Err(e) = $metrics_client.put_metrics_batch($metrics).await {
            error!(error = %e, "Failed to send metrics batch");
        }
    }};
}

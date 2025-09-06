use crate::metrics::MetricsClient;
use lambda_runtime::Context;
use std::collections::HashMap;
use tracing::{error, info, instrument, warn};

/// Lambda 関数のトレーシング情報
#[derive(Debug)]
pub struct LambdaTraceContext {
    pub function_name: String,
    pub function_version: String,
    pub request_id: String,
    pub trace_id: Option<String>,
}

impl LambdaTraceContext {
    /// Lambda Context からトレーシング情報を抽出
    pub fn from_lambda_context(context: &Context) -> Self {
        Self {
            function_name: context.env_config.function_name.clone(),
            function_version: context.env_config.version.clone(),
            request_id: context.request_id.clone(),
            trace_id: std::env::var("_X_AMZN_TRACE_ID").ok(),
        }
    }
}

/// Lambda 関数実行をトレースするマクロ
#[macro_export]
macro_rules! trace_lambda_handler {
    ($handler_name:expr, $event:expr, $context:expr, $handler_fn:expr) => {{
        use $crate::telemetry::{create_lambda_span, LambdaTraceContext};
        use tracing::{error, info};

        let trace_context = LambdaTraceContext::from_lambda_context(&$context);
        let span = create_lambda_span($handler_name, &trace_context);
        let _guard = span.enter();

        info!(
            function_name = %trace_context.function_name,
            request_id = %trace_context.request_id,
            "Lambda function started"
        );

        let result = $handler_fn($event, $context).await;

        match &result {
            Ok(_) => {
                info!("Lambda function completed successfully");
            }
            Err(e) => {
                error!(error = %e, "Lambda function failed");
            }
        }

        result
    }};
}

/// Lambda 関数用のスパンを作成
pub fn create_lambda_span(handler_name: &str, trace_context: &LambdaTraceContext) -> tracing::Span {
    tracing::span!(
        tracing::Level::INFO,
        "lambda_handler",
        handler = handler_name,
        function_name = %trace_context.function_name,
        function_version = %trace_context.function_version,
        request_id = %trace_context.request_id,
        trace_id = %trace_context.trace_id.as_deref().unwrap_or("none")
    )
}

/// DynamoDB 操作をトレース
#[instrument(skip(operation))]
pub async fn trace_dynamodb_operation<T, F, Fut>(
    table_name: &str,
    operation_name: &str,
    operation: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
{
    let start_time = std::time::Instant::now();
    let result = operation().await;
    let duration = start_time.elapsed();

    match &result {
        Ok(_) => {
            info!(
                table = table_name,
                operation = operation_name,
                duration_ms = duration.as_millis(),
                "DynamoDB operation completed successfully"
            );
        }
        Err(e) => {
            error!(
                table = table_name,
                operation = operation_name,
                duration_ms = duration.as_millis(),
                error = %e,
                "DynamoDB operation failed"
            );
        }
    }

    result
}

/// HTTP リクエスト/レスポンスをトレース
pub fn trace_http_request(
    method: &str,
    path: &str,
    status_code: u16,
    user_id: Option<&str>,
) -> tracing::Span {
    let span = tracing::span!(
        tracing::Level::INFO,
        "http_request",
        method = method,
        path = path,
        status_code = status_code,
        user_id = user_id
    );

    if status_code >= 400 {
        warn!(
            method = method,
            path = path,
            status_code = status_code,
            "HTTP request failed"
        );
    } else {
        info!(
            method = method,
            path = path,
            status_code = status_code,
            "HTTP request completed"
        );
    }

    span
}

/// カスタムメトリクスを記録（ログ出力のみ - 実際の送信は MetricsClient を使用）
pub fn record_custom_metric(name: &str, value: f64, attributes: HashMap<String, String>) {
    info!(
        metric_name = name,
        metric_value = value,
        ?attributes,
        "Custom metric recorded"
    );
}

/// パフォーマンス測定とメトリクス送信を行うヘルパー
pub struct PerformanceTracker {
    start_time: std::time::Instant,
    operation_name: String,
    context: HashMap<String, String>,
}

impl PerformanceTracker {
    /// 新しいパフォーマンストラッカーを開始
    pub fn start(operation_name: String, context: HashMap<String, String>) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            operation_name,
            context,
        }
    }

    /// 測定を終了し、メトリクスを送信
    pub async fn finish(self, metrics_client: &MetricsClient, success: bool) {
        let duration_ms = self.start_time.elapsed().as_millis() as f64;

        // パフォーマンスメトリクスを送信
        let mut dimensions = self.context;
        dimensions.insert("Operation".to_string(), self.operation_name.clone());
        dimensions.insert("Success".to_string(), success.to_string());

        let metric = crate::metrics::CustomMetric::duration_ms(
            "OperationDuration".to_string(),
            duration_ms,
            dimensions,
        );

        if let Err(e) = metrics_client.put_metrics_batch(vec![metric]).await {
            error!(
                operation = %self.operation_name,
                duration_ms = duration_ms,
                error = %e,
                "Failed to send performance metric"
            );
        } else {
            info!(
                operation = %self.operation_name,
                duration_ms = duration_ms,
                success = success,
                "Performance metric sent"
            );
        }
    }
}

/// エラーをトレースに記録
pub fn record_error(error: &dyn std::error::Error, context: &str) {
    let current_span = tracing::Span::current();
    current_span.record("error", tracing::field::display(error));
    current_span.record("error.context", context);

    error!(
        error = %error,
        context = context,
        "Error recorded in trace"
    );
}

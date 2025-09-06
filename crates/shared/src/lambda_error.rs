use lambda_runtime::LambdaEvent;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

use crate::errors::{AppError, ErrorResponse};

/// Lambda関数用のエラーハンドリングユーティリティ
pub struct LambdaErrorHandler;

impl LambdaErrorHandler {
    /// AppErrorをAPI Gateway用のレスポンスに変換
    pub fn to_api_gateway_response(
        error: &AppError,
        request_id: Option<String>,
        include_details: bool,
    ) -> lambda_runtime::Error {
        let request_id = request_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let error_response = ErrorResponse::from_app_error(error, request_id, include_details);

        let status_code = error.http_status_code();
        let body = error_response.to_json().unwrap_or_else(|_| {
            r#"{"code":"SERIALIZATION_ERROR","message":"エラーレスポンスの生成に失敗しました"}"#
                .to_string()
        });

        // API Gateway用のエラーレスポンス構造を作成
        let response_body = serde_json::json!({
            "statusCode": status_code,
            "headers": {
                "Content-Type": "application/json",
                "X-Request-ID": error_response.request_id
            },
            "body": body,
            "isBase64Encoded": false
        });

        lambda_runtime::Error::from(response_body.to_string())
    }

    /// DynamoDB Streams用のエラーハンドリング
    pub fn handle_stream_error(
        error: &AppError,
        record_sequence_number: Option<String>,
    ) -> lambda_runtime::Error {
        let metadata = error.metadata();

        // リトライ可能なエラーの場合は、バッチアイテム失敗として返す
        if metadata.retryable {
            let batch_failure = serde_json::json!({
                "batchItemFailures": [{
                    "itemIdentifier": record_sequence_number.unwrap_or_default()
                }]
            });
            lambda_runtime::Error::from(batch_failure.to_string())
        } else {
            // リトライ不可能なエラーは通常のエラーとして返す
            lambda_runtime::Error::from(error.to_string())
        }
    }

    /// EventBridge用のエラーハンドリング
    pub fn handle_eventbridge_error(error: &AppError) -> lambda_runtime::Error {
        let error_response = ErrorResponse::from_app_error(
            error,
            Uuid::new_v4().to_string(),
            true, // EventBridgeでは詳細情報を含める
        );

        lambda_runtime::Error::from(
            error_response
                .to_json()
                .unwrap_or_else(|_| error.to_string()),
        )
    }

    /// リクエストIDを抽出
    pub fn extract_request_id<T>(event: &LambdaEvent<T>) -> Option<String> {
        // Lambda contextからリクエストIDを取得
        Some(event.context.request_id.clone())
    }

    /// API Gateway eventからリクエストIDを抽出
    pub fn extract_api_gateway_request_id(event: &Value) -> Option<String> {
        event
            .get("requestContext")
            .and_then(|ctx| ctx.get("requestId"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string())
    }

    /// エラーをログに記録
    pub fn log_error(error: &AppError, context: Option<HashMap<String, String>>) {
        let metadata = error.metadata();

        match metadata.severity {
            crate::errors::ErrorSeverity::Critical => {
                tracing::error!(
                    error = %error,
                    code = %metadata.code,
                    category = ?metadata.category,
                    retryable = metadata.retryable,
                    context = ?context,
                    "Critical error occurred"
                );
            }
            crate::errors::ErrorSeverity::Error => {
                tracing::error!(
                    error = %error,
                    code = %metadata.code,
                    category = ?metadata.category,
                    retryable = metadata.retryable,
                    context = ?context,
                    "Error occurred"
                );
            }
            crate::errors::ErrorSeverity::Warning => {
                tracing::warn!(
                    error = %error,
                    code = %metadata.code,
                    category = ?metadata.category,
                    retryable = metadata.retryable,
                    context = ?context,
                    "Warning occurred"
                );
            }
            crate::errors::ErrorSeverity::Info => {
                tracing::info!(
                    error = %error,
                    code = %metadata.code,
                    category = ?metadata.category,
                    retryable = metadata.retryable,
                    context = ?context,
                    "Info level error occurred"
                );
            }
        }
    }
}

/// Lambda関数のエラーハンドリングマクロ
#[macro_export]
macro_rules! handle_lambda_error {
    ($result:expr, $event:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                let request_id = $crate::lambda_error::LambdaErrorHandler::extract_request_id(&$event);
                $crate::lambda_error::LambdaErrorHandler::log_error(&error, None);

                return Err($crate::lambda_error::LambdaErrorHandler::to_api_gateway_response(
                    &error,
                    request_id,
                    cfg!(debug_assertions), // デバッグビルドでのみ詳細情報を含める
                ));
            }
        }
    };

    ($result:expr, $event:expr, $context:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                let request_id = $crate::lambda_error::LambdaErrorHandler::extract_request_id(&$event);
                $crate::lambda_error::LambdaErrorHandler::log_error(&error, Some($context));

                return Err($crate::lambda_error::LambdaErrorHandler::to_api_gateway_response(
                    &error,
                    request_id,
                    cfg!(debug_assertions),
                ));
            }
        }
    };
}

/// DynamoDB Streams用のエラーハンドリングマクロ
#[macro_export]
macro_rules! handle_stream_error {
    ($result:expr, $record:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                $crate::lambda_error::LambdaErrorHandler::log_error(&error, None);

                return Err(
                    $crate::lambda_error::LambdaErrorHandler::handle_stream_error(
                        &error,
                        $record.dynamodb.sequence_number.clone(),
                    ),
                );
            }
        }
    };
}

/// EventBridge用のエラーハンドリングマクロ
#[macro_export]
macro_rules! handle_eventbridge_error {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                $crate::lambda_error::LambdaErrorHandler::log_error(&error, None);

                return Err(
                    $crate::lambda_error::LambdaErrorHandler::handle_eventbridge_error(&error),
                );
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_gateway_response_creation() {
        let error = AppError::NotFound("Todo not found".to_string());
        let lambda_error = LambdaErrorHandler::to_api_gateway_response(
            &error,
            Some("test-request-id".to_string()),
            false,
        );

        let error_str = lambda_error.to_string();
        assert!(error_str.contains("404"));
        assert!(error_str.contains("NOT_FOUND"));
        assert!(error_str.contains("test-request-id"));
    }

    #[test]
    fn test_stream_error_handling_retryable() {
        let error = AppError::ConcurrentModification;
        let lambda_error =
            LambdaErrorHandler::handle_stream_error(&error, Some("12345".to_string()));

        let error_str = lambda_error.to_string();
        assert!(error_str.contains("batchItemFailures"));
        assert!(error_str.contains("12345"));
    }

    #[test]
    fn test_stream_error_handling_non_retryable() {
        let error = AppError::Validation("Invalid data".to_string());
        let lambda_error = LambdaErrorHandler::handle_stream_error(&error, None);

        let error_str = lambda_error.to_string();
        assert!(!error_str.contains("batchItemFailures"));
    }

    #[test]
    fn test_extract_api_gateway_request_id() {
        let event = serde_json::json!({
            "requestContext": {
                "requestId": "test-request-123"
            }
        });

        let request_id = LambdaErrorHandler::extract_api_gateway_request_id(&event);
        assert_eq!(request_id, Some("test-request-123".to_string()));
    }
}

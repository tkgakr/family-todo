use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// アプリケーション全体で使用される包括的なエラー型
#[derive(Debug, Clone, Error)]
pub enum AppError {
    // ドメインエラー
    #[error("Domain error: {0}")]
    Domain(#[from] domain::DomainError),

    // インフラストラクチャエラー
    #[error("DynamoDB error: {0}")]
    DynamoDb(String),

    #[error("AWS SDK error: {0}")]
    AwsSdk(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    // 認証・認可エラー
    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Authorization failed: {0}")]
    Authorization(String),

    #[error("Invalid JWT token: {0}")]
    InvalidJwt(String),

    #[error("Token expired")]
    TokenExpired,

    // ビジネスロジックエラー
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    #[error("Concurrent modification detected")]
    ConcurrentModification,

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Business rule violation: {0}")]
    BusinessRule(String),

    // システムエラー
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Timeout occurred: {0}")]
    Timeout(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    // 外部サービスエラー
    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Network error: {0}")]
    Network(String),
}

/// エラーの分類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// クライアントエラー（4xx相当）
    Client,
    /// サーバーエラー（5xx相当）
    Server,
    /// 一時的なエラー（リトライ可能）
    Transient,
    /// 永続的なエラー（リトライ不可）
    Permanent,
}

/// エラーの重要度
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// 情報レベル
    Info,
    /// 警告レベル
    Warning,
    /// エラーレベル
    Error,
    /// 致命的エラー
    Critical,
}

/// リトライ戦略
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    /// 最大リトライ回数
    pub max_attempts: u32,
    /// 初期遅延時間
    pub initial_delay: Duration,
    /// 最大遅延時間
    pub max_delay: Duration,
    /// バックオフ倍率
    pub backoff_multiplier: f64,
    /// ジッター追加フラグ
    pub add_jitter: bool,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            add_jitter: true,
        }
    }
}

/// エラーメタデータ
#[derive(Debug, Clone)]
pub struct ErrorMetadata {
    /// エラーコード
    pub code: String,
    /// エラー分類
    pub category: ErrorCategory,
    /// エラー重要度
    pub severity: ErrorSeverity,
    /// リトライ可能フラグ
    pub retryable: bool,
    /// リトライ戦略
    pub retry_strategy: Option<RetryStrategy>,
    /// 追加コンテキスト
    pub context: std::collections::HashMap<String, String>,
}

impl AppError {
    /// エラーメタデータを取得
    pub fn metadata(&self) -> ErrorMetadata {
        match self {
            // ドメインエラー
            AppError::Domain(_) => ErrorMetadata {
                code: "DOMAIN_ERROR".to_string(),
                category: ErrorCategory::Client,
                severity: ErrorSeverity::Error,
                retryable: false,
                retry_strategy: None,
                context: std::collections::HashMap::new(),
            },

            // DynamoDBエラー
            AppError::DynamoDb(msg) => {
                let retryable = is_dynamodb_retryable(msg);
                ErrorMetadata {
                    code: "DYNAMODB_ERROR".to_string(),
                    category: if retryable {
                        ErrorCategory::Transient
                    } else {
                        ErrorCategory::Server
                    },
                    severity: ErrorSeverity::Error,
                    retryable,
                    retry_strategy: if retryable {
                        Some(RetryStrategy::default())
                    } else {
                        None
                    },
                    context: std::collections::HashMap::new(),
                }
            }

            // 認証エラー
            AppError::Authentication(_) => ErrorMetadata {
                code: "AUTHENTICATION_ERROR".to_string(),
                category: ErrorCategory::Client,
                severity: ErrorSeverity::Warning,
                retryable: false,
                retry_strategy: None,
                context: std::collections::HashMap::new(),
            },

            AppError::Authorization(_) => ErrorMetadata {
                code: "AUTHORIZATION_ERROR".to_string(),
                category: ErrorCategory::Client,
                severity: ErrorSeverity::Warning,
                retryable: false,
                retry_strategy: None,
                context: std::collections::HashMap::new(),
            },

            // リソースエラー
            AppError::NotFound(_) => ErrorMetadata {
                code: "NOT_FOUND".to_string(),
                category: ErrorCategory::Client,
                severity: ErrorSeverity::Info,
                retryable: false,
                retry_strategy: None,
                context: std::collections::HashMap::new(),
            },

            AppError::AlreadyExists(_) => ErrorMetadata {
                code: "ALREADY_EXISTS".to_string(),
                category: ErrorCategory::Client,
                severity: ErrorSeverity::Info,
                retryable: false,
                retry_strategy: None,
                context: std::collections::HashMap::new(),
            },

            AppError::ConcurrentModification => ErrorMetadata {
                code: "CONCURRENT_MODIFICATION".to_string(),
                category: ErrorCategory::Transient,
                severity: ErrorSeverity::Warning,
                retryable: true,
                retry_strategy: Some(RetryStrategy {
                    max_attempts: 5,
                    initial_delay: Duration::from_millis(50),
                    max_delay: Duration::from_secs(5),
                    backoff_multiplier: 1.5,
                    add_jitter: true,
                }),
                context: std::collections::HashMap::new(),
            },

            // バリデーションエラー
            AppError::Validation(_) => ErrorMetadata {
                code: "VALIDATION_ERROR".to_string(),
                category: ErrorCategory::Client,
                severity: ErrorSeverity::Info,
                retryable: false,
                retry_strategy: None,
                context: std::collections::HashMap::new(),
            },

            // システムエラー
            AppError::ServiceUnavailable(_) => ErrorMetadata {
                code: "SERVICE_UNAVAILABLE".to_string(),
                category: ErrorCategory::Transient,
                severity: ErrorSeverity::Error,
                retryable: true,
                retry_strategy: Some(RetryStrategy::default()),
                context: std::collections::HashMap::new(),
            },

            AppError::RateLimitExceeded => ErrorMetadata {
                code: "RATE_LIMIT_EXCEEDED".to_string(),
                category: ErrorCategory::Transient,
                severity: ErrorSeverity::Warning,
                retryable: true,
                retry_strategy: Some(RetryStrategy {
                    max_attempts: 3,
                    initial_delay: Duration::from_secs(1),
                    max_delay: Duration::from_secs(60),
                    backoff_multiplier: 2.0,
                    add_jitter: true,
                }),
                context: std::collections::HashMap::new(),
            },

            AppError::Timeout(_) => ErrorMetadata {
                code: "TIMEOUT".to_string(),
                category: ErrorCategory::Transient,
                severity: ErrorSeverity::Error,
                retryable: true,
                retry_strategy: Some(RetryStrategy::default()),
                context: std::collections::HashMap::new(),
            },

            AppError::Internal(_) => ErrorMetadata {
                code: "INTERNAL_ERROR".to_string(),
                category: ErrorCategory::Server,
                severity: ErrorSeverity::Critical,
                retryable: false,
                retry_strategy: None,
                context: std::collections::HashMap::new(),
            },

            // その他のエラー
            _ => ErrorMetadata {
                code: "UNKNOWN_ERROR".to_string(),
                category: ErrorCategory::Server,
                severity: ErrorSeverity::Error,
                retryable: false,
                retry_strategy: None,
                context: std::collections::HashMap::new(),
            },
        }
    }

    /// HTTPステータスコードを取得
    pub fn http_status_code(&self) -> u16 {
        match self.metadata().category {
            ErrorCategory::Client => match self {
                AppError::NotFound(_) => 404,
                AppError::Authentication(_) | AppError::InvalidJwt(_) | AppError::TokenExpired => {
                    401
                }
                AppError::Authorization(_) => 403,
                AppError::AlreadyExists(_) => 409,
                AppError::Validation(_) => 400,
                AppError::RateLimitExceeded => 429,
                _ => 400,
            },
            ErrorCategory::Server | ErrorCategory::Permanent => 500,
            ErrorCategory::Transient => match self {
                AppError::ServiceUnavailable(_) => 503,
                AppError::Timeout(_) => 504,
                _ => 500,
            },
        }
    }

    /// ユーザー向けメッセージを取得
    pub fn user_message(&self) -> String {
        match self {
            AppError::NotFound(_) => "リソースが見つかりません".to_string(),
            AppError::Authentication(_) => "認証に失敗しました".to_string(),
            AppError::Authorization(_) => "この操作を実行する権限がありません".to_string(),
            AppError::Validation(_) => "入力データが無効です".to_string(),
            AppError::ConcurrentModification => {
                "他のユーザーによって変更されました。再度お試しください".to_string()
            }
            AppError::RateLimitExceeded => {
                "リクエストが多すぎます。しばらく待ってから再度お試しください".to_string()
            }
            AppError::ServiceUnavailable(_) => "サービスが一時的に利用できません".to_string(),
            AppError::Timeout(_) => "処理がタイムアウトしました".to_string(),
            _ => "予期しないエラーが発生しました".to_string(),
        }
    }
}

/// DynamoDBエラーがリトライ可能かどうかを判定
fn is_dynamodb_retryable(error_message: &str) -> bool {
    let retryable_errors = [
        "ThrottlingException",
        "ProvisionedThroughputExceededException",
        "ServiceUnavailable",
        "InternalServerError",
        "RequestLimitExceeded",
        "ItemCollectionSizeLimitExceededException",
    ];

    retryable_errors
        .iter()
        .any(|&err| error_message.contains(err))
}

/// 標準化されたエラーレスポンス
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// エラーコード
    pub code: String,
    /// ユーザー向けメッセージ
    pub message: String,
    /// 詳細情報（開発環境のみ）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// リクエストID
    pub request_id: String,
    /// タイムスタンプ
    pub timestamp: String,
    /// 追加コンテキスト
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub context: std::collections::HashMap<String, String>,
}

impl ErrorResponse {
    /// AppErrorからErrorResponseを作成
    pub fn from_app_error(error: &AppError, request_id: String, include_details: bool) -> Self {
        let metadata = error.metadata();

        Self {
            code: metadata.code,
            message: error.user_message(),
            details: if include_details {
                Some(error.to_string())
            } else {
                None
            },
            request_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: metadata.context,
        }
    }

    /// JSONレスポンスとして返すためのシリアライズ
    pub fn to_json(&self) -> Result<String, AppError> {
        serde_json::to_string(self).map_err(|e| AppError::Serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_metadata() {
        let error = AppError::NotFound("test".to_string());
        let metadata = error.metadata();

        assert_eq!(metadata.code, "NOT_FOUND");
        assert_eq!(metadata.category, ErrorCategory::Client);
        assert!(!metadata.retryable);
    }

    #[test]
    fn test_concurrent_modification_retryable() {
        let error = AppError::ConcurrentModification;
        let metadata = error.metadata();

        assert_eq!(metadata.code, "CONCURRENT_MODIFICATION");
        assert_eq!(metadata.category, ErrorCategory::Transient);
        assert!(metadata.retryable);
        assert!(metadata.retry_strategy.is_some());
    }

    #[test]
    fn test_http_status_codes() {
        assert_eq!(
            AppError::NotFound("test".to_string()).http_status_code(),
            404
        );
        assert_eq!(
            AppError::Authentication("test".to_string()).http_status_code(),
            401
        );
        assert_eq!(
            AppError::Authorization("test".to_string()).http_status_code(),
            403
        );
        assert_eq!(
            AppError::Validation("test".to_string()).http_status_code(),
            400
        );
        assert_eq!(
            AppError::Internal("test".to_string()).http_status_code(),
            500
        );
    }

    #[test]
    fn test_error_response_creation() {
        let error = AppError::NotFound("Todo not found".to_string());
        let response = ErrorResponse::from_app_error(&error, "req-123".to_string(), false);

        assert_eq!(response.code, "NOT_FOUND");
        assert_eq!(response.message, "リソースが見つかりません");
        assert_eq!(response.request_id, "req-123");
        assert!(response.details.is_none());
    }

    #[test]
    fn test_dynamodb_retryable_detection() {
        assert!(is_dynamodb_retryable("ThrottlingException: Rate exceeded"));
        assert!(is_dynamodb_retryable("ServiceUnavailable"));
        assert!(!is_dynamodb_retryable("ValidationException: Invalid input"));
        assert!(!is_dynamodb_retryable("ConditionalCheckFailedException"));
    }
}

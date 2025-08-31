use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub dynamodb_table: String,
    pub environment: String,
    pub aws_region: String,
    pub dynamodb_endpoint: Option<String>,
    pub retry_max_attempts: u32,
    pub retry_initial_delay_ms: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Config {
            dynamodb_table: env::var("DYNAMODB_TABLE")
                .unwrap_or_else(|_| "family-todo-dev".to_string()),
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()),
            aws_region: env::var("AWS_REGION").unwrap_or_else(|_| "ap-northeast-1".to_string()),
            dynamodb_endpoint: env::var("DYNAMODB_ENDPOINT").ok(),
            retry_max_attempts: env::var("RETRY_MAX_ATTEMPTS")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
            retry_initial_delay_ms: env::var("RETRY_INITIAL_DELAY_MS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .unwrap_or(100),
        })
    }

    /// テスト用設定を作成
    #[cfg(test)]
    pub fn for_test() -> Self {
        Self {
            dynamodb_table: "test-table".to_string(),
            environment: "test".to_string(),
            aws_region: "ap-northeast-1".to_string(),
            dynamodb_endpoint: Some("http://localhost:8000".to_string()),
            retry_max_attempts: 2,
            retry_initial_delay_ms: 10,
        }
    }
}

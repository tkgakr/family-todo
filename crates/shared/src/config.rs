use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub dynamodb_table: String,
    pub environment: String,
    pub aws_region: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Config {
            dynamodb_table: env::var("DYNAMODB_TABLE")
                .unwrap_or_else(|_| "family-todo-dev".to_string()),
            environment: env::var("ENVIRONMENT")
                .unwrap_or_else(|_| "dev".to_string()),
            aws_region: env::var("AWS_REGION")
                .unwrap_or_else(|_| "ap-northeast-1".to_string()),
        })
    }
}
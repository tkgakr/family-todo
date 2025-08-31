use std::time::Duration;
use tokio::time::sleep;
use tracing::{warn, debug};
use domain::errors::TodoError;

/// リトライ設定
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大リトライ回数
    pub max_attempts: u32,
    /// 初期待機時間（ミリ秒）
    pub initial_delay_ms: u64,
    /// 指数バックオフの倍率
    pub backoff_multiplier: f64,
    /// 最大待機時間（ミリ秒）
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            backoff_multiplier: 2.0,
            max_delay_ms: 5000,
        }
    }
}

/// 指数バックオフによるリトライ実行
/// リトライ可能なエラーに対してのみリトライを実行
pub async fn retry_with_backoff<F, Fut, T, E>(
    operation: F,
    config: &RetryConfig,
    is_retryable: impl Fn(&E) -> bool,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay_ms;

    loop {
        attempt += 1;
        
        debug!("操作実行中... 試行回数: {}/{}", attempt, config.max_attempts);
        
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!("操作が {} 回目の試行で成功", attempt);
                }
                return Ok(result);
            }
            Err(error) => {
                if attempt >= config.max_attempts {
                    warn!("最大リトライ回数 {} に達しました。エラー: {}", 
                          config.max_attempts, error);
                    return Err(error);
                }

                if !is_retryable(&error) {
                    debug!("リトライ不可能なエラー: {}", error);
                    return Err(error);
                }

                warn!("リトライ可能なエラーが発生。{}ms 後に再試行します。エラー: {}", 
                      delay, error);
                
                sleep(Duration::from_millis(delay)).await;
                
                // 指数バックオフで待機時間を増加
                delay = ((delay as f64) * config.backoff_multiplier) as u64;
                delay = delay.min(config.max_delay_ms);
            }
        }
    }
}

/// DynamoDB 操作用のリトライヘルパー
pub async fn retry_dynamodb_operation<F, Fut, T>(
    operation: F,
    config: Option<&RetryConfig>,
) -> Result<T, TodoError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, TodoError>>,
{
    let default_config = RetryConfig::default();
    let config = config.unwrap_or(&default_config);
    
    retry_with_backoff(
        operation,
        config,
        |error| matches!(error, 
            TodoError::DynamoDb(msg) if msg.contains("スロットリング") || 
                                       msg.contains("一時的に利用できません")
        ),
    ).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 10,
            backoff_multiplier: 2.0,
            max_delay_ms: 100,
        };

        let result = retry_with_backoff(
            || {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count == 0 {
                        Err("一時的なエラー")
                    } else {
                        Ok("成功")
                    }
                }
            },
            &config,
            |_| true, // すべてのエラーをリトライ可能とする
        ).await;

        assert_eq!(result, Ok("成功"));
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_fails_after_max_attempts() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let config = RetryConfig {
            max_attempts: 2,
            initial_delay_ms: 10,
            backoff_multiplier: 2.0,
            max_delay_ms: 100,
        };

        let result: Result<&str, &str> = retry_with_backoff(
            || {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err("常にエラー")
                }
            },
            &config,
            |_| true,
        ).await;

        assert_eq!(result, Err("常にエラー"));
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let config = RetryConfig::default();

        let result: Result<&str, &str> = retry_with_backoff(
            || {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err("リトライ不可能なエラー")
                }
            },
            &config,
            |_| false, // すべてのエラーをリトライ不可能とする
        ).await;

        assert_eq!(result, Err("リトライ不可能なエラー"));
        assert_eq!(counter.load(Ordering::SeqCst), 1); // 1回のみ実行
    }
}
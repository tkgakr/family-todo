use rand::Rng;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, warn};

use crate::errors::{AppError, RetryStrategy};

/// リトライ実行結果
#[derive(Debug)]
pub enum RetryResult<T> {
    /// 成功
    Success(T),
    /// 最大試行回数に達して失敗
    MaxAttemptsReached(AppError),
    /// リトライ不可能なエラーで失敗
    NonRetryable(AppError),
}

/// リトライ実行器
pub struct RetryExecutor {
    strategy: RetryStrategy,
}

impl RetryExecutor {
    /// 新しいリトライ実行器を作成
    pub fn new(strategy: RetryStrategy) -> Self {
        Self { strategy }
    }
}

impl Default for RetryExecutor {
    fn default() -> Self {
        Self::new(RetryStrategy::default())
    }
}

impl RetryExecutor {
    /// 指数バックオフ設定でリトライ実行器を作成
    pub fn exponential_backoff(max_attempts: u32, initial_delay: Duration) -> Self {
        Self::new(RetryStrategy {
            max_attempts,
            initial_delay,
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            add_jitter: true,
        })
    }

    /// 固定間隔設定でリトライ実行器を作成
    pub fn fixed_interval(max_attempts: u32, interval: Duration) -> Self {
        Self::new(RetryStrategy {
            max_attempts,
            initial_delay: interval,
            max_delay: interval,
            backoff_multiplier: 1.0,
            add_jitter: false,
        })
    }

    /// 操作をリトライ付きで実行
    pub async fn execute<F, Fut, T>(&self, operation: F) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        let mut attempt = 1;
        let mut _last_error = None;

        loop {
            debug!(
                "Executing operation, attempt {}/{}",
                attempt, self.strategy.max_attempts
            );

            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        debug!("Operation succeeded after {} attempts", attempt);
                    }
                    return RetryResult::Success(result);
                }
                Err(error) => {
                    _last_error = Some(error.clone());

                    // エラーメタデータを取得してリトライ可能性を判定
                    let metadata = error.metadata();

                    if !metadata.retryable {
                        warn!("Non-retryable error encountered: {}", error);
                        return RetryResult::NonRetryable(error);
                    }

                    if attempt >= self.strategy.max_attempts {
                        error!(
                            "Max attempts ({}) reached, giving up",
                            self.strategy.max_attempts
                        );
                        return RetryResult::MaxAttemptsReached(error);
                    }

                    // 遅延時間を計算
                    let delay = self.calculate_delay(attempt);
                    warn!(
                        "Operation failed (attempt {}/{}), retrying in {:?}: {}",
                        attempt, self.strategy.max_attempts, delay, error
                    );

                    // 遅延実行
                    sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }

    /// 遅延時間を計算
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = if self.strategy.backoff_multiplier == 1.0 {
            // 固定間隔
            self.strategy.initial_delay
        } else {
            // 指数バックオフ
            let multiplier = self.strategy.backoff_multiplier.powi((attempt - 1) as i32);
            Duration::from_millis(
                (self.strategy.initial_delay.as_millis() as f64 * multiplier) as u64,
            )
        };

        // 最大遅延時間でクランプ
        let delay = std::cmp::min(base_delay, self.strategy.max_delay);

        // ジッターを追加
        if self.strategy.add_jitter {
            self.add_jitter(delay)
        } else {
            delay
        }
    }

    /// ジッターを追加（±25%のランダム変動）
    fn add_jitter(&self, delay: Duration) -> Duration {
        let mut rng = rand::thread_rng();
        let jitter_factor = rng.gen_range(0.75..=1.25);
        Duration::from_millis((delay.as_millis() as f64 * jitter_factor) as u64)
    }
}

/// 便利な関数：デフォルト設定でリトライ実行
pub async fn retry_with_default<F, Fut, T>(operation: F) -> RetryResult<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    RetryExecutor::default().execute(operation).await
}

/// 便利な関数：指数バックオフでリトライ実行
pub async fn retry_with_exponential_backoff<F, Fut, T>(
    operation: F,
    max_attempts: u32,
    initial_delay: Duration,
) -> RetryResult<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    RetryExecutor::exponential_backoff(max_attempts, initial_delay)
        .execute(operation)
        .await
}

/// 便利な関数：固定間隔でリトライ実行
pub async fn retry_with_fixed_interval<F, Fut, T>(
    operation: F,
    max_attempts: u32,
    interval: Duration,
) -> RetryResult<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    RetryExecutor::fixed_interval(max_attempts, interval)
        .execute(operation)
        .await
}

/// DynamoDB操作専用のリトライ実行器
pub struct DynamoDbRetryExecutor;

impl DynamoDbRetryExecutor {
    /// DynamoDB操作をリトライ付きで実行
    pub async fn execute<F, Fut, T>(operation: F) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        let strategy = RetryStrategy {
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            add_jitter: true,
        };

        RetryExecutor::new(strategy).execute(operation).await
    }
}

/// 楽観的ロック専用のリトライ実行器
pub struct OptimisticLockRetryExecutor;

impl OptimisticLockRetryExecutor {
    /// 楽観的ロック操作をリトライ付きで実行
    pub async fn execute<F, Fut, T>(operation: F) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        let strategy = RetryStrategy {
            max_attempts: 10,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 1.5,
            add_jitter: true,
        };

        RetryExecutor::new(strategy).execute(operation).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let executor = RetryExecutor::default();
        let result = executor.execute(|| async { Ok::<i32, AppError>(42) }).await;

        match result {
            RetryResult::Success(value) => assert_eq!(value, 42),
            _ => panic!("Expected success"),
        }
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let executor = RetryExecutor::default();
        let result = executor
            .execute(|| {
                let count = attempt_count_clone.clone();
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                    if current < 3 {
                        Err(AppError::ServiceUnavailable(
                            "Temporary failure".to_string(),
                        ))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        match result {
            RetryResult::Success(value) => {
                assert_eq!(value, 42);
                assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
            }
            _ => panic!("Expected success after retries"),
        }
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let executor = RetryExecutor::default();
        let result = executor
            .execute(|| async {
                Err::<i32, AppError>(AppError::Validation("Invalid input".to_string()))
            })
            .await;

        match result {
            RetryResult::NonRetryable(_) => {}
            _ => panic!("Expected non-retryable error"),
        }
    }

    #[tokio::test]
    async fn test_max_attempts_reached() {
        let executor = RetryExecutor::new(RetryStrategy {
            max_attempts: 2,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(1),
            backoff_multiplier: 1.0,
            add_jitter: false,
        });

        let result = executor
            .execute(|| async {
                Err::<i32, AppError>(AppError::ServiceUnavailable("Always fails".to_string()))
            })
            .await;

        match result {
            RetryResult::MaxAttemptsReached(_) => {}
            _ => panic!("Expected max attempts reached"),
        }
    }

    #[test]
    fn test_delay_calculation() {
        let _executor = RetryExecutor::exponential_backoff(5, Duration::from_millis(100));

        // 指数バックオフのテスト（ジッターなし）
        let executor_no_jitter = RetryExecutor::new(RetryStrategy {
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            add_jitter: false,
        });

        assert_eq!(
            executor_no_jitter.calculate_delay(1),
            Duration::from_millis(100)
        );
        assert_eq!(
            executor_no_jitter.calculate_delay(2),
            Duration::from_millis(200)
        );
        assert_eq!(
            executor_no_jitter.calculate_delay(3),
            Duration::from_millis(400)
        );
    }
}

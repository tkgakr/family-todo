use aws_config::{BehaviorVersion, Region};
use aws_sdk_dynamodb::Client;
use shared::{AppError, Config};
use std::sync::Arc;
use tracing::{error, info, warn};

/// DynamoDB クライアントのラッパー
/// コネクション管理、エラーハンドリング、設定管理を提供
#[derive(Debug, Clone)]
pub struct DynamoDbClient {
    client: Arc<Client>,
    table_name: String,
    region: String,
}

impl DynamoDbClient {
    /// 新しい DynamoDB クライアントを作成
    /// AWS 設定を自動的に読み込み、適切なリージョンを設定
    pub async fn new(config: &Config) -> Result<Self, AppError> {
        info!(
            "DynamoDB クライアントを初期化中... region: {}, table: {}",
            config.aws_region, config.dynamodb_table
        );

        let mut aws_config_builder = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.aws_region.clone()));

        // ローカル開発用のエンドポイント設定
        if let Some(endpoint) = &config.dynamodb_endpoint {
            info!("カスタム DynamoDB エンドポイントを使用: {}", endpoint);
            aws_config_builder = aws_config_builder.endpoint_url(endpoint);
        }

        let aws_config = aws_config_builder.load().await;
        let client = Client::new(&aws_config);

        // 接続テスト（テーブルの存在確認）
        match client
            .describe_table()
            .table_name(&config.dynamodb_table)
            .send()
            .await
        {
            Ok(_) => {
                info!(
                    "DynamoDB テーブル '{}' への接続を確認",
                    config.dynamodb_table
                );
            }
            Err(e) => {
                warn!(
                    "DynamoDB テーブル '{}' への接続確認に失敗: {}",
                    config.dynamodb_table, e
                );
                // 開発環境では警告のみ、本番環境ではエラーとして扱う可能性
                if config.environment == "prod" {
                    return Err(AppError::DynamoDb(format!("テーブル接続エラー: {e}")));
                }
            }
        }

        Ok(Self {
            client: Arc::new(client),
            table_name: config.dynamodb_table.clone(),
            region: config.aws_region.clone(),
        })
    }

    /// テスト用のモッククライアントを作成
    #[cfg(test)]
    pub async fn new_for_test(config: &Config) -> Result<Self, AppError> {
        use aws_sdk_dynamodb::config::Builder;

        let aws_config = Builder::new()
            .endpoint_url(
                config
                    .dynamodb_endpoint
                    .as_ref()
                    .unwrap_or(&"http://localhost:8000".to_string()),
            )
            .region(Region::new(config.aws_region.clone()))
            .behavior_version(BehaviorVersion::latest())
            .build();

        let client = Client::from_conf(aws_config);

        Ok(Self {
            client: Arc::new(client),
            table_name: config.dynamodb_table.clone(),
            region: config.aws_region.clone(),
        })
    }

    /// DynamoDB クライアントへの参照を取得
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// テーブル名を取得
    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    /// リージョン名を取得
    pub fn region(&self) -> &str {
        &self.region
    }

    /// DynamoDB エラーを AppError に変換
    pub fn convert_error(&self, error: impl std::fmt::Display) -> AppError {
        let error_str = error.to_string();

        if error_str.contains("ConditionalCheckFailedException") {
            AppError::ConcurrentModification
        } else if error_str.contains("ResourceNotFoundException") {
            AppError::NotFound("リソースが見つかりません".to_string())
        } else if error_str.contains("ValidationException") {
            AppError::Validation(error_str)
        } else if error_str.contains("ThrottlingException") {
            AppError::DynamoDb("リクエストがスロットリングされました".to_string())
        } else if error_str.contains("ProvisionedThroughputExceededException") {
            AppError::DynamoDb("プロビジョニングされたスループットを超過しました".to_string())
        } else if error_str.contains("ServiceUnavailableException") {
            AppError::ServiceUnavailable("DynamoDBサービスが一時的に利用できません".to_string())
        } else if error_str.contains("InternalServerError") {
            AppError::Internal("DynamoDB内部サーバーエラー".to_string())
        } else if error_str.contains("RequestLimitExceeded") {
            AppError::RateLimitExceeded
        } else if error_str.contains("ItemCollectionSizeLimitExceededException") {
            AppError::DynamoDb("アイテムコレクションサイズ制限を超過しました".to_string())
        } else {
            error!("予期しない DynamoDB エラー: {}", error_str);
            AppError::DynamoDb(format!("DynamoDB エラー: {error_str}"))
        }
    }

    /// リトライ可能なエラーかどうかを判定（簡略版）
    pub fn is_retryable_error(&self, error: impl std::fmt::Display) -> bool {
        let error_str = error.to_string();
        error_str.contains("ThrottlingException")
            || error_str.contains("ServiceUnavailableException")
            || error_str.contains("InternalServerError")
            || error_str.contains("TimeoutError")
    }
}

/// コネクションプール管理
/// 複数の Lambda 関数間でクライアントインスタンスを効率的に共有
#[derive(Debug)]
pub struct DynamoDbConnectionPool {
    client: DynamoDbClient,
}

impl DynamoDbConnectionPool {
    /// 新しいコネクションプールを作成
    pub async fn new(config: &Config) -> Result<Self, AppError> {
        let client = DynamoDbClient::new(config).await?;

        info!("DynamoDB コネクションプールを初期化完了");

        Ok(Self { client })
    }

    /// クライアントを取得（クローンは軽量）
    pub fn get_client(&self) -> DynamoDbClient {
        self.client.clone()
    }

    /// ヘルスチェック - 接続状態を確認
    pub async fn health_check(&self) -> Result<(), AppError> {
        match self
            .client
            .client()
            .describe_table()
            .table_name(self.client.table_name())
            .send()
            .await
        {
            Ok(_) => {
                info!("DynamoDB ヘルスチェック成功");
                Ok(())
            }
            Err(e) => {
                error!("DynamoDB ヘルスチェック失敗: {}", e);
                Err(self.client.convert_error(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::Config;

    #[tokio::test]
    async fn test_dynamodb_client_creation() {
        let config = Config {
            dynamodb_table: "test-table".to_string(),
            environment: "test".to_string(),
            aws_region: "ap-northeast-1".to_string(),
            dynamodb_endpoint: Some("http://localhost:8000".to_string()),
            retry_max_attempts: 2,
            retry_initial_delay_ms: 10,
        };

        let client = DynamoDbClient::new_for_test(&config).await;
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.table_name(), "test-table");
        assert_eq!(client.region(), "ap-northeast-1");
    }

    #[tokio::test]
    async fn test_connection_pool_creation() {
        let config = Config {
            dynamodb_table: "test-table".to_string(),
            environment: "test".to_string(),
            aws_region: "ap-northeast-1".to_string(),
            dynamodb_endpoint: Some("http://localhost:8000".to_string()),
            retry_max_attempts: 2,
            retry_initial_delay_ms: 10,
        };

        // 実際のDynamoDB Localが動いていない場合はスキップ
        if let Ok(pool) = DynamoDbConnectionPool::new(&config).await {
            let client1 = pool.get_client();
            let client2 = pool.get_client();

            // クライアントは異なるインスタンスだが、同じ設定を持つ
            assert_eq!(client1.table_name(), client2.table_name());
        }
    }

    // エラー変換のテストは実際のDynamoDBエラーが必要なため、統合テストで実施
}

use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::{
    types::{
        AttributeDefinition, AttributeValue, BillingMode, GlobalSecondaryIndex, KeySchemaElement,
        KeyType, Projection, ProjectionType, ScalarAttributeType, StreamSpecification,
        StreamViewType,
    },
    Client as DynamoDbClient,
};
use std::env;

pub struct DynamoDbTestClient {
    pub client: DynamoDbClient,
    pub table_name: String,
}

impl DynamoDbTestClient {
    /// DynamoDB Local（docker-compose環境）に接続するクライアントを作成
    /// 環境変数 DYNAMODB_ENDPOINT を指定することで接続先をカスタマイズ可能
    pub async fn new() -> Result<Self> {
        let table_name = "FamilyTodoApp-MainTable-Test".to_string();
        
        // docker-compose.ymlのDynamoDB Localに接続
        let endpoint = env::var("DYNAMODB_ENDPOINT").unwrap_or_else(|_| "http://localhost:8000".to_string());
        
        let config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(&endpoint)
            .region("us-east-1")
            .credentials_provider(aws_sdk_dynamodb::config::SharedCredentialsProvider::new(
                aws_sdk_dynamodb::config::Credentials::new("test", "test", None, None, "test")
            ))
            .load()
            .await;
            
        let client = DynamoDbClient::new(&config);

        let instance = Self {
            client,
            table_name,
        };

        // テーブル作成（存在しない場合のみ）
        instance.ensure_table_exists().await?;

        Ok(instance)
    }

    /// テーブルが存在することを確認し、なければ作成
    pub async fn ensure_table_exists(&self) -> Result<()> {
        match self.verify_table_exists().await {
            Ok(true) => {
                // テーブル存在時はクリア
                self.clear_table().await?;
                Ok(())
            }
            Ok(false) | Err(_) => {
                // テーブルが存在しない場合は作成
                self.create_table().await
            }
        }
    }

    async fn create_table(&self) -> Result<()> {
        self.client
            .create_table()
            .table_name(&self.table_name)
            .billing_mode(BillingMode::PayPerRequest)
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("PK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()?,
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("SK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()?,
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("GSI1PK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()?,
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("GSI1SK")
                    .attribute_type(ScalarAttributeType::S)
                    .build()?,
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("PK")
                    .key_type(KeyType::Hash)
                    .build()?,
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("SK")
                    .key_type(KeyType::Range)
                    .build()?,
            )
            .global_secondary_indexes(
                GlobalSecondaryIndex::builder()
                    .index_name("GSI1")
                    .key_schema(
                        KeySchemaElement::builder()
                            .attribute_name("GSI1PK")
                            .key_type(KeyType::Hash)
                            .build()?,
                    )
                    .key_schema(
                        KeySchemaElement::builder()
                            .attribute_name("GSI1SK")
                            .key_type(KeyType::Range)
                            .build()?,
                    )
                    .projection(
                        Projection::builder()
                            .projection_type(ProjectionType::All)
                            .build(),
                    )
                    .build()?,
            )
            .stream_specification(
                StreamSpecification::builder()
                    .stream_enabled(true)
                    .stream_view_type(StreamViewType::NewAndOldImages)
                    .build()?,
            )
            .send()
            .await?;

        Ok(())
    }

    pub async fn clear_table(&self) -> Result<()> {
        let scan_output = self
            .client
            .scan()
            .table_name(&self.table_name)
            .send()
            .await?;

        if let Some(items) = scan_output.items {
            for item in items {
                if let (Some(AttributeValue::S(pk)), Some(AttributeValue::S(sk))) =
                    (item.get("PK"), item.get("SK"))
                {
                    self.client
                        .delete_item()
                        .table_name(&self.table_name)
                        .key("PK", AttributeValue::S(pk.clone()))
                        .key("SK", AttributeValue::S(sk.clone()))
                        .send()
                        .await?;
                }
            }
        }

        Ok(())
    }

    pub async fn verify_table_exists(&self) -> Result<bool> {
        match self
            .client
            .describe_table()
            .table_name(&self.table_name)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub async fn count_items(&self) -> Result<i32> {
        let scan_output = self
            .client
            .scan()
            .table_name(&self.table_name)
            .select(aws_sdk_dynamodb::types::Select::Count)
            .send()
            .await?;

        Ok(scan_output.count)
    }

    /// テスト用に設定されたDynamoDBリポジトリを作成
    pub async fn create_repository(&self) -> crate::DynamoDbRepository {
        crate::DynamoDbRepository::new(self.table_name.clone()).await
    }
}

// 共有リポジトリ型の再エクスポート
pub use shared::infra::dynamodb::DynamoDbRepository;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dynamodb_client_setup() -> Result<()> {
        // このテストはDynamoDB Localが起動していることを前提とする
        // docker-compose up dynamodb でローカル環境を起動してから実行
        match DynamoDbTestClient::new().await {
            Ok(client) => {
                assert!(client.verify_table_exists().await?);
                assert_eq!(client.count_items().await?, 0);
                Ok(())
            }
            Err(_) => {
                eprintln!("DynamoDB Local が起動していません。テストをスキップします。");
                eprintln!("docker-compose up dynamodb を実行してください。");
                Ok(()) // テストスキップ
            }
        }
    }
}
use aws_sdk_dynamodb::Client;
use shared::Config;

pub struct DynamoDbClient {
    client: Client,
    table_name: String,
}

impl DynamoDbClient {
    pub async fn new(config: &Config) -> Self {
        let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&aws_config);

        Self {
            client,
            table_name: config.dynamodb_table.clone(),
        }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

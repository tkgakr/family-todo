use lambda_http::{run, service_fn, Error, Request};
use tracing_subscriber::EnvFilter;

mod db;
mod error;
mod handlers;
mod models;
mod router;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let table_name =
        std::env::var("TABLE_NAME").unwrap_or_else(|_| "family-todo-table".to_string());
    let db_client = db::DynamoClient::new(&table_name).await;

    run(service_fn(move |req: Request| {
        let db = db_client.clone();
        async move { router::route(req, &db).await }
    }))
    .await
}

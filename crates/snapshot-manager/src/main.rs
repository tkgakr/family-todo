use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde_json::Value;
use shared::init_tracing;

async fn function_handler(_event: LambdaEvent<Value>) -> Result<Value, Error> {
    // Placeholder implementation
    Ok(serde_json::json!({
        "statusCode": 200,
        "body": "Snapshot manager placeholder"
    }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();
    
    run(service_fn(function_handler)).await
}
mod error_handling;
mod processor;

use aws_lambda_events::event::dynamodb::Event as DynamoDbEvent;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use shared::{init_telemetry, shutdown_telemetry};
use tracing::{error, info};

use error_handling::BatchItemFailures;
use processor::StreamProcessor;

async fn function_handler(event: LambdaEvent<DynamoDbEvent>) -> Result<BatchItemFailures, Error> {
    let (dynamodb_event, context) = event.into_parts();

    let span = tracing::Span::current();
    span.record("request_id", &context.request_id);
    span.record("function_name", &context.env_config.function_name);

    info!(
        record_count = dynamodb_event.records.len(),
        request_id = %context.request_id,
        "Processing DynamoDB stream records"
    );

    let table_name = std::env::var("TABLE_NAME").unwrap_or_else(|_| "MainTable".to_string());

    let processor = StreamProcessor::new(table_name);

    match processor.process_records(dynamodb_event.records).await {
        Ok(failures) => {
            if failures.batch_item_failures.is_empty() {
                info!(
                    request_id = %context.request_id,
                    "All records processed successfully"
                );
            } else {
                error!(
                    failure_count = failures.batch_item_failures.len(),
                    request_id = %context.request_id,
                    "Some records failed to process"
                );
            }
            Ok(failures)
        }
        Err(e) => {
            error!(
                error = %e,
                request_id = %context.request_id,
                "Failed to process stream records"
            );
            Err(Error::from(e.to_string()))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_telemetry().map_err(|e| {
        eprintln!("Failed to initialize telemetry: {e}");
        Error::from(e.to_string())
    })?;

    info!("Todo Event Processor starting...");

    let result = run(service_fn(function_handler)).await;

    shutdown_telemetry();

    result
}

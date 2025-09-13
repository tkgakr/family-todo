mod commands;
mod handlers;
mod responses;

use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use shared::{init_telemetry, shutdown_telemetry};
use tracing::{error, info};

use handlers::CommandHandler;

async fn function_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let (request, context) = event.into_parts();

    let span = tracing::Span::current();
    span.record("request_id", &context.request_id);
    span.record("function_name", &context.env_config.function_name);

    info!(
        method = ?request.http_method,
        path = ?request.path,
        request_id = %context.request_id,
        "Processing command request"
    );

    let table_name = std::env::var("TABLE_NAME").unwrap_or_else(|_| "MainTable".to_string());

    let handler = CommandHandler::new(table_name).await;

    match handler.handle_request(request).await {
        Ok(response) => {
            info!(
                status_code = response.status_code,
                request_id = %context.request_id,
                "Command processed successfully"
            );
            Ok(response)
        }
        Err(e) => {
            error!(
                error = %e,
                request_id = %context.request_id,
                "Failed to process command"
            );

            let error_response = ApiGatewayProxyResponse {
                status_code: 500,
                headers: Default::default(),
                multi_value_headers: Default::default(),
                body: Some(aws_lambda_events::encodings::Body::Text(
                    serde_json::json!({
                        "error": "Internal server error",
                        "request_id": context.request_id
                    })
                    .to_string(),
                )),
                is_base64_encoded: false,
            };
            Ok(error_response)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_telemetry().map_err(|e| {
        eprintln!("Failed to initialize telemetry: {e}");
        Error::from(e.to_string())
    })?;

    info!("Todo Command Handler starting...");

    let result = run(service_fn(function_handler)).await;

    shutdown_telemetry();

    result
}

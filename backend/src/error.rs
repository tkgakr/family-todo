use lambda_http::{Body, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Not found")]
    NotFound,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl ApiError {
    pub fn into_response(self) -> Response<Body> {
        let (status, message) = match &self {
            ApiError::NotFound => (404, self.to_string()),
            ApiError::BadRequest(_) => (400, self.to_string()),
            ApiError::Unauthorized(_) => (401, self.to_string()),
            ApiError::Internal(_) => (500, "Internal server error".to_string()),
        };

        let body = serde_json::json!({ "error": message }).to_string();

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Body::from(body))
            .unwrap()
    }
}

impl From<aws_sdk_dynamodb::Error> for ApiError {
    fn from(e: aws_sdk_dynamodb::Error) -> Self {
        tracing::error!("DynamoDB error: {:?}", e);
        ApiError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::BadRequest(format!("Invalid JSON: {e}"))
    }
}

use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;
use serde_json;
use shared::domain::aggregates::Todo;
use std::collections::HashMap;

pub struct ApiResponse;

impl ApiResponse {
    pub fn success<T: serde::Serialize>(data: T, status_code: i64) -> ApiGatewayProxyResponse {
        let headers = Self::default_headers();
        
        ApiGatewayProxyResponse {
            status_code,
            headers,
            multi_value_headers: HashMap::new(),
            body: Some(serde_json::to_string(&data).unwrap_or_default()),
            is_base64_encoded: Some(false),
        }
    }

    pub fn created(todo: Todo) -> ApiGatewayProxyResponse {
        Self::success(todo, 201)
    }

    pub fn ok<T: serde::Serialize>(data: T) -> ApiGatewayProxyResponse {
        Self::success(data, 200)
    }

    pub fn no_content() -> ApiGatewayProxyResponse {
        let headers = Self::default_headers();
        
        ApiGatewayProxyResponse {
            status_code: 204,
            headers,
            multi_value_headers: HashMap::new(),
            body: None,
            is_base64_encoded: Some(false),
        }
    }

    pub fn bad_request(message: &str) -> ApiGatewayProxyResponse {
        Self::error(400, "Bad Request", message)
    }

    pub fn not_found(message: &str) -> ApiGatewayProxyResponse {
        Self::error(404, "Not Found", message)
    }

    pub fn conflict(message: &str) -> ApiGatewayProxyResponse {
        Self::error(409, "Conflict", message)
    }

    pub fn internal_server_error(message: &str) -> ApiGatewayProxyResponse {
        Self::error(500, "Internal Server Error", message)
    }

    fn error(status_code: i64, error_type: &str, message: &str) -> ApiGatewayProxyResponse {
        let headers = Self::default_headers();
        
        let error_body = serde_json::json!({
            "error": {
                "type": error_type,
                "message": message
            }
        });

        ApiGatewayProxyResponse {
            status_code,
            headers,
            multi_value_headers: HashMap::new(),
            body: Some(error_body.to_string()),
            is_base64_encoded: Some(false),
        }
    }

    fn default_headers() -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        headers.insert("Access-Control-Allow-Headers".to_string(), "Content-Type,Authorization".to_string());
        headers.insert("Access-Control-Allow-Methods".to_string(), "GET,POST,PUT,DELETE,OPTIONS".to_string());
        headers
    }
}
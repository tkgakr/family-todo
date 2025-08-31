use aws_lambda_events::event::apigw::ApiGatewayProxyResponse;
use aws_lambda_events::encodings::Body;
use aws_lambda_events::http::HeaderMap;
use serde_json;

pub struct ApiResponse;

impl ApiResponse {
    pub fn success<T: serde::Serialize>(data: T, status_code: i64) -> ApiGatewayProxyResponse {
        let headers = Self::default_headers();
        
        ApiGatewayProxyResponse {
            status_code,
            headers,
            multi_value_headers: HeaderMap::new(),
            body: Some(Body::Text(serde_json::to_string(&data).unwrap_or_default())),
            is_base64_encoded: false,
        }
    }

    pub fn ok<T: serde::Serialize>(data: T) -> ApiGatewayProxyResponse {
        Self::success(data, 200)
    }

    pub fn bad_request(message: &str) -> ApiGatewayProxyResponse {
        Self::error(400, "Bad Request", message)
    }

    pub fn not_found(message: &str) -> ApiGatewayProxyResponse {
        Self::error(404, "Not Found", message)
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
            multi_value_headers: HeaderMap::new(),
            body: Some(Body::Text(error_body.to_string())),
            is_base64_encoded: false,
        }
    }

    fn default_headers() -> HeaderMap {
        HeaderMap::new()
    }
}
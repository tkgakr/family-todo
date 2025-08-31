use aws_lambda_events::encodings::Body;
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use http::HeaderMap;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use shared::auth::RefreshTokenResponse;

use tracing::{error, info, warn};

#[derive(Debug, Deserialize)]
struct RefreshTokenRequest {
    refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

/// トークンリフレッシュのメイン関数
async fn function_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let request = event.payload;

    info!("トークンリフレッシュリクエストを処理中");

    // リクエストボディをパース
    let body = request.body.unwrap_or_default();
    let refresh_request: RefreshTokenRequest = serde_json::from_str(&body).map_err(|e| {
        warn!("リクエストボディのパースに失敗: {}", e);
        anyhow::anyhow!("無効なリクエスト形式")
    })?;

    // 環境変数から設定を取得
    let client_id = std::env::var("COGNITO_CLIENT_ID")
        .map_err(|_| anyhow::anyhow!("COGNITO_CLIENT_ID 環境変数が設定されていません"))?;
    let _region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

    // トークンをリフレッシュ
    match refresh_access_token(&refresh_request.refresh_token, &client_id).await {
        Ok(response) => {
            info!("トークンリフレッシュ成功");
            Ok(create_success_response(response))
        }
        Err(e) => {
            error!("トークンリフレッシュ失敗: {}", e);
            Ok(create_error_response(400, "invalid_grant", &e.to_string()))
        }
    }
}

/// Cognito を使用してアクセストークンをリフレッシュ
async fn refresh_access_token(
    refresh_token: &str,
    client_id: &str,
) -> Result<RefreshTokenResponse, anyhow::Error> {
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    let cognito_client = aws_sdk_cognitoidentityprovider::Client::new(&config);

    // リフレッシュトークンを使用して新しいアクセストークンを取得
    let response = cognito_client
        .initiate_auth()
        .auth_flow(aws_sdk_cognitoidentityprovider::types::AuthFlowType::RefreshTokenAuth)
        .client_id(client_id)
        .auth_parameters("REFRESH_TOKEN", refresh_token)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Cognito InitiateAuth エラー: {}", e))?;

    let auth_result = response
        .authentication_result()
        .ok_or_else(|| anyhow::anyhow!("認証結果が取得できませんでした"))?;

    Ok(RefreshTokenResponse {
        access_token: auth_result
            .access_token()
            .ok_or_else(|| anyhow::anyhow!("アクセストークンが取得できませんでした"))?
            .to_string(),
        id_token: auth_result.id_token().map(|s| s.to_string()),
        expires_in: 3600, // デフォルト値を設定
    })
}

/// 成功レスポンスを作成
fn create_success_response(token_response: RefreshTokenResponse) -> ApiGatewayProxyResponse {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert("access-control-allow-origin", "*".parse().unwrap());
    headers.insert(
        "access-control-allow-headers",
        "Content-Type,Authorization".parse().unwrap(),
    );

    ApiGatewayProxyResponse {
        status_code: 200,
        headers,
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(serde_json::to_string(&token_response).unwrap())),
        is_base64_encoded: false,
    }
}

/// エラーレスポンスを作成
fn create_error_response(status_code: i64, error: &str, message: &str) -> ApiGatewayProxyResponse {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert("access-control-allow-origin", "*".parse().unwrap());
    headers.insert(
        "access-control-allow-headers",
        "Content-Type,Authorization".parse().unwrap(),
    );

    let error_response = ErrorResponse {
        error: error.to_string(),
        message: message.to_string(),
    };

    ApiGatewayProxyResponse {
        status_code,
        headers,
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(serde_json::to_string(&error_response).unwrap())),
        is_base64_encoded: false,
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // トレーシングを初期化
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Token Refresh Lambda を開始中...");

    run(service_fn(function_handler)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_refresh_token_request_deserialization() {
        let json_input = json!({
            "refresh_token": "eyJjdHkiOiJKV1QiLCJlbmMiOiJBMjU2R0NNIiwiYWxnIjoiUlNBLU9BRVAifQ..."
        });

        let request: RefreshTokenRequest = serde_json::from_value(json_input).unwrap();
        assert!(request.refresh_token.starts_with("eyJ"));
    }

    #[test]
    fn test_error_response_creation() {
        let response =
            create_error_response(400, "invalid_grant", "リフレッシュトークンが無効です");

        assert_eq!(response.status_code, 400);
        assert!(response.body.is_some());

        let body_str = match response.body.as_ref().unwrap() {
            Body::Text(s) => s,
            _ => panic!("Expected text body"),
        };
        let body: ErrorResponse = serde_json::from_str(body_str).unwrap();
        assert_eq!(body.error, "invalid_grant");
    }

    #[test]
    fn test_success_response_creation() {
        let token_response = RefreshTokenResponse {
            access_token: "new_access_token".to_string(),
            id_token: Some("new_id_token".to_string()),
            expires_in: 3600,
        };

        let response = create_success_response(token_response);

        assert_eq!(response.status_code, 200);
        assert!(response.body.is_some());
    }
}

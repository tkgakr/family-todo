use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use shared::auth::{extract_family_id_from_claims, JwtValidator};
use std::collections::HashMap;
use tracing::{error, info, warn};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct AuthorizerRequest {
    #[serde(rename = "type")]
    request_type: String,
    authorization_token: Option<String>,
    method_arn: String,
    request_context: Option<RequestContext>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct RequestContext {
    account_id: String,
    api_id: String,
    stage: String,
    request_id: String,
    identity: Identity,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct Identity {
    source_ip: String,
    user_agent: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizerResponse {
    principal_id: String,
    policy_document: PolicyDocument,
    context: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PolicyDocument {
    version: String,
    statement: Vec<Statement>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Statement {
    action: String,
    effect: String,
    resource: String,
}

/// Lambda Authorizer のメイン関数
async fn function_handler(
    event: LambdaEvent<AuthorizerRequest>,
) -> Result<AuthorizerResponse, Error> {
    let request = event.payload;

    info!("認証リクエストを処理中: method_arn={}", request.method_arn);

    // 環境変数から設定を取得
    let user_pool_id = std::env::var("COGNITO_USER_POOL_ID")
        .map_err(|_| anyhow::anyhow!("COGNITO_USER_POOL_ID 環境変数が設定されていません"))?;
    let client_id = std::env::var("COGNITO_CLIENT_ID")
        .map_err(|_| anyhow::anyhow!("COGNITO_CLIENT_ID 環境変数が設定されていません"))?;
    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

    // JWT バリデーターを初期化
    let validator = JwtValidator::new(&user_pool_id, &client_id, &region)
        .await
        .map_err(|e| {
            error!("JWT バリデーターの初期化に失敗: {}", e);
            anyhow::anyhow!("認証設定エラー")
        })?;

    // Authorization トークンを取得
    let token = request
        .authorization_token
        .ok_or_else(|| anyhow::anyhow!("Authorization トークンが提供されていません"))?;

    // Bearer プレフィックスを除去
    let token = token
        .strip_prefix("Bearer ")
        .ok_or_else(|| anyhow::anyhow!("無効な Authorization ヘッダー形式"))?;

    // JWT トークンを検証
    let claims = validator.validate_token(token).map_err(|e| {
        warn!("JWT トークン検証失敗: {}", e);
        anyhow::anyhow!("無効なトークン")
    })?;

    // 家族 ID を抽出
    let family_id = extract_family_id_from_claims(&claims).map_err(|e| {
        warn!("家族 ID の抽出に失敗: {}", e);
        anyhow::anyhow!("家族 ID が見つかりません")
    })?;

    info!("認証成功: user_id={}, family_id={}", claims.sub, family_id);

    // IAM ポリシーを生成
    let policy = generate_policy(&claims.sub, "Allow", &request.method_arn);

    // コンテキスト情報を設定
    let mut context = HashMap::new();
    context.insert("userId".to_string(), claims.sub.clone());
    context.insert("familyId".to_string(), family_id);
    context.insert("email".to_string(), claims.email.clone());
    context.insert("tokenUse".to_string(), claims.token_use.clone());

    // Cognito グループ情報を追加
    if let Some(groups) = &claims.cognito_groups {
        context.insert("cognitoGroups".to_string(), groups.join(","));
    }

    Ok(AuthorizerResponse {
        principal_id: claims.sub,
        policy_document: policy,
        context,
    })
}

/// IAM ポリシードキュメントを生成
fn generate_policy(_principal_id: &str, effect: &str, resource: &str) -> PolicyDocument {
    PolicyDocument {
        version: "2012-10-17".to_string(),
        statement: vec![Statement {
            action: "execute-api:Invoke".to_string(),
            effect: effect.to_string(),
            resource: resource.to_string(),
        }],
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // トレーシングを初期化
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Lambda Authorizer を開始中...");

    run(service_fn(function_handler)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_policy() {
        let policy = generate_policy(
            "user123",
            "Allow",
            "arn:aws:execute-api:us-east-1:123456789012:abcdef123/*",
        );

        assert_eq!(policy.version, "2012-10-17");
        assert_eq!(policy.statement.len(), 1);
        assert_eq!(policy.statement[0].effect, "Allow");
        assert_eq!(policy.statement[0].action, "execute-api:Invoke");
    }

    #[tokio::test]
    async fn test_authorizer_request_deserialization() {
        let json_input = json!({
            "type": "TOKEN",
            "authorizationToken": "Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
            "methodArn": "arn:aws:execute-api:us-east-1:123456789012:abcdef123/dev/GET/commands/todos"
        });

        let request: AuthorizerRequest = serde_json::from_value(json_input).unwrap();
        assert_eq!(request.request_type, "TOKEN");
        assert!(request.authorization_token.is_some());
    }
}

use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_lambda_events::encodings::Body;
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_sdk_cognitoidentityprovider::Client as CognitoClient;
use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_ses::Client as SesClient;
use chrono::{DateTime, Duration, Utc};
use http::HeaderMap;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use shared::{config::Config, tracing::init_tracing};
use std::sync::Arc;
use tracing::{error, info};
use ulid::Ulid;

/// 家族招待リクエスト
#[derive(Debug, Deserialize)]
struct InviteFamilyMemberRequest {
    email: String,
    role: String, // "admin" | "member"
    display_name: Option<String>,
}

/// 招待受諾リクエスト
#[derive(Debug, Deserialize)]
struct AcceptInvitationRequest {
    invitation_token: String,
    display_name: String,
}

/// 招待情報
#[derive(Debug, Serialize, Deserialize)]
struct InvitationRecord {
    invitation_token: String,
    family_id: String,
    email: String,
    role: String,
    invited_by: String,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

/// アプリケーション状態
struct AppState {
    dynamodb_client: DynamoDbClient,
    cognito_client: CognitoClient,
    ses_client: SesClient,
    config: Config,
}

impl AppState {
    async fn new() -> Result<Self> {
        let aws_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let config = Config::from_env().map_err(|e| anyhow::anyhow!("Config error: {}", e))?;

        Ok(Self {
            dynamodb_client: DynamoDbClient::new(&aws_config),
            cognito_client: CognitoClient::new(&aws_config),
            ses_client: SesClient::new(&aws_config),
            config,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();

    let app_state = Arc::new(AppState::new().await.map_err(|e| {
        error!("Failed to initialize app state: {}", e);
        Error::from(e.to_string().as_str())
    })?);

    run(service_fn(
        move |event: LambdaEvent<ApiGatewayProxyRequest>| {
            let app_state = Arc::clone(&app_state);
            async move { handle_request(event, &app_state).await }
        },
    ))
    .await
}

/// HTTP リクエストハンドラー
async fn handle_request(
    event: LambdaEvent<ApiGatewayProxyRequest>,
    app_state: &AppState,
) -> Result<ApiGatewayProxyResponse, Error> {
    let request = event.payload;
    let method = request.http_method.as_str();
    let path = request.path.as_deref().unwrap_or("");

    info!("Handling {} {}", method, path);

    let response = match (method, path) {
        ("POST", "/family/invite") => handle_invite_member(request, app_state).await,
        ("POST", "/family/accept-invitation") => handle_accept_invitation(request, app_state).await,
        ("GET", "/family/members") => handle_list_members(request, app_state).await,
        ("GET", "/family/invitations") => handle_list_invitations(request, app_state).await,
        _ => Ok(create_response(404, "Not Found")),
    };

    match response {
        Ok(resp) => Ok(resp),
        Err(e) => {
            error!("Request handling error: {}", e);
            Ok(create_response(
                500,
                &format!("Internal Server Error: {}", e),
            ))
        }
    }
}

/// レスポンスを作成するヘルパー関数
fn create_response(status_code: i64, body: &str) -> ApiGatewayProxyResponse {
    ApiGatewayProxyResponse {
        status_code,
        headers: HeaderMap::new(),
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(body.to_string())),
        is_base64_encoded: false,
    }
}

/// JSON レスポンスを作成するヘルパー関数
fn create_json_response<T: Serialize>(status_code: i64, data: &T) -> ApiGatewayProxyResponse {
    let body = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());

    ApiGatewayProxyResponse {
        status_code,
        headers,
        multi_value_headers: HeaderMap::new(),
        body: Some(Body::Text(body)),
        is_base64_encoded: false,
    }
}

/// 家族メンバー招待処理
async fn handle_invite_member(
    request: ApiGatewayProxyRequest,
    app_state: &AppState,
) -> Result<ApiGatewayProxyResponse, anyhow::Error> {
    // 簡略化のため、認証は後で実装
    let family_id = "test-family-id";
    let inviter_id = "test-user-id";

    // リクエストボディをパース
    let body = request.body.unwrap_or_default();
    let invite_request: InviteFamilyMemberRequest = serde_json::from_str(&body)?;

    // 招待トークンを生成
    let invitation_token = Ulid::new().to_string();
    let expires_at = Utc::now() + Duration::days(7); // 7日間有効

    // 招待レコードを DynamoDB に保存
    let invitation_record = InvitationRecord {
        invitation_token: invitation_token.clone(),
        family_id: family_id.to_string(),
        email: invite_request.email.clone(),
        role: invite_request.role.clone(),
        invited_by: inviter_id.to_string(),
        expires_at,
        created_at: Utc::now(),
    };

    save_invitation_record(app_state, &invitation_record).await?;

    // 招待メールを送信
    send_invitation_email(app_state, &invitation_record, &invite_request.display_name).await?;

    info!(
        "Family invitation sent: family_id={}, email={}, role={}",
        family_id, invite_request.email, invite_request.role
    );

    Ok(create_json_response(
        201,
        &serde_json::json!({
            "message": "Invitation sent successfully",
            "invitation_token": invitation_token,
            "expires_at": expires_at
        }),
    ))
}

/// 招待受諾処理
async fn handle_accept_invitation(
    request: ApiGatewayProxyRequest,
    app_state: &AppState,
) -> Result<ApiGatewayProxyResponse, anyhow::Error> {
    // リクエストボディをパース
    let body = request.body.unwrap_or_default();
    let accept_request: AcceptInvitationRequest = serde_json::from_str(&body)?;

    // 招待レコードを取得
    let invitation = get_invitation_record(app_state, &accept_request.invitation_token).await?;

    // 招待の有効性をチェック
    if invitation.expires_at < Utc::now() {
        return Ok(create_response(400, "Invitation has expired"));
    }

    // Cognito でユーザーを作成
    create_cognito_user(app_state, &invitation, &accept_request.display_name).await?;

    // 招待レコードを削除
    delete_invitation_record(app_state, &accept_request.invitation_token).await?;

    info!(
        "Family invitation accepted: family_id={}, email={}",
        invitation.family_id, invitation.email
    );

    Ok(create_json_response(
        200,
        &serde_json::json!({
            "message": "Invitation accepted successfully",
            "family_id": invitation.family_id
        }),
    ))
}

/// 家族メンバー一覧取得
async fn handle_list_members(
    _request: ApiGatewayProxyRequest,
    _app_state: &AppState,
) -> Result<ApiGatewayProxyResponse, anyhow::Error> {
    // 簡略化のため、空の配列を返す
    Ok(create_json_response(200, &Vec::<String>::new()))
}

/// 招待一覧取得
async fn handle_list_invitations(
    _request: ApiGatewayProxyRequest,
    app_state: &AppState,
) -> Result<ApiGatewayProxyResponse, anyhow::Error> {
    let family_id = "test-family-id"; // 簡略化
    let invitations = list_family_invitations(app_state, family_id).await?;
    Ok(create_json_response(200, &invitations))
}

/// 招待レコードを DynamoDB に保存
async fn save_invitation_record(app_state: &AppState, invitation: &InvitationRecord) -> Result<()> {
    let ttl = invitation.expires_at.timestamp();

    app_state
        .dynamodb_client
        .put_item()
        .table_name(&app_state.config.dynamodb_table)
        .item(
            "PK",
            aws_sdk_dynamodb::types::AttributeValue::S(format!("FAMILY#{}", invitation.family_id)),
        )
        .item(
            "SK",
            aws_sdk_dynamodb::types::AttributeValue::S(format!(
                "INVITATION#{}",
                invitation.invitation_token
            )),
        )
        .item(
            "EntityType",
            aws_sdk_dynamodb::types::AttributeValue::S("Invitation".to_string()),
        )
        .item(
            "Data",
            aws_sdk_dynamodb::types::AttributeValue::S(serde_json::to_string(invitation)?),
        )
        .item(
            "TTL",
            aws_sdk_dynamodb::types::AttributeValue::N(ttl.to_string()),
        )
        .item(
            "CreatedAt",
            aws_sdk_dynamodb::types::AttributeValue::S(invitation.created_at.to_rfc3339()),
        )
        .send()
        .await?;

    Ok(())
}

/// 招待レコードを DynamoDB から取得
async fn get_invitation_record(
    app_state: &AppState,
    invitation_token: &str,
) -> Result<InvitationRecord> {
    // 簡略化のため、GSI を使わずに直接検索
    // 実際の実装では GSI1 を使用する必要がある
    let result = app_state
        .dynamodb_client
        .scan()
        .table_name(&app_state.config.dynamodb_table)
        .filter_expression("contains(SK, :token)")
        .expression_attribute_values(
            ":token",
            aws_sdk_dynamodb::types::AttributeValue::S(invitation_token.to_string()),
        )
        .send()
        .await?;

    let items = result.items.unwrap_or_default();
    if items.is_empty() {
        return Err(anyhow::anyhow!("Invitation not found"));
    }

    let data = items[0]
        .get("Data")
        .and_then(|v| v.as_s().ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid invitation data"))?;

    let invitation: InvitationRecord = serde_json::from_str(data)?;
    Ok(invitation)
}

/// 招待レコードを削除
async fn delete_invitation_record(app_state: &AppState, invitation_token: &str) -> Result<()> {
    // まず招待レコードを取得してファミリーIDを取得
    let invitation = get_invitation_record(app_state, invitation_token).await?;

    app_state
        .dynamodb_client
        .delete_item()
        .table_name(&app_state.config.dynamodb_table)
        .key(
            "PK",
            aws_sdk_dynamodb::types::AttributeValue::S(format!("FAMILY#{}", invitation.family_id)),
        )
        .key(
            "SK",
            aws_sdk_dynamodb::types::AttributeValue::S(format!("INVITATION#{}", invitation_token)),
        )
        .send()
        .await?;

    Ok(())
}

/// Cognito でユーザーを作成
async fn create_cognito_user(
    app_state: &AppState,
    invitation: &InvitationRecord,
    display_name: &str,
) -> Result<()> {
    let user_pool_id = std::env::var("COGNITO_USER_POOL_ID")?;

    app_state
        .cognito_client
        .admin_create_user()
        .user_pool_id(&user_pool_id)
        .username(&invitation.email)
        .user_attributes(
            aws_sdk_cognitoidentityprovider::types::AttributeType::builder()
                .name("email")
                .value(&invitation.email)
                .build()?,
        )
        .user_attributes(
            aws_sdk_cognitoidentityprovider::types::AttributeType::builder()
                .name("custom:family_id")
                .value(&invitation.family_id)
                .build()?,
        )
        .user_attributes(
            aws_sdk_cognitoidentityprovider::types::AttributeType::builder()
                .name("custom:family_role")
                .value(&invitation.role)
                .build()?,
        )
        .user_attributes(
            aws_sdk_cognitoidentityprovider::types::AttributeType::builder()
                .name("custom:display_name")
                .value(display_name)
                .build()?,
        )
        .message_action(aws_sdk_cognitoidentityprovider::types::MessageActionType::Suppress)
        .send()
        .await?;

    Ok(())
}

/// 家族の招待一覧を取得
async fn list_family_invitations(
    app_state: &AppState,
    family_id: &str,
) -> Result<Vec<InvitationRecord>> {
    let result = app_state
        .dynamodb_client
        .query()
        .table_name(&app_state.config.dynamodb_table)
        .key_condition_expression("PK = :pk AND begins_with(SK, :sk)")
        .expression_attribute_values(
            ":pk",
            aws_sdk_dynamodb::types::AttributeValue::S(format!("FAMILY#{}", family_id)),
        )
        .expression_attribute_values(
            ":sk",
            aws_sdk_dynamodb::types::AttributeValue::S("INVITATION#".to_string()),
        )
        .send()
        .await?;

    let mut invitations = Vec::new();
    for item in result.items.unwrap_or_default() {
        if let Some(data) = item.get("Data").and_then(|v| v.as_s().ok()) {
            if let Ok(invitation) = serde_json::from_str::<InvitationRecord>(data) {
                invitations.push(invitation);
            }
        }
    }

    Ok(invitations)
}

/// 招待メールを送信
async fn send_invitation_email(
    app_state: &AppState,
    invitation: &InvitationRecord,
    display_name: &Option<String>,
) -> Result<()> {
    let frontend_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let invitation_url = format!(
        "{}/accept-invitation?token={}",
        frontend_url, invitation.invitation_token
    );

    let subject = "家族 ToDo アプリへの招待";
    let body_text = format!(
        "家族 ToDo アプリへの招待\n\n{}さん、こんにちは！\n\n家族 ToDo アプリへの招待が届きました。\n\n以下のリンクをクリックして参加してください：\n{}\n\nこの招待は{}まで有効です。\n\n家族 ToDo アプリチーム",
        display_name.as_deref().unwrap_or(""),
        invitation_url,
        invitation.expires_at.format("%Y年%m月%d日 %H:%M")
    );

    app_state
        .ses_client
        .send_email()
        .source("noreply@familytodo.app")
        .destination(
            aws_sdk_ses::types::Destination::builder()
                .to_addresses(&invitation.email)
                .build(),
        )
        .message(
            aws_sdk_ses::types::Message::builder()
                .subject(
                    aws_sdk_ses::types::Content::builder()
                        .data(subject)
                        .charset("UTF-8")
                        .build()?,
                )
                .body(
                    aws_sdk_ses::types::Body::builder()
                        .text(
                            aws_sdk_ses::types::Content::builder()
                                .data(body_text)
                                .charset("UTF-8")
                                .build()?,
                        )
                        .build(),
                )
                .build(),
        )
        .send()
        .await?;

    Ok(())
}

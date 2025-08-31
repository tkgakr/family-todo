use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_lambda_events::encodings::Body;
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use aws_sdk_cognitoidentityprovider::Client as CognitoClient;
use aws_sdk_ses::Client as SesClient;
use chrono::{DateTime, Duration, Utc};
use domain::{
    FamilyEvent, FamilyId, FamilyInvitation, FamilyMember, FamilyRole, InvitationToken, UserId,
};
use http::HeaderMap;
use infrastructure::{
    DynamoDbClient, FamilyEventRepository, FamilyInvitationRepository, FamilyMemberRepository,
};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use shared::{
    auth::{extract_family_id_from_claims, extract_user_claims_from_headers, JwtValidator},
    config::Config,
    tracing::init_tracing,
};
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

/// 招待レスポンス
#[derive(Debug, Serialize)]
struct InvitationResponse {
    message: String,
    invitation_token: String,
    expires_at: DateTime<Utc>,
}

/// 招待受諾レスポンス
#[derive(Debug, Serialize)]
struct AcceptInvitationResponse {
    message: String,
    family_id: String,
    user_id: String,
}

/// アプリケーション状態
struct AppState {
    invitation_repo: FamilyInvitationRepository,
    member_repo: FamilyMemberRepository,
    event_repo: FamilyEventRepository,
    cognito_client: CognitoClient,
    ses_client: SesClient,
    jwt_validator: JwtValidator,
    #[allow(dead_code)]
    config: Config,
}

impl AppState {
    async fn new() -> Result<Self> {
        let aws_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let config = Config::from_env().map_err(|e| anyhow::anyhow!("Config error: {}", e))?;

        // DynamoDB クライアントを初期化
        let dynamodb_client = DynamoDbClient::new(&config)
            .await
            .map_err(|e| anyhow::anyhow!("DynamoDB client initialization error: {}", e))?;

        // リポジトリを初期化
        let invitation_repo = FamilyInvitationRepository::new(dynamodb_client.clone());
        let member_repo = FamilyMemberRepository::new(dynamodb_client.clone());
        let event_repo = FamilyEventRepository::new(dynamodb_client);

        // JWT バリデーターを初期化
        let user_pool_id = std::env::var("COGNITO_USER_POOL_ID")
            .map_err(|_| anyhow::anyhow!("COGNITO_USER_POOL_ID environment variable not set"))?;
        let client_id = std::env::var("COGNITO_CLIENT_ID")
            .map_err(|_| anyhow::anyhow!("COGNITO_CLIENT_ID environment variable not set"))?;
        let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        let jwt_validator = JwtValidator::new(&user_pool_id, &client_id, &region)
            .await
            .map_err(|e| anyhow::anyhow!("JWT validator initialization error: {}", e))?;

        Ok(Self {
            invitation_repo,
            member_repo,
            event_repo,
            cognito_client: CognitoClient::new(&aws_config),
            ses_client: SesClient::new(&aws_config),
            jwt_validator,
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
            Ok(create_response(500, &format!("Internal Server Error: {e}")))
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
    // 認証情報を取得
    let claims = extract_user_claims_from_headers(&request.headers, &app_state.jwt_validator)
        .await
        .map_err(|e| anyhow::anyhow!("認証エラー: {}", e))?;

    let family_id = extract_family_id_from_claims(&claims)
        .map_err(|e| anyhow::anyhow!("家族ID取得エラー: {}", e))?;

    let inviter_id = claims.sub.clone();

    // 管理者権限をチェック
    let inviter = app_state
        .member_repo
        .get_member_by_user_id(&family_id, &inviter_id)
        .await
        .map_err(|e| anyhow::anyhow!("メンバー取得エラー: {}", e))?;

    match inviter {
        Some(member) if member.role.is_admin() => {
            // 管理者権限あり - 続行
        }
        Some(_) => {
            return Ok(create_response(403, "管理者権限が必要です"));
        }
        None => {
            return Ok(create_response(403, "家族メンバーではありません"));
        }
    }

    // リクエストボディをパース
    let body = request.body.unwrap_or_default();
    let invite_request: InviteFamilyMemberRequest = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("リクエストパースエラー: {}", e))?;

    // 役割をバリデーション
    let role = FamilyRole::from_string(&invite_request.role)
        .map_err(|e| anyhow::anyhow!("無効な役割: {}", e))?;

    // 招待を作成
    let family_id_obj = FamilyId::from_string(family_id.clone())
        .map_err(|e| anyhow::anyhow!("家族ID変換エラー: {}", e))?;
    let inviter_id_obj = UserId::from_string(inviter_id.clone())
        .map_err(|e| anyhow::anyhow!("ユーザーID変換エラー: {}", e))?;

    let expires_at = Utc::now() + Duration::days(7); // 7日間有効
    let invitation = FamilyInvitation::new(
        family_id_obj,
        invite_request.email.clone(),
        role,
        inviter_id_obj,
        expires_at,
    )
    .map_err(|e| anyhow::anyhow!("招待作成エラー: {}", e))?;

    // 招待を保存
    app_state
        .invitation_repo
        .save_invitation(&invitation)
        .await
        .map_err(|e| anyhow::anyhow!("招待保存エラー: {}", e))?;

    // 招待イベントを記録
    let invitation_event = FamilyEvent::new_member_invited(
        family_id.clone(),
        invitation.invitation_token.as_str().to_string(),
        invitation.email.clone(),
        invitation.role.as_str().to_string(),
        inviter_id,
        expires_at,
    );

    app_state
        .event_repo
        .save_family_event(invitation_event)
        .await
        .map_err(|e| anyhow::anyhow!("イベント保存エラー: {}", e))?;

    // 招待メールを送信
    send_invitation_email(app_state, &invitation, &invite_request.display_name).await?;

    info!(
        "家族招待を送信しました: family_id={}, email={}, role={}",
        family_id, invite_request.email, invite_request.role
    );

    let response = InvitationResponse {
        message: "招待を送信しました".to_string(),
        invitation_token: invitation.invitation_token.as_str().to_string(),
        expires_at: invitation.expires_at,
    };

    Ok(create_json_response(201, &response))
}

/// 招待受諾処理
async fn handle_accept_invitation(
    request: ApiGatewayProxyRequest,
    app_state: &AppState,
) -> Result<ApiGatewayProxyResponse, anyhow::Error> {
    // リクエストボディをパース
    let body = request.body.unwrap_or_default();
    let accept_request: AcceptInvitationRequest = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("リクエストパースエラー: {}", e))?;

    // 招待を取得
    let invitation_token = InvitationToken::from_string(accept_request.invitation_token.clone())
        .map_err(|e| anyhow::anyhow!("招待トークン変換エラー: {}", e))?;

    let invitation = app_state
        .invitation_repo
        .get_invitation_by_token(invitation_token.as_str())
        .await
        .map_err(|e| anyhow::anyhow!("招待取得エラー: {}", e))?;

    let invitation = match invitation {
        Some(inv) => inv,
        None => {
            return Ok(create_response(404, "招待が見つかりません"));
        }
    };

    // 招待の有効性をチェック
    if !invitation.is_valid() {
        if invitation.is_expired() {
            return Ok(create_response(400, "招待の有効期限が切れています"));
        } else {
            return Ok(create_response(400, "招待は既に使用済みです"));
        }
    }

    // Cognito でユーザーを作成
    let user_id = create_cognito_user(app_state, &invitation, &accept_request.display_name).await?;

    // 家族メンバーとして追加
    let member = FamilyMember::new(
        UserId::from_string(user_id.clone())
            .map_err(|e| anyhow::anyhow!("ユーザーID変換エラー: {}", e))?,
        invitation.email.clone(),
        accept_request.display_name.clone(),
        invitation.role.clone(),
    )
    .map_err(|e| anyhow::anyhow!("メンバー作成エラー: {}", e))?;

    app_state
        .member_repo
        .save_member(invitation.family_id.as_str(), &member)
        .await
        .map_err(|e| anyhow::anyhow!("メンバー保存エラー: {}", e))?;

    // メンバー参加イベントを記録
    let join_event = FamilyEvent::new_member_joined(
        invitation.family_id.as_str().to_string(),
        user_id.clone(),
        invitation.email.clone(),
        invitation.role.as_str().to_string(),
        accept_request.display_name.clone(),
        invitation.invitation_token.as_str().to_string(),
    );

    app_state
        .event_repo
        .save_family_event(join_event)
        .await
        .map_err(|e| anyhow::anyhow!("イベント保存エラー: {}", e))?;

    // 招待を削除
    app_state
        .invitation_repo
        .delete_invitation(
            invitation.family_id.as_str(),
            invitation.invitation_token.as_str(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("招待削除エラー: {}", e))?;

    info!(
        "家族招待を受諾しました: family_id={}, email={}, user_id={}",
        invitation.family_id.as_str(),
        invitation.email,
        user_id
    );

    let response = AcceptInvitationResponse {
        message: "招待を受諾しました".to_string(),
        family_id: invitation.family_id.as_str().to_string(),
        user_id,
    };

    Ok(create_json_response(200, &response))
}

/// 家族メンバー一覧取得
async fn handle_list_members(
    request: ApiGatewayProxyRequest,
    app_state: &AppState,
) -> Result<ApiGatewayProxyResponse, anyhow::Error> {
    // 認証情報を取得
    let claims = extract_user_claims_from_headers(&request.headers, &app_state.jwt_validator)
        .await
        .map_err(|e| anyhow::anyhow!("認証エラー: {}", e))?;

    let family_id = extract_family_id_from_claims(&claims)
        .map_err(|e| anyhow::anyhow!("家族ID取得エラー: {}", e))?;

    // 家族メンバーかチェック
    let requester = app_state
        .member_repo
        .get_member_by_user_id(&family_id, &claims.sub)
        .await
        .map_err(|e| anyhow::anyhow!("メンバー取得エラー: {}", e))?;

    if requester.is_none() {
        return Ok(create_response(403, "家族メンバーではありません"));
    }

    // メンバー一覧を取得
    let members = app_state
        .member_repo
        .list_family_members(&family_id)
        .await
        .map_err(|e| anyhow::anyhow!("メンバー一覧取得エラー: {}", e))?;

    // アクティブなメンバーのみフィルタ
    let active_members: Vec<_> = members.into_iter().filter(|m| m.is_active).collect();

    info!(
        "家族メンバー一覧を取得しました: family_id={}, count={}",
        family_id,
        active_members.len()
    );

    Ok(create_json_response(200, &active_members))
}

/// 招待一覧取得
async fn handle_list_invitations(
    request: ApiGatewayProxyRequest,
    app_state: &AppState,
) -> Result<ApiGatewayProxyResponse, anyhow::Error> {
    // 認証情報を取得
    let claims = extract_user_claims_from_headers(&request.headers, &app_state.jwt_validator)
        .await
        .map_err(|e| anyhow::anyhow!("認証エラー: {}", e))?;

    let family_id = extract_family_id_from_claims(&claims)
        .map_err(|e| anyhow::anyhow!("家族ID取得エラー: {}", e))?;

    // 管理者権限をチェック
    let requester = app_state
        .member_repo
        .get_member_by_user_id(&family_id, &claims.sub)
        .await
        .map_err(|e| anyhow::anyhow!("メンバー取得エラー: {}", e))?;

    match requester {
        Some(member) if member.role.is_admin() => {
            // 管理者権限あり - 続行
        }
        Some(_) => {
            return Ok(create_response(403, "管理者権限が必要です"));
        }
        None => {
            return Ok(create_response(403, "家族メンバーではありません"));
        }
    }

    // 招待一覧を取得
    let invitations = app_state
        .invitation_repo
        .list_family_invitations(&family_id)
        .await
        .map_err(|e| anyhow::anyhow!("招待一覧取得エラー: {}", e))?;

    // 有効な招待のみフィルタ
    let valid_invitations: Vec<_> = invitations
        .into_iter()
        .filter(|inv| inv.is_valid())
        .collect();

    info!(
        "招待一覧を取得しました: family_id={}, count={}",
        family_id,
        valid_invitations.len()
    );

    Ok(create_json_response(200, &valid_invitations))
}

/// Cognito でユーザーを作成
async fn create_cognito_user(
    app_state: &AppState,
    invitation: &FamilyInvitation,
    display_name: &str,
) -> Result<String> {
    let user_pool_id = std::env::var("COGNITO_USER_POOL_ID")
        .map_err(|_| anyhow::anyhow!("COGNITO_USER_POOL_ID environment variable not set"))?;

    // ユーザーIDを生成
    let user_id = Ulid::new().to_string();

    let _result = app_state
        .cognito_client
        .admin_create_user()
        .user_pool_id(&user_pool_id)
        .username(&user_id) // ULIDをユーザー名として使用
        .user_attributes(
            aws_sdk_cognitoidentityprovider::types::AttributeType::builder()
                .name("email")
                .value(&invitation.email)
                .build()
                .map_err(|e| anyhow::anyhow!("Email attribute build error: {}", e))?,
        )
        .user_attributes(
            aws_sdk_cognitoidentityprovider::types::AttributeType::builder()
                .name("custom:family_id")
                .value(invitation.family_id.as_str())
                .build()
                .map_err(|e| anyhow::anyhow!("Family ID attribute build error: {}", e))?,
        )
        .user_attributes(
            aws_sdk_cognitoidentityprovider::types::AttributeType::builder()
                .name("custom:family_role")
                .value(invitation.role.as_str())
                .build()
                .map_err(|e| anyhow::anyhow!("Family role attribute build error: {}", e))?,
        )
        .user_attributes(
            aws_sdk_cognitoidentityprovider::types::AttributeType::builder()
                .name("custom:display_name")
                .value(display_name)
                .build()
                .map_err(|e| anyhow::anyhow!("Display name attribute build error: {}", e))?,
        )
        .message_action(aws_sdk_cognitoidentityprovider::types::MessageActionType::Suppress)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Cognito user creation error: {}", e))?;

    info!(
        "Cognitoユーザーを作成しました: user_id={}, email={}",
        user_id, invitation.email
    );

    Ok(user_id)
}

/// 招待メールを送信
async fn send_invitation_email(
    app_state: &AppState,
    invitation: &FamilyInvitation,
    display_name: &Option<String>,
) -> Result<()> {
    let frontend_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let invitation_url = format!(
        "{}/accept-invitation?token={}",
        frontend_url,
        invitation.invitation_token.as_str()
    );

    let subject = "家族 ToDo アプリへの招待";
    let body_text = format!(
        "家族 ToDo アプリへの招待\n\n{}さん、こんにちは！\n\n家族 ToDo アプリへの招待が届きました。\n\n以下のリンクをクリックして参加してください：\n{}\n\nこの招待は{}まで有効です。\n\n家族 ToDo アプリチーム",
        display_name.as_deref().unwrap_or(""),
        invitation_url,
        invitation.expires_at.format("%Y年%m月%d日 %H:%M")
    );

    let from_email =
        std::env::var("SES_FROM_EMAIL").unwrap_or_else(|_| "noreply@familytodo.app".to_string());

    app_state
        .ses_client
        .send_email()
        .source(&from_email)
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
                        .build()
                        .map_err(|e| anyhow::anyhow!("Subject content build error: {}", e))?,
                )
                .body(
                    aws_sdk_ses::types::Body::builder()
                        .text(
                            aws_sdk_ses::types::Content::builder()
                                .data(body_text)
                                .charset("UTF-8")
                                .build()
                                .map_err(|e| anyhow::anyhow!("Body content build error: {}", e))?,
                        )
                        .build(),
                )
                .build(),
        )
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("SES send email error: {}", e))?;

    info!("招待メールを送信しました: email={}", invitation.email);

    Ok(())
}

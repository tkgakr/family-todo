use http::HeaderMap;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

#[cfg(test)]
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub family_id: Option<String>,
    pub exp: i64,
    pub iat: i64,
    pub aud: String,
    pub iss: String,
    pub token_use: String,
    #[serde(rename = "cognito:groups")]
    pub cognito_groups: Option<Vec<String>>,
    #[serde(flatten)]
    pub custom: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwkKey {
    pub kty: String,
    pub kid: String,
    pub use_: Option<String>,
    pub n: String,
    pub e: String,
    pub alg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwkSet {
    pub keys: Vec<JwkKey>,
}

#[derive(Debug, Clone)]
pub struct JwtValidator {
    pub jwk_set: JwkSet,
    pub user_pool_id: String,
    pub client_id: String,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserId(pub String);

impl UserId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl JwtValidator {
    /// Cognito JWK エンドポイントから公開鍵を取得して JwtValidator を初期化
    pub async fn new(
        user_pool_id: &str,
        client_id: &str,
        region: &str,
    ) -> Result<Self, anyhow::Error> {
        let jwk_url = format!(
            "https://cognito-idp.{}.amazonaws.com/{}/.well-known/jwks.json",
            region, user_pool_id
        );

        info!("JWK エンドポイントから公開鍵を取得中: {}", jwk_url);

        let response = reqwest::get(&jwk_url).await?;
        let jwk_set: JwkSet = response.json().await?;

        info!("JWK セットを取得しました。キー数: {}", jwk_set.keys.len());

        Ok(Self {
            jwk_set,
            user_pool_id: user_pool_id.to_string(),
            client_id: client_id.to_string(),
            region: region.to_string(),
        })
    }

    /// JWT トークンを検証してクレームを取得
    pub fn validate_token(&self, token: &str) -> Result<Claims, anyhow::Error> {
        // JWT ヘッダーをデコードして kid を取得
        let header = decode_header(token)?;
        let kid = header
            .kid
            .ok_or_else(|| anyhow::anyhow!("JWT ヘッダーに kid が見つかりません"))?;

        // kid に対応する JWK を検索
        let jwk = self
            .jwk_set
            .keys
            .iter()
            .find(|key| key.kid == kid)
            .ok_or_else(|| anyhow::anyhow!("指定された kid の JWK が見つかりません: {}", kid))?;

        // RSA 公開鍵を構築
        let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)?;

        // JWT 検証設定
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.client_id]);
        validation.set_issuer(&[&format!(
            "https://cognito-idp.{}.amazonaws.com/{}",
            self.region, self.user_pool_id
        )]);

        // JWT を検証してクレームを取得
        let token_data = decode::<Claims>(token, &decoding_key, &validation)?;

        // 追加の検証
        self.validate_claims(&token_data.claims)?;

        Ok(token_data.claims)
    }

    /// クレームの追加検証
    fn validate_claims(&self, claims: &Claims) -> Result<(), anyhow::Error> {
        // トークンの用途を確認（access または id）
        if claims.token_use != "access" && claims.token_use != "id" {
            return Err(anyhow::anyhow!("無効なトークン用途: {}", claims.token_use));
        }

        // 現在時刻と比較して有効期限を確認
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        if claims.exp < now {
            return Err(anyhow::anyhow!("トークンの有効期限が切れています"));
        }

        // 発行時刻が未来でないことを確認
        if claims.iat > now + 300 {
            // 5分の猶予
            return Err(anyhow::anyhow!("トークンの発行時刻が無効です"));
        }

        Ok(())
    }
}

/// HTTP リクエストから JWT トークンを抽出してクレームを取得
pub async fn extract_user_claims_from_headers(
    headers: &HeaderMap,
    validator: &JwtValidator,
) -> Result<Claims, anyhow::Error> {
    // Authorization ヘッダーからトークンを抽出
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| anyhow::anyhow!("Authorization ヘッダーが見つかりません"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| anyhow::anyhow!("無効な Authorization ヘッダー形式"))?;

    // JWT トークンを検証
    validator.validate_token(token)
}

/// 家族 ID をクレームから抽出
pub fn extract_family_id_from_claims(claims: &Claims) -> Result<String, anyhow::Error> {
    // カスタム属性から family_id を取得
    if let Some(family_id) = &claims.family_id {
        return Ok(family_id.clone());
    }

    // Cognito グループから family_id を抽出
    if let Some(groups) = &claims.cognito_groups {
        for group in groups {
            if group.starts_with("family_") {
                return Ok(group.strip_prefix("family_").unwrap().to_string());
            }
        }
    }

    Err(anyhow::anyhow!(
        "ユーザーに家族 ID が関連付けられていません"
    ))
}

/// トークンリフレッシュ機能
pub async fn refresh_access_token(
    refresh_token: &str,
    client_id: &str,
    _region: &str,
) -> Result<RefreshTokenResponse, anyhow::Error> {
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    let cognito_client = aws_sdk_cognitoidentityprovider::Client::new(&config);

    let response = cognito_client
        .initiate_auth()
        .auth_flow(aws_sdk_cognitoidentityprovider::types::AuthFlowType::RefreshTokenAuth)
        .client_id(client_id)
        .auth_parameters("REFRESH_TOKEN", refresh_token)
        .send()
        .await?;

    let auth_result = response
        .authentication_result()
        .ok_or_else(|| anyhow::anyhow!("認証結果が取得できませんでした"))?;

    Ok(RefreshTokenResponse {
        access_token: auth_result
            .access_token()
            .ok_or_else(|| anyhow::anyhow!("アクセストークンが取得できませんでした"))?
            .to_string(),
        id_token: auth_result.id_token().map(|s| s.to_string()),
        expires_in: 3600, // デフォルト値を設定（AWS SDK の expires_in() の型が不明なため）
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub id_token: Option<String>,
    pub expires_in: i32,
}

#[cfg(test)]
pub mod test_utils {
    use super::*;

    /// テスト用の Authorizer 構造体
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TestAuthorizer {
        pub user_id: Option<String>,
        pub family_id: Option<String>,
        pub email: Option<String>,
        pub token_use: Option<String>,
        pub cognito_groups: Option<String>,
    }

    /// テスト用の API Gateway リクエスト構造体
    #[derive(Debug, Deserialize)]
    pub struct TestApiGatewayRequest {
        pub http_method: String,
        pub path: String,
        pub body: Option<String>,
        pub request_context: TestRequestContext,
    }

    #[derive(Debug, Deserialize)]
    pub struct TestRequestContext {
        pub authorizer: Option<TestAuthorizer>,
    }

    /// テスト用のクレームを作成
    pub fn create_test_claims() -> Claims {
        Claims {
            sub: "test-user-id".to_string(),
            email: "test@example.com".to_string(),
            family_id: Some("test-family-id".to_string()),
            exp: (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600) as i64,
            iat: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            aud: "test-client-id".to_string(),
            iss: "https://cognito-idp.us-east-1.amazonaws.com/us-east-1_test".to_string(),
            token_use: "access".to_string(),
            cognito_groups: Some(vec!["family-member".to_string()]),
            custom: HashMap::new(),
        }
    }

    /// テスト用のリクエストから認証情報を抽出
    pub fn extract_user_claims(request: &TestApiGatewayRequest) -> Result<Claims> {
        match &request.request_context.authorizer {
            Some(auth) => {
                let claims = Claims {
                    sub: auth.user_id.clone().unwrap_or_default(),
                    email: auth.email.clone().unwrap_or_default(),
                    family_id: auth.family_id.clone(),
                    exp: (SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        + 3600) as i64,
                    iat: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    aud: "test-client-id".to_string(),
                    iss: "https://cognito-idp.us-east-1.amazonaws.com/us-east-1_test".to_string(),
                    token_use: auth.token_use.clone().unwrap_or("access".to_string()),
                    cognito_groups: auth.cognito_groups.as_ref().map(|g| vec![g.clone()]),
                    custom: HashMap::new(),
                };
                Ok(claims)
            }
            None => Err(anyhow::anyhow!("認証情報が見つかりません")),
        }
    }
}

use http::HeaderMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub family_id: Option<String>,
    pub exp: i64,
    pub iat: i64,
    #[serde(flatten)]
    pub custom: HashMap<String, String>,
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

/// HTTP リクエストから JWT トークンを抽出してクレームを取得
pub fn extract_user_claims_from_headers(headers: &HeaderMap) -> Result<Claims, anyhow::Error> {
    // Authorization ヘッダーからトークンを抽出
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| anyhow::anyhow!("Authorization header not found"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| anyhow::anyhow!("Invalid authorization header format"))?;

    // JWT トークンをデコード（簡略化 - 実際には署名検証が必要）
    decode_jwt_claims(token)
}

/// JWT トークンからクレームをデコード
pub fn decode_jwt_claims(token: &str) -> Result<Claims, anyhow::Error> {
    // JWT の構造: header.payload.signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(anyhow::anyhow!("Invalid JWT format"));
    }

    // Base64 デコード
    let payload = base64_decode(parts[1])?;
    let claims: Claims = serde_json::from_slice(&payload)?;

    // トークンの有効期限をチェック
    let now = chrono::Utc::now().timestamp();
    if claims.exp < now {
        return Err(anyhow::anyhow!("Token has expired"));
    }

    Ok(claims)
}

/// Base64 URL セーフデコード
fn base64_decode(input: &str) -> Result<Vec<u8>, anyhow::Error> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

    // パディングを追加
    let mut padded = input.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }

    URL_SAFE_NO_PAD
        .decode(padded.as_bytes())
        .map_err(|e| anyhow::anyhow!("Base64 decode error: {}", e))
}

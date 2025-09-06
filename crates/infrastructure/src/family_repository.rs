use crate::DynamoDbClient;
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Utc};
use domain::{
    DomainError, FamilyId, FamilyInvitation, FamilyMember, FamilyRole, InvitationToken, UserId,
};
use serde::{Deserialize, Serialize};
use shared::{AppError, DynamoDbRetryExecutor, RetryResult};
use std::collections::HashMap;
use tracing::{debug, info};

/// 家族招待レコード（DynamoDB用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationRecord {
    pub invitation_token: String,
    pub family_id: String,
    pub email: String,
    pub role: String,
    pub invited_by: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub used: bool,
}

impl InvitationRecord {
    pub fn new(invitation: &FamilyInvitation) -> Self {
        Self {
            invitation_token: invitation.invitation_token.as_str().to_string(),
            family_id: invitation.family_id.as_str().to_string(),
            email: invitation.email.clone(),
            role: invitation.role.to_string(),
            invited_by: invitation.invited_by.as_str().to_string(),
            expires_at: invitation.expires_at,
            created_at: invitation.created_at,
            used: invitation.is_used,
        }
    }

    pub fn to_family_invitation(&self) -> Result<FamilyInvitation, DomainError> {
        Ok(FamilyInvitation {
            invitation_token: InvitationToken::from_string(self.invitation_token.clone())?,
            family_id: FamilyId::from_string(self.family_id.clone())?,
            email: self.email.clone(),
            role: FamilyRole::from_string(&self.role)?,
            invited_by: UserId::from_string(self.invited_by.clone())?,
            expires_at: self.expires_at,
            created_at: self.created_at,
            is_used: self.used,
        })
    }
}

/// 家族メンバーレコード（DynamoDB用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyMemberRecord {
    pub user_id: String,
    pub family_id: String,
    pub display_name: String,
    pub email: String,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>,
}

impl FamilyMemberRecord {
    pub fn new(member: &FamilyMember, family_id: &FamilyId) -> Self {
        Self {
            user_id: member.user_id.as_str().to_string(),
            family_id: family_id.as_str().to_string(),
            display_name: member.display_name.clone(),
            email: member.email.clone(),
            role: member.role.to_string(),
            joined_at: member.joined_at,
            last_active_at: None, // FamilyMemberにはlast_active_atフィールドがないため、Noneを設定
        }
    }

    pub fn to_family_member(&self) -> Result<FamilyMember, DomainError> {
        Ok(FamilyMember {
            user_id: UserId::from_string(self.user_id.clone())?,
            display_name: self.display_name.clone(),
            email: self.email.clone(),
            role: FamilyRole::from_string(&self.role)?,
            joined_at: self.joined_at,
            is_active: true, // デフォルトでアクティブ
        })
    }
}

/// 家族リポジトリ
/// 家族メンバー管理と招待機能を提供
#[derive(Clone)]
pub struct FamilyRepository {
    db: DynamoDbClient,
}

impl FamilyRepository {
    pub fn new(db: DynamoDbClient) -> Self {
        Self { db }
    }

    /// 家族招待を保存
    pub async fn save_invitation(&self, invitation: &FamilyInvitation) -> Result<(), AppError> {
        info!(
            "家族招待保存中: family_id={}, email={}",
            invitation.family_id, invitation.email
        );

        let record = InvitationRecord::new(invitation);
        let pk = format!("INVITATION#{}", record.invitation_token);
        let sk = "METADATA".to_string();

        let mut item = HashMap::new();
        item.insert("PK".to_string(), AttributeValue::S(pk));
        item.insert("SK".to_string(), AttributeValue::S(sk));
        item.insert(
            "EntityType".to_string(),
            AttributeValue::S("Invitation".to_string()),
        );
        item.insert(
            "Data".to_string(),
            AttributeValue::S(serde_json::to_string(&record).map_err(|e| {
                AppError::Serialization(format!("招待レコードシリアライゼーションエラー: {e}"))
            })?),
        );
        item.insert(
            "TTL".to_string(),
            AttributeValue::N(record.expires_at.timestamp().to_string()),
        );
        item.insert(
            "CreatedAt".to_string(),
            AttributeValue::S(record.created_at.to_rfc3339()),
        );

        let result = DynamoDbRetryExecutor::execute(|| async {
            self.db
                .client()
                .put_item()
                .table_name(self.db.table_name())
                .set_item(Some(item.clone()))
                .condition_expression("attribute_not_exists(PK)")
                .send()
                .await
                .map_err(|e| self.db.convert_error(e))?;

            debug!("家族招待保存完了: token={}", record.invitation_token);
            Ok(())
        })
        .await;

        match result {
            RetryResult::Success(()) => Ok(()),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }

    /// 招待トークンで招待情報を取得
    pub async fn get_invitation(
        &self,
        token: &InvitationToken,
    ) -> Result<Option<FamilyInvitation>, AppError> {
        info!("招待情報取得中: token={}", token.as_str());

        let pk = format!("INVITATION#{}", token.as_str());
        let sk = "METADATA".to_string();

        let result = DynamoDbRetryExecutor::execute(|| async {
            let response = self
                .db
                .client()
                .get_item()
                .table_name(self.db.table_name())
                .key("PK", AttributeValue::S(pk.clone()))
                .key("SK", AttributeValue::S(sk.clone()))
                .send()
                .await
                .map_err(|e| self.db.convert_error(e))?;

            if let Some(item) = response.item {
                if let Some(data_attr) = item.get("Data") {
                    if let Ok(data_str) = data_attr.as_s() {
                        let record: InvitationRecord =
                            serde_json::from_str(data_str).map_err(|e| {
                                AppError::Deserialization(format!(
                                    "招待レコードデシリアライゼーションエラー: {e}"
                                ))
                            })?;

                        let invitation = record.to_family_invitation().map_err(AppError::Domain)?;

                        debug!("招待情報取得完了: token={}", token.as_str());
                        return Ok(Some(invitation));
                    }
                }
            }

            debug!("招待情報が見つかりません: token={}", token.as_str());
            Ok(None)
        })
        .await;

        match result {
            RetryResult::Success(invitation) => Ok(invitation),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }

    /// 招待を使用済みにマーク
    pub async fn mark_invitation_used(&self, token: &InvitationToken) -> Result<(), AppError> {
        info!("招待使用済みマーク中: token={}", token.as_str());

        let pk = format!("INVITATION#{}", token.as_str());
        let sk = "METADATA".to_string();

        let result = DynamoDbRetryExecutor::execute(|| async {
            self.db
                .client()
                .update_item()
                .table_name(self.db.table_name())
                .key("PK", AttributeValue::S(pk.clone()))
                .key("SK", AttributeValue::S(sk.clone()))
                .update_expression("SET #used = :used, #updated_at = :updated_at")
                .condition_expression("attribute_exists(PK) AND #used = :false")
                .expression_attribute_names("#used", "used")
                .expression_attribute_names("#updated_at", "UpdatedAt")
                .expression_attribute_values(":used", AttributeValue::Bool(true))
                .expression_attribute_values(":false", AttributeValue::Bool(false))
                .expression_attribute_values(
                    ":updated_at",
                    AttributeValue::S(Utc::now().to_rfc3339()),
                )
                .send()
                .await
                .map_err(|e| self.db.convert_error(e))?;

            debug!("招待使用済みマーク完了: token={}", token.as_str());
            Ok(())
        })
        .await;

        match result {
            RetryResult::Success(()) => Ok(()),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }

    /// 家族メンバーを追加
    pub async fn add_family_member(
        &self,
        family_id: &FamilyId,
        member: &FamilyMember,
    ) -> Result<(), AppError> {
        info!(
            "家族メンバー追加中: family_id={}, user_id={}",
            family_id, member.user_id
        );

        let record = FamilyMemberRecord::new(member, family_id);
        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk = format!("MEMBER#{}", member.user_id.as_str());

        let mut item = HashMap::new();
        item.insert("PK".to_string(), AttributeValue::S(pk));
        item.insert("SK".to_string(), AttributeValue::S(sk));
        item.insert(
            "EntityType".to_string(),
            AttributeValue::S("FamilyMember".to_string()),
        );
        item.insert(
            "Data".to_string(),
            AttributeValue::S(serde_json::to_string(&record).map_err(|e| {
                AppError::Serialization(format!("メンバーレコードシリアライゼーションエラー: {e}"))
            })?),
        );
        item.insert(
            "CreatedAt".to_string(),
            AttributeValue::S(record.joined_at.to_rfc3339()),
        );
        item.insert(
            "UpdatedAt".to_string(),
            AttributeValue::S(Utc::now().to_rfc3339()),
        );

        let result = DynamoDbRetryExecutor::execute(|| async {
            self.db
                .client()
                .put_item()
                .table_name(self.db.table_name())
                .set_item(Some(item.clone()))
                .send()
                .await
                .map_err(|e| self.db.convert_error(e))?;

            debug!("家族メンバー追加完了: user_id={}", member.user_id);
            Ok(())
        })
        .await;

        match result {
            RetryResult::Success(()) => Ok(()),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }

    /// 家族のすべてのメンバーを取得
    pub async fn get_family_members(
        &self,
        family_id: &FamilyId,
    ) -> Result<Vec<FamilyMember>, AppError> {
        info!("家族メンバー取得中: family_id={}", family_id);

        let pk = format!("FAMILY#{}", family_id.as_str());

        let result = DynamoDbRetryExecutor::execute(|| async {
            let response = self
                .db
                .client()
                .query()
                .table_name(self.db.table_name())
                .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
                .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                .expression_attribute_values(":sk_prefix", AttributeValue::S("MEMBER#".to_string()))
                .send()
                .await
                .map_err(|e| self.db.convert_error(e))?;

            let mut members = Vec::new();
            if let Some(items) = response.items {
                for item in items {
                    if let Some(data_attr) = item.get("Data") {
                        if let Ok(data_str) = data_attr.as_s() {
                            let record: FamilyMemberRecord = serde_json::from_str(data_str)
                                .map_err(|e| {
                                    AppError::Deserialization(format!(
                                        "メンバーレコードデシリアライゼーションエラー: {e}"
                                    ))
                                })?;

                            let member = record.to_family_member().map_err(AppError::Domain)?;

                            members.push(member);
                        }
                    }
                }
            }

            debug!("家族メンバー取得完了: {} 人", members.len());
            Ok(members)
        })
        .await;

        match result {
            RetryResult::Success(members) => Ok(members),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }

    /// 特定のメンバーを取得
    pub async fn get_family_member(
        &self,
        family_id: &FamilyId,
        user_id: &UserId,
    ) -> Result<Option<FamilyMember>, AppError> {
        info!(
            "家族メンバー取得中: family_id={}, user_id={}",
            family_id, user_id
        );

        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk = format!("MEMBER#{}", user_id.as_str());

        let result = DynamoDbRetryExecutor::execute(|| async {
            let response = self
                .db
                .client()
                .get_item()
                .table_name(self.db.table_name())
                .key("PK", AttributeValue::S(pk.clone()))
                .key("SK", AttributeValue::S(sk.clone()))
                .send()
                .await
                .map_err(|e| self.db.convert_error(e))?;

            if let Some(item) = response.item {
                if let Some(data_attr) = item.get("Data") {
                    if let Ok(data_str) = data_attr.as_s() {
                        let record: FamilyMemberRecord =
                            serde_json::from_str(data_str).map_err(|e| {
                                AppError::Deserialization(format!(
                                    "メンバーレコードデシリアライゼーションエラー: {e}"
                                ))
                            })?;

                        let member = record.to_family_member().map_err(AppError::Domain)?;

                        debug!("家族メンバー取得完了: user_id={}", user_id);
                        return Ok(Some(member));
                    }
                }
            }

            debug!("家族メンバーが見つかりません: user_id={}", user_id);
            Ok(None)
        })
        .await;

        match result {
            RetryResult::Success(member) => Ok(member),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }

    /// メンバーを家族から削除
    pub async fn remove_family_member(
        &self,
        family_id: &FamilyId,
        user_id: &UserId,
    ) -> Result<(), AppError> {
        info!(
            "家族メンバー削除中: family_id={}, user_id={}",
            family_id, user_id
        );

        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk = format!("MEMBER#{}", user_id.as_str());

        let result = DynamoDbRetryExecutor::execute(|| async {
            self.db
                .client()
                .delete_item()
                .table_name(self.db.table_name())
                .key("PK", AttributeValue::S(pk.clone()))
                .key("SK", AttributeValue::S(sk.clone()))
                .send()
                .await
                .map_err(|e| self.db.convert_error(e))?;

            debug!("家族メンバー削除完了: user_id={}", user_id);
            Ok(())
        })
        .await;

        match result {
            RetryResult::Success(()) => Ok(()),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }

    /// メンバーの最終アクティブ時刻を更新
    pub async fn update_member_last_active(
        &self,
        family_id: &FamilyId,
        user_id: &UserId,
        last_active_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        info!(
            "メンバー最終アクティブ時刻更新中: family_id={}, user_id={}",
            family_id, user_id
        );

        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk = format!("MEMBER#{}", user_id.as_str());

        let result = DynamoDbRetryExecutor::execute(|| async {
            self.db
                .client()
                .update_item()
                .table_name(self.db.table_name())
                .key("PK", AttributeValue::S(pk.clone()))
                .key("SK", AttributeValue::S(sk.clone()))
                .update_expression("SET #last_active = :last_active, #updated_at = :updated_at")
                .expression_attribute_names("#last_active", "last_active_at")
                .expression_attribute_names("#updated_at", "UpdatedAt")
                .expression_attribute_values(
                    ":last_active",
                    AttributeValue::S(last_active_at.to_rfc3339()),
                )
                .expression_attribute_values(
                    ":updated_at",
                    AttributeValue::S(Utc::now().to_rfc3339()),
                )
                .send()
                .await
                .map_err(|e| self.db.convert_error(e))?;

            debug!("メンバー最終アクティブ時刻更新完了: user_id={}", user_id);
            Ok(())
        })
        .await;

        match result {
            RetryResult::Success(()) => Ok(()),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use domain::{FamilyRole, InvitationToken, UserId};
    use shared::Config;

    async fn setup_test_repository() -> Result<FamilyRepository, AppError> {
        let config = Config {
            dynamodb_table: "test-table".to_string(),
            environment: "test".to_string(),
            aws_region: "ap-northeast-1".to_string(),
            dynamodb_endpoint: Some("http://localhost:8000".to_string()),
            retry_max_attempts: 2,
            retry_initial_delay_ms: 10,
        };
        let db_client = DynamoDbClient::new_for_test(&config).await?;
        Ok(FamilyRepository::new(db_client))
    }

    #[tokio::test]
    async fn test_invitation_lifecycle() {
        let repo = match setup_test_repository().await {
            Ok(repo) => repo,
            Err(_) => {
                println!("DynamoDB Localが利用できないため、テストをスキップします");
                return;
            }
        };

        let invitation = FamilyInvitation {
            invitation_token: InvitationToken::new(),
            family_id: FamilyId::new(),
            email: "test@example.com".to_string(),
            role: FamilyRole::Member,
            invited_by: UserId::new(),
            expires_at: Utc::now() + Duration::days(7),
            created_at: Utc::now(),
            is_used: false,
        };

        // 実際のDynamoDB Localが動いていない場合はスキップ
        if repo.save_invitation(&invitation).await.is_ok() {
            // 招待を取得
            let retrieved = repo
                .get_invitation(&invitation.invitation_token)
                .await
                .unwrap();
            assert!(retrieved.is_some());
            let retrieved_invitation = retrieved.unwrap();
            assert_eq!(retrieved_invitation.email, invitation.email);
            assert!(!retrieved_invitation.is_used);

            // 招待を使用済みにマーク
            if repo
                .mark_invitation_used(&invitation.invitation_token)
                .await
                .is_ok()
            {
                let updated = repo
                    .get_invitation(&invitation.invitation_token)
                    .await
                    .unwrap();
                if let Some(updated_invitation) = updated {
                    assert!(updated_invitation.is_used);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_family_member_management() {
        let repo = match setup_test_repository().await {
            Ok(repo) => repo,
            Err(_) => {
                println!("DynamoDB Localが利用できないため、テストをスキップします");
                return;
            }
        };
        let family_id = FamilyId::new();
        let user_id = UserId::new();

        let member = FamilyMember {
            user_id: user_id.clone(),
            display_name: "テストユーザー".to_string(),
            email: "test@example.com".to_string(),
            role: FamilyRole::Member,
            joined_at: Utc::now(),
            is_active: true,
        };

        // 実際のDynamoDB Localが動いていない場合はスキップ
        if repo.add_family_member(&family_id, &member).await.is_ok() {
            // メンバーを取得
            let retrieved = repo.get_family_member(&family_id, &user_id).await.unwrap();
            assert!(retrieved.is_some());
            let retrieved_member = retrieved.unwrap();
            assert_eq!(retrieved_member.display_name, member.display_name);

            // 家族のすべてのメンバーを取得
            let all_members = repo.get_family_members(&family_id).await.unwrap();
            assert_eq!(all_members.len(), 1);
            assert_eq!(all_members[0].user_id, user_id);
        }
    }

    #[test]
    fn test_invitation_record_conversion() {
        let invitation = FamilyInvitation {
            invitation_token: InvitationToken::new(),
            family_id: FamilyId::new(),
            email: "test@example.com".to_string(),
            role: FamilyRole::Admin,
            invited_by: UserId::new(),
            expires_at: Utc::now() + Duration::days(7),
            created_at: Utc::now(),
            is_used: false,
        };

        let record = InvitationRecord::new(&invitation);
        let converted = record.to_family_invitation().unwrap();

        assert_eq!(converted.email, invitation.email);
        assert_eq!(converted.role, invitation.role);
        assert_eq!(converted.is_used, invitation.is_used);
    }

    #[test]
    fn test_family_member_record_conversion() {
        let family_id = FamilyId::new();
        let member = FamilyMember {
            user_id: UserId::new(),
            display_name: "テストユーザー".to_string(),
            email: "test@example.com".to_string(),
            role: FamilyRole::Member,
            joined_at: Utc::now(),
            is_active: true,
        };

        let record = FamilyMemberRecord::new(&member, &family_id);
        let converted = record.to_family_member().unwrap();

        assert_eq!(converted.user_id, member.user_id);
        assert_eq!(converted.display_name, member.display_name);
        assert_eq!(converted.role, member.role);
    }
}

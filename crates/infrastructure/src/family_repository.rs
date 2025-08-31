use crate::{retry_dynamodb_operation, DynamoDbClient};
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Utc};
use domain::{
    DomainError, FamilyEvent, FamilyId, FamilyInvitation, FamilyMember, InvitationToken, UserId,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info};

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
    pub is_used: bool,
}

impl From<&FamilyInvitation> for InvitationRecord {
    fn from(invitation: &FamilyInvitation) -> Self {
        Self {
            invitation_token: invitation.invitation_token.as_str().to_string(),
            family_id: invitation.family_id.as_str().to_string(),
            email: invitation.email.clone(),
            role: invitation.role.as_str().to_string(),
            invited_by: invitation.invited_by.as_str().to_string(),
            expires_at: invitation.expires_at,
            created_at: invitation.created_at,
            is_used: invitation.is_used,
        }
    }
}

impl TryFrom<InvitationRecord> for FamilyInvitation {
    type Error = DomainError;

    fn try_from(record: InvitationRecord) -> Result<Self, Self::Error> {
        Ok(FamilyInvitation {
            invitation_token: InvitationToken::from_string(record.invitation_token)?,
            family_id: FamilyId::from_string(record.family_id)?,
            email: record.email,
            role: domain::FamilyRole::from_string(&record.role)?,
            invited_by: UserId::from_string(record.invited_by)?,
            expires_at: record.expires_at,
            created_at: record.created_at,
            is_used: record.is_used,
        })
    }
}

/// 家族メンバーレコード（DynamoDB用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberRecord {
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    pub is_active: bool,
}

impl From<&FamilyMember> for MemberRecord {
    fn from(member: &FamilyMember) -> Self {
        Self {
            user_id: member.user_id.as_str().to_string(),
            email: member.email.clone(),
            display_name: member.display_name.clone(),
            role: member.role.as_str().to_string(),
            joined_at: member.joined_at,
            is_active: member.is_active,
        }
    }
}

impl TryFrom<MemberRecord> for FamilyMember {
    type Error = DomainError;

    fn try_from(record: MemberRecord) -> Result<Self, Self::Error> {
        Ok(FamilyMember {
            user_id: UserId::from_string(record.user_id)?,
            email: record.email,
            display_name: record.display_name,
            role: domain::FamilyRole::from_string(&record.role)?,
            joined_at: record.joined_at,
            is_active: record.is_active,
        })
    }
}

/// 家族イベントリポジトリ
/// 家族メンバー管理に関するイベントの保存・取得機能を提供
#[derive(Clone)]
pub struct FamilyEventRepository {
    db: DynamoDbClient,
}

impl FamilyEventRepository {
    pub fn new(db: DynamoDbClient) -> Self {
        Self { db }
    }

    /// 家族イベントを保存する
    pub async fn save_family_event(&self, event: FamilyEvent) -> Result<(), DomainError> {
        info!(
            "家族イベントを保存中: family_id={}, event_id={}",
            event.family_id(),
            event.event_id()
        );

        let pk = format!("FAMILY#{}", event.family_id());
        let sk = format!("EVENT#{}", event.event_id());
        let event_json = event.to_json()?;

        let mut item = HashMap::new();
        item.insert("PK".to_string(), AttributeValue::S(pk));
        item.insert("SK".to_string(), AttributeValue::S(sk));
        item.insert(
            "EntityType".to_string(),
            AttributeValue::S("FamilyEvent".to_string()),
        );
        item.insert("Data".to_string(), AttributeValue::S(event_json));
        item.insert(
            "CreatedAt".to_string(),
            AttributeValue::S(event.timestamp().to_rfc3339()),
        );

        retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .put_item()
                    .table_name(self.db.table_name())
                    .set_item(Some(item.clone()))
                    .condition_expression("attribute_not_exists(PK) AND attribute_not_exists(SK)")
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("家族イベント保存完了: {}", event.event_id());
                Ok(())
            },
            None,
        )
        .await
        .map_err(|e| DomainError::Validation(format!("家族イベント保存エラー: {e}")))
    }

    /// 家族の全イベントを取得する
    pub async fn get_family_events(
        &self,
        family_id: &str,
    ) -> Result<Vec<FamilyEvent>, DomainError> {
        info!("家族イベントを取得中: family_id={}", family_id);

        let pk = format!("FAMILY#{}", family_id);

        let result = retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .query()
                    .table_name(self.db.table_name())
                    .key_condition_expression("PK = :pk AND begins_with(SK, :sk)")
                    .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                    .expression_attribute_values(":sk", AttributeValue::S("EVENT#".to_string()))
                    .filter_expression("EntityType = :entity_type")
                    .expression_attribute_values(
                        ":entity_type",
                        AttributeValue::S("FamilyEvent".to_string()),
                    )
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))
            },
            None,
        )
        .await
        .map_err(|e| DomainError::Validation(format!("家族イベント取得エラー: {e}")))?;

        let mut events = Vec::new();
        for item in result.items.unwrap_or_default() {
            if let Some(data) = item.get("Data").and_then(|v| v.as_s().ok()) {
                match serde_json::from_str::<FamilyEvent>(data) {
                    Ok(event) => events.push(event),
                    Err(e) => {
                        error!("家族イベントデシリアライゼーションエラー: {}", e);
                        continue;
                    }
                }
            }
        }

        debug!("家族イベント取得完了: {} 件", events.len());
        Ok(events)
    }
}

/// 家族招待リポジトリ
/// 招待の保存・取得・削除機能を提供
#[derive(Clone)]
pub struct FamilyInvitationRepository {
    db: DynamoDbClient,
}

impl FamilyInvitationRepository {
    pub fn new(db: DynamoDbClient) -> Self {
        Self { db }
    }

    /// 招待を保存する
    pub async fn save_invitation(&self, invitation: &FamilyInvitation) -> Result<(), DomainError> {
        info!(
            "招待を保存中: family_id={}, token={}",
            invitation.family_id.as_str(),
            invitation.invitation_token.as_str()
        );

        let record = InvitationRecord::from(invitation);
        let pk = format!("FAMILY#{}", record.family_id);
        let sk = format!("INVITATION#{}", record.invitation_token);
        let ttl = record.expires_at.timestamp();

        let mut item = HashMap::new();
        item.insert("PK".to_string(), AttributeValue::S(pk));
        item.insert("SK".to_string(), AttributeValue::S(sk));
        item.insert(
            "EntityType".to_string(),
            AttributeValue::S("Invitation".to_string()),
        );
        item.insert(
            "Data".to_string(),
            AttributeValue::S(
                serde_json::to_string(&record)
                    .map_err(|e| DomainError::EventSerialization(e.to_string()))?,
            ),
        );
        item.insert("TTL".to_string(), AttributeValue::N(ttl.to_string()));
        item.insert(
            "CreatedAt".to_string(),
            AttributeValue::S(record.created_at.to_rfc3339()),
        );

        // GSI1 for token-based lookup
        item.insert(
            "GSI1PK".to_string(),
            AttributeValue::S(format!("INVITATION#{}", record.invitation_token)),
        );
        item.insert(
            "GSI1SK".to_string(),
            AttributeValue::S(record.family_id.clone()),
        );

        retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .put_item()
                    .table_name(self.db.table_name())
                    .set_item(Some(item.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("招待保存完了: {}", record.invitation_token);
                Ok(())
            },
            None,
        )
        .await
        .map_err(|e| DomainError::Validation(format!("招待保存エラー: {e}")))
    }

    /// 招待トークンで招待を取得する
    pub async fn get_invitation_by_token(
        &self,
        token: &str,
    ) -> Result<Option<FamilyInvitation>, DomainError> {
        info!("招待を取得中: token={}", token);

        let result = retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .query()
                    .table_name(self.db.table_name())
                    .index_name("GSI1")
                    .key_condition_expression("GSI1PK = :pk")
                    .expression_attribute_values(
                        ":pk",
                        AttributeValue::S(format!("INVITATION#{}", token)),
                    )
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))
            },
            None,
        )
        .await
        .map_err(|e| DomainError::Validation(format!("招待取得エラー: {e}")))?;

        let items = result.items.unwrap_or_default();
        if items.is_empty() {
            debug!("招待が見つかりません: token={}", token);
            return Ok(None);
        }

        let item = &items[0];
        if let Some(data) = item.get("Data").and_then(|v| v.as_s().ok()) {
            let record: InvitationRecord = serde_json::from_str(data)
                .map_err(|e| DomainError::EventDeserialization(e.to_string()))?;
            let invitation = FamilyInvitation::try_from(record)?;
            debug!("招待取得完了: token={}", token);
            Ok(Some(invitation))
        } else {
            error!("招待データが無効です: token={}", token);
            Err(DomainError::InvalidEvent(
                "Invalid invitation data".to_string(),
            ))
        }
    }

    /// 家族の招待一覧を取得する
    pub async fn list_family_invitations(
        &self,
        family_id: &str,
    ) -> Result<Vec<FamilyInvitation>, DomainError> {
        info!("家族の招待一覧を取得中: family_id={}", family_id);

        let pk = format!("FAMILY#{}", family_id);

        let result = retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .query()
                    .table_name(self.db.table_name())
                    .key_condition_expression("PK = :pk AND begins_with(SK, :sk)")
                    .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                    .expression_attribute_values(
                        ":sk",
                        AttributeValue::S("INVITATION#".to_string()),
                    )
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))
            },
            None,
        )
        .await
        .map_err(|e| DomainError::Validation(format!("招待一覧取得エラー: {e}")))?;

        let mut invitations = Vec::new();
        for item in result.items.unwrap_or_default() {
            if let Some(data) = item.get("Data").and_then(|v| v.as_s().ok()) {
                match serde_json::from_str::<InvitationRecord>(data) {
                    Ok(record) => match FamilyInvitation::try_from(record) {
                        Ok(invitation) => invitations.push(invitation),
                        Err(e) => {
                            error!("招待変換エラー: {}", e);
                            continue;
                        }
                    },
                    Err(e) => {
                        error!("招待デシリアライゼーションエラー: {}", e);
                        continue;
                    }
                }
            }
        }

        debug!("招待一覧取得完了: {} 件", invitations.len());
        Ok(invitations)
    }

    /// 招待を削除する
    pub async fn delete_invitation(&self, family_id: &str, token: &str) -> Result<(), DomainError> {
        info!("招待を削除中: family_id={}, token={}", family_id, token);

        let pk = format!("FAMILY#{}", family_id);
        let sk = format!("INVITATION#{}", token);

        retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .delete_item()
                    .table_name(self.db.table_name())
                    .key("PK", AttributeValue::S(pk.clone()))
                    .key("SK", AttributeValue::S(sk.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("招待削除完了: token={}", token);
                Ok(())
            },
            None,
        )
        .await
        .map_err(|e| DomainError::Validation(format!("招待削除エラー: {e}")))
    }
}

/// 家族メンバーリポジトリ
/// メンバーの保存・取得・更新機能を提供
#[derive(Clone)]
pub struct FamilyMemberRepository {
    db: DynamoDbClient,
}

impl FamilyMemberRepository {
    pub fn new(db: DynamoDbClient) -> Self {
        Self { db }
    }

    /// メンバーを保存する
    pub async fn save_member(
        &self,
        family_id: &str,
        member: &FamilyMember,
    ) -> Result<(), DomainError> {
        info!(
            "メンバーを保存中: family_id={}, user_id={}",
            family_id,
            member.user_id.as_str()
        );

        let record = MemberRecord::from(member);
        let pk = format!("FAMILY#{}", family_id);
        let sk = format!("MEMBER#{}", record.user_id);

        let mut item = HashMap::new();
        item.insert("PK".to_string(), AttributeValue::S(pk));
        item.insert("SK".to_string(), AttributeValue::S(sk));
        item.insert(
            "EntityType".to_string(),
            AttributeValue::S("Member".to_string()),
        );
        item.insert(
            "Data".to_string(),
            AttributeValue::S(
                serde_json::to_string(&record)
                    .map_err(|e| DomainError::EventSerialization(e.to_string()))?,
            ),
        );
        item.insert(
            "CreatedAt".to_string(),
            AttributeValue::S(record.joined_at.to_rfc3339()),
        );

        // GSI1 for user-based lookup
        item.insert(
            "GSI1PK".to_string(),
            AttributeValue::S(format!("USER#{}", record.user_id)),
        );
        item.insert(
            "GSI1SK".to_string(),
            AttributeValue::S(family_id.to_string()),
        );

        retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .put_item()
                    .table_name(self.db.table_name())
                    .set_item(Some(item.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("メンバー保存完了: {}", record.user_id);
                Ok(())
            },
            None,
        )
        .await
        .map_err(|e| DomainError::Validation(format!("メンバー保存エラー: {e}")))
    }

    /// 家族のメンバー一覧を取得する
    pub async fn list_family_members(
        &self,
        family_id: &str,
    ) -> Result<Vec<FamilyMember>, DomainError> {
        info!("家族メンバー一覧を取得中: family_id={}", family_id);

        let pk = format!("FAMILY#{}", family_id);

        let result = retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .query()
                    .table_name(self.db.table_name())
                    .key_condition_expression("PK = :pk AND begins_with(SK, :sk)")
                    .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                    .expression_attribute_values(":sk", AttributeValue::S("MEMBER#".to_string()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))
            },
            None,
        )
        .await
        .map_err(|e| DomainError::Validation(format!("メンバー一覧取得エラー: {e}")))?;

        let mut members = Vec::new();
        for item in result.items.unwrap_or_default() {
            if let Some(data) = item.get("Data").and_then(|v| v.as_s().ok()) {
                match serde_json::from_str::<MemberRecord>(data) {
                    Ok(record) => match FamilyMember::try_from(record) {
                        Ok(member) => members.push(member),
                        Err(e) => {
                            error!("メンバー変換エラー: {}", e);
                            continue;
                        }
                    },
                    Err(e) => {
                        error!("メンバーデシリアライゼーションエラー: {}", e);
                        continue;
                    }
                }
            }
        }

        debug!("メンバー一覧取得完了: {} 件", members.len());
        Ok(members)
    }

    /// ユーザーIDでメンバーを取得する
    pub async fn get_member_by_user_id(
        &self,
        family_id: &str,
        user_id: &str,
    ) -> Result<Option<FamilyMember>, DomainError> {
        info!(
            "メンバーを取得中: family_id={}, user_id={}",
            family_id, user_id
        );

        let pk = format!("FAMILY#{}", family_id);
        let sk = format!("MEMBER#{}", user_id);

        let result = retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .get_item()
                    .table_name(self.db.table_name())
                    .key("PK", AttributeValue::S(pk.clone()))
                    .key("SK", AttributeValue::S(sk.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))
            },
            None,
        )
        .await
        .map_err(|e| DomainError::Validation(format!("メンバー取得エラー: {e}")))?;

        if let Some(item) = result.item {
            if let Some(data) = item.get("Data").and_then(|v| v.as_s().ok()) {
                let record: MemberRecord = serde_json::from_str(data)
                    .map_err(|e| DomainError::EventDeserialization(e.to_string()))?;
                let member = FamilyMember::try_from(record)?;
                debug!("メンバー取得完了: user_id={}", user_id);
                Ok(Some(member))
            } else {
                error!("メンバーデータが無効です: user_id={}", user_id);
                Err(DomainError::InvalidEvent("Invalid member data".to_string()))
            }
        } else {
            debug!("メンバーが見つかりません: user_id={}", user_id);
            Ok(None)
        }
    }

    /// メンバーを削除する
    pub async fn delete_member(&self, family_id: &str, user_id: &str) -> Result<(), DomainError> {
        info!(
            "メンバーを削除中: family_id={}, user_id={}",
            family_id, user_id
        );

        let pk = format!("FAMILY#{}", family_id);
        let sk = format!("MEMBER#{}", user_id);

        retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .delete_item()
                    .table_name(self.db.table_name())
                    .key("PK", AttributeValue::S(pk.clone()))
                    .key("SK", AttributeValue::S(sk.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("メンバー削除完了: user_id={}", user_id);
                Ok(())
            },
            None,
        )
        .await
        .map_err(|e| DomainError::Validation(format!("メンバー削除エラー: {e}")))
    }
}

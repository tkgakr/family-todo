use crate::errors::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 家族ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FamilyId(String);

impl FamilyId {
    /// 新しい家族IDを生成
    pub fn new() -> Self {
        Self(ulid::Ulid::new().to_string())
    }

    /// 文字列から家族IDを作成
    pub fn from_string(id: String) -> Result<Self, DomainError> {
        if id.is_empty() {
            return Err(DomainError::InvalidFamilyId(
                "Family ID cannot be empty".to_string(),
            ));
        }
        Ok(Self(id))
    }

    /// 家族IDを文字列として取得
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 家族IDが有効かチェック
    pub fn is_valid(&self) -> bool {
        !self.0.is_empty()
    }
}

impl Default for FamilyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FamilyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ユーザーID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(String);

impl UserId {
    /// 新しいユーザーIDを生成
    pub fn new() -> Self {
        Self(ulid::Ulid::new().to_string())
    }

    /// 文字列からユーザーIDを作成
    pub fn from_string(id: String) -> Result<Self, DomainError> {
        if id.is_empty() {
            return Err(DomainError::InvalidUserId(
                "User ID cannot be empty".to_string(),
            ));
        }
        Ok(Self(id))
    }

    /// ユーザーIDを文字列として取得
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// ユーザーIDが有効かチェック
    pub fn is_valid(&self) -> bool {
        !self.0.is_empty()
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 家族メンバーの役割
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FamilyRole {
    Admin,
    Member,
}

impl FamilyRole {
    /// 文字列から役割を作成
    pub fn from_string(role: &str) -> Result<Self, DomainError> {
        match role.to_lowercase().as_str() {
            "admin" => Ok(FamilyRole::Admin),
            "member" => Ok(FamilyRole::Member),
            _ => Err(DomainError::InvalidFamilyRole(format!(
                "Invalid role: {role}"
            ))),
        }
    }

    /// 役割を文字列として取得
    pub fn as_str(&self) -> &'static str {
        match self {
            FamilyRole::Admin => "admin",
            FamilyRole::Member => "member",
        }
    }

    /// 管理者権限があるかチェック
    pub fn is_admin(&self) -> bool {
        matches!(self, FamilyRole::Admin)
    }
}

impl std::fmt::Display for FamilyRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 家族メンバー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyMember {
    pub user_id: UserId,
    pub email: String,
    pub display_name: String,
    pub role: FamilyRole,
    pub joined_at: DateTime<Utc>,
    pub is_active: bool,
}

impl FamilyMember {
    /// 新しい家族メンバーを作成
    pub fn new(
        user_id: UserId,
        email: String,
        display_name: String,
        role: FamilyRole,
    ) -> Result<Self, DomainError> {
        // バリデーション
        if email.is_empty() || !email.contains('@') {
            return Err(DomainError::InvalidEmail(email));
        }
        if display_name.is_empty() {
            return Err(DomainError::InvalidDisplayName(
                "Display name cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            user_id,
            email,
            display_name,
            role,
            joined_at: Utc::now(),
            is_active: true,
        })
    }

    /// メンバーを非アクティブにする
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// メンバーをアクティブにする
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// 役割を変更する
    pub fn change_role(&mut self, new_role: FamilyRole) {
        self.role = new_role;
    }
}

/// 招待トークン
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvitationToken(String);

impl InvitationToken {
    /// 新しい招待トークンを生成
    pub fn new() -> Self {
        Self(ulid::Ulid::new().to_string())
    }

    /// 文字列から招待トークンを作成
    pub fn from_string(token: String) -> Result<Self, DomainError> {
        if token.is_empty() {
            return Err(DomainError::InvalidInvitationToken(
                "Invitation token cannot be empty".to_string(),
            ));
        }
        Ok(Self(token))
    }

    /// 招待トークンを文字列として取得
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 招待トークンが有効かチェック
    pub fn is_valid(&self) -> bool {
        !self.0.is_empty()
    }
}

impl Default for InvitationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for InvitationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 家族への招待
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyInvitation {
    pub invitation_token: InvitationToken,
    pub family_id: FamilyId,
    pub email: String,
    pub role: FamilyRole,
    pub invited_by: UserId,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub is_used: bool,
}

impl FamilyInvitation {
    /// 新しい家族招待を作成
    pub fn new(
        family_id: FamilyId,
        email: String,
        role: FamilyRole,
        invited_by: UserId,
        expires_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        // バリデーション
        if email.is_empty() || !email.contains('@') {
            return Err(DomainError::InvalidEmail(email));
        }
        if expires_at <= Utc::now() {
            return Err(DomainError::InvalidExpirationDate(
                "Expiration date must be in the future".to_string(),
            ));
        }

        Ok(Self {
            invitation_token: InvitationToken::new(),
            family_id,
            email,
            role,
            invited_by,
            expires_at,
            created_at: Utc::now(),
            is_used: false,
        })
    }

    /// 招待が有効かチェック
    pub fn is_valid(&self) -> bool {
        !self.is_used && self.expires_at > Utc::now()
    }

    /// 招待が期限切れかチェック
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }

    /// 招待を使用済みにマーク
    pub fn mark_as_used(&mut self) {
        self.is_used = true;
    }
}

/// 家族
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Family {
    pub family_id: FamilyId,
    pub name: String,
    pub members: HashMap<UserId, FamilyMember>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Family {
    /// 新しい家族を作成
    pub fn new(name: String, creator: FamilyMember) -> Result<Self, DomainError> {
        if name.is_empty() {
            return Err(DomainError::InvalidFamilyName(
                "Family name cannot be empty".to_string(),
            ));
        }

        let family_id = FamilyId::new();
        let mut members = HashMap::new();
        members.insert(creator.user_id.clone(), creator);

        Ok(Self {
            family_id,
            name,
            members,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// メンバーを追加
    pub fn add_member(&mut self, member: FamilyMember) -> Result<(), DomainError> {
        if self.members.contains_key(&member.user_id) {
            return Err(DomainError::MemberAlreadyExists(member.user_id.to_string()));
        }

        self.members.insert(member.user_id.clone(), member);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// メンバーを削除
    pub fn remove_member(&mut self, user_id: &UserId) -> Result<(), DomainError> {
        if !self.members.contains_key(user_id) {
            return Err(DomainError::MemberNotFound(user_id.to_string()));
        }

        // 最後の管理者を削除しようとしていないかチェック
        let admin_count = self
            .members
            .values()
            .filter(|m| m.role.is_admin() && m.is_active)
            .count();

        if let Some(member) = self.members.get(user_id) {
            if member.role.is_admin() && admin_count <= 1 {
                return Err(DomainError::CannotRemoveLastAdmin);
            }
        }

        self.members.remove(user_id);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// メンバーの役割を変更
    pub fn change_member_role(
        &mut self,
        user_id: &UserId,
        new_role: FamilyRole,
    ) -> Result<(), DomainError> {
        // 最初に管理者数をチェック
        if let Some(member) = self.members.get(user_id) {
            if member.role.is_admin() && !new_role.is_admin() {
                let admin_count = self
                    .members
                    .values()
                    .filter(|m| m.role.is_admin() && m.is_active)
                    .count();
                if admin_count <= 1 {
                    return Err(DomainError::CannotRemoveLastAdmin);
                }
            }
        } else {
            return Err(DomainError::MemberNotFound(user_id.to_string()));
        }

        // 役割を変更
        let member = self
            .members
            .get_mut(user_id)
            .ok_or_else(|| DomainError::MemberNotFound(user_id.to_string()))?;

        member.change_role(new_role);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// メンバーが管理者権限を持っているかチェック
    pub fn is_admin(&self, user_id: &UserId) -> bool {
        self.members
            .get(user_id)
            .map(|m| m.role.is_admin() && m.is_active)
            .unwrap_or(false)
    }

    /// メンバーが家族に属しているかチェック
    pub fn is_member(&self, user_id: &UserId) -> bool {
        self.members
            .get(user_id)
            .map(|m| m.is_active)
            .unwrap_or(false)
    }

    /// アクティブなメンバー数を取得
    pub fn active_member_count(&self) -> usize {
        self.members.values().filter(|m| m.is_active).count()
    }

    /// 管理者数を取得
    pub fn admin_count(&self) -> usize {
        self.members
            .values()
            .filter(|m| m.role.is_admin() && m.is_active)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_family_id_creation() {
        let family_id = FamilyId::new();
        assert!(family_id.is_valid());
        assert!(!family_id.as_str().is_empty());
    }

    #[test]
    fn test_family_id_from_string() {
        let id_str = "test-family-id".to_string();
        let family_id = FamilyId::from_string(id_str.clone()).unwrap();
        assert_eq!(family_id.as_str(), "test-family-id");

        // 空文字列はエラー
        assert!(FamilyId::from_string("".to_string()).is_err());
    }

    #[test]
    fn test_user_id_creation() {
        let user_id = UserId::new();
        assert!(user_id.is_valid());
        assert!(!user_id.as_str().is_empty());
    }

    #[test]
    fn test_family_role_from_string() {
        assert_eq!(FamilyRole::from_string("admin").unwrap(), FamilyRole::Admin);
        assert_eq!(
            FamilyRole::from_string("member").unwrap(),
            FamilyRole::Member
        );
        assert_eq!(FamilyRole::from_string("ADMIN").unwrap(), FamilyRole::Admin);
        assert!(FamilyRole::from_string("invalid").is_err());
    }

    #[test]
    fn test_family_role_is_admin() {
        assert!(FamilyRole::Admin.is_admin());
        assert!(!FamilyRole::Member.is_admin());
    }

    #[test]
    fn test_family_member_creation() {
        let user_id = UserId::new();
        let member = FamilyMember::new(
            user_id.clone(),
            "test@example.com".to_string(),
            "Test User".to_string(),
            FamilyRole::Member,
        )
        .unwrap();

        assert_eq!(member.user_id, user_id);
        assert_eq!(member.email, "test@example.com");
        assert_eq!(member.display_name, "Test User");
        assert_eq!(member.role, FamilyRole::Member);
        assert!(member.is_active);

        // 無効なメールアドレス
        assert!(FamilyMember::new(
            UserId::new(),
            "invalid-email".to_string(),
            "Test User".to_string(),
            FamilyRole::Member,
        )
        .is_err());

        // 空の表示名
        assert!(FamilyMember::new(
            UserId::new(),
            "test@example.com".to_string(),
            "".to_string(),
            FamilyRole::Member,
        )
        .is_err());
    }

    #[test]
    fn test_invitation_token_creation() {
        let token = InvitationToken::new();
        assert!(token.is_valid());
        assert!(!token.as_str().is_empty());
    }

    #[test]
    fn test_family_invitation_creation() {
        let family_id = FamilyId::new();
        let invited_by = UserId::new();
        let expires_at = Utc::now() + Duration::days(7);

        let invitation = FamilyInvitation::new(
            family_id.clone(),
            "test@example.com".to_string(),
            FamilyRole::Member,
            invited_by.clone(),
            expires_at,
        )
        .unwrap();

        assert_eq!(invitation.family_id, family_id);
        assert_eq!(invitation.email, "test@example.com");
        assert_eq!(invitation.role, FamilyRole::Member);
        assert_eq!(invitation.invited_by, invited_by);
        assert!(invitation.is_valid());
        assert!(!invitation.is_expired());
        assert!(!invitation.is_used);

        // 過去の有効期限
        assert!(FamilyInvitation::new(
            family_id,
            "test@example.com".to_string(),
            FamilyRole::Member,
            invited_by,
            Utc::now() - Duration::hours(1),
        )
        .is_err());
    }

    #[test]
    fn test_family_creation() {
        let creator = FamilyMember::new(
            UserId::new(),
            "creator@example.com".to_string(),
            "Creator".to_string(),
            FamilyRole::Admin,
        )
        .unwrap();

        let family = Family::new("Test Family".to_string(), creator.clone()).unwrap();

        assert_eq!(family.name, "Test Family");
        assert_eq!(family.members.len(), 1);
        assert!(family.is_admin(&creator.user_id));
        assert!(family.is_member(&creator.user_id));
        assert_eq!(family.active_member_count(), 1);
        assert_eq!(family.admin_count(), 1);

        // 空の家族名
        assert!(Family::new("".to_string(), creator).is_err());
    }

    #[test]
    fn test_family_add_member() {
        let creator = FamilyMember::new(
            UserId::new(),
            "creator@example.com".to_string(),
            "Creator".to_string(),
            FamilyRole::Admin,
        )
        .unwrap();

        let mut family = Family::new("Test Family".to_string(), creator).unwrap();

        let new_member = FamilyMember::new(
            UserId::new(),
            "member@example.com".to_string(),
            "Member".to_string(),
            FamilyRole::Member,
        )
        .unwrap();

        assert!(family.add_member(new_member.clone()).is_ok());
        assert_eq!(family.members.len(), 2);
        assert!(family.is_member(&new_member.user_id));

        // 同じメンバーを再度追加
        assert!(family.add_member(new_member).is_err());
    }

    #[test]
    fn test_family_remove_member() {
        let creator = FamilyMember::new(
            UserId::new(),
            "creator@example.com".to_string(),
            "Creator".to_string(),
            FamilyRole::Admin,
        )
        .unwrap();

        let mut family = Family::new("Test Family".to_string(), creator.clone()).unwrap();

        let member = FamilyMember::new(
            UserId::new(),
            "member@example.com".to_string(),
            "Member".to_string(),
            FamilyRole::Member,
        )
        .unwrap();

        family.add_member(member.clone()).unwrap();

        // 一般メンバーを削除
        assert!(family.remove_member(&member.user_id).is_ok());
        assert_eq!(family.members.len(), 1);

        // 最後の管理者を削除しようとする
        assert!(family.remove_member(&creator.user_id).is_err());

        // 存在しないメンバーを削除
        let non_existent = UserId::new();
        assert!(family.remove_member(&non_existent).is_err());
    }

    #[test]
    fn test_family_change_member_role() {
        let creator = FamilyMember::new(
            UserId::new(),
            "creator@example.com".to_string(),
            "Creator".to_string(),
            FamilyRole::Admin,
        )
        .unwrap();

        let mut family = Family::new("Test Family".to_string(), creator.clone()).unwrap();

        let member = FamilyMember::new(
            UserId::new(),
            "member@example.com".to_string(),
            "Member".to_string(),
            FamilyRole::Member,
        )
        .unwrap();

        family.add_member(member.clone()).unwrap();

        // 一般メンバーを管理者に昇格
        assert!(family
            .change_member_role(&member.user_id, FamilyRole::Admin)
            .is_ok());
        assert!(family.is_admin(&member.user_id));
        assert_eq!(family.admin_count(), 2);

        // 最後の管理者を一般メンバーに降格しようとする（複数管理者がいるので成功）
        assert!(family
            .change_member_role(&creator.user_id, FamilyRole::Member)
            .is_ok());
        assert!(!family.is_admin(&creator.user_id));
        assert_eq!(family.admin_count(), 1);

        // 最後の管理者を一般メンバーに降格しようとする（失敗）
        assert!(family
            .change_member_role(&member.user_id, FamilyRole::Member)
            .is_err());
    }
}

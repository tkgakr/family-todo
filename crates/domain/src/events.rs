use crate::errors::DomainError;
use crate::todo::TodoId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

/// イベントのバージョン情報を表す構造体
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventVersion {
    pub major: u32,
    pub minor: u32,
}

impl EventVersion {
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// バージョン文字列から EventVersion を作成
    pub fn from_string(version: &str) -> Result<Self, DomainError> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 2 {
            return Err(DomainError::InvalidEventVersion(version.to_string()));
        }

        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| DomainError::InvalidEventVersion(version.to_string()))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| DomainError::InvalidEventVersion(version.to_string()))?;

        Ok(Self { major, minor })
    }

    /// バージョンを文字列として取得
    pub fn as_string(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }

    /// このバージョンが他のバージョンと互換性があるかチェック
    pub fn is_compatible_with(&self, other: &EventVersion) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}

impl fmt::Display for EventVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// 生のイベントデータを表す構造体（デシリアライゼーション用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEventData {
    pub event_type: String,
    pub version: String,
    pub data: Value,
}

/// メインのTodoEventイベント列挙型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum TodoEvent {
    TodoCreatedV2 {
        event_id: String,
        todo_id: TodoId,
        title: String,
        description: Option<String>,
        tags: Vec<String>,
        created_by: String, // UserId
        timestamp: DateTime<Utc>,
        #[serde(default = "default_created_v2_version")]
        version: String,
    },
    TodoUpdatedV1 {
        event_id: String,
        todo_id: TodoId,
        title: Option<String>,
        description: Option<String>,
        updated_by: String, // UserId
        timestamp: DateTime<Utc>,
        #[serde(default = "default_updated_v1_version")]
        version: String,
    },
    TodoCompletedV1 {
        event_id: String,
        todo_id: TodoId,
        completed_by: String, // UserId
        timestamp: DateTime<Utc>,
        #[serde(default = "default_completed_v1_version")]
        version: String,
    },
    TodoDeletedV1 {
        event_id: String,
        todo_id: TodoId,
        deleted_by: String, // UserId
        reason: Option<String>,
        timestamp: DateTime<Utc>,
        #[serde(default = "default_deleted_v1_version")]
        version: String,
    },
}

/// 家族メンバー管理イベント列挙型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum FamilyEvent {
    FamilyMemberInvitedV1 {
        event_id: String,
        family_id: String,
        invitation_token: String,
        email: String,
        role: String,       // "admin" | "member"
        invited_by: String, // UserId
        expires_at: DateTime<Utc>,
        timestamp: DateTime<Utc>,
        #[serde(default = "default_member_invited_v1_version")]
        version: String,
    },
    FamilyMemberJoinedV1 {
        event_id: String,
        family_id: String,
        user_id: String,
        email: String,
        role: String,
        display_name: String,
        invitation_token: String,
        timestamp: DateTime<Utc>,
        #[serde(default = "default_member_joined_v1_version")]
        version: String,
    },
    FamilyMemberRemovedV1 {
        event_id: String,
        family_id: String,
        user_id: String,
        removed_by: String, // UserId
        reason: Option<String>,
        timestamp: DateTime<Utc>,
        #[serde(default = "default_member_removed_v1_version")]
        version: String,
    },
    InvitationExpiredV1 {
        event_id: String,
        family_id: String,
        invitation_token: String,
        email: String,
        timestamp: DateTime<Utc>,
        #[serde(default = "default_invitation_expired_v1_version")]
        version: String,
    },
}

// デフォルトバージョン関数
fn default_created_v2_version() -> String {
    "2.0".to_string()
}
fn default_updated_v1_version() -> String {
    "1.0".to_string()
}
fn default_completed_v1_version() -> String {
    "1.0".to_string()
}
fn default_deleted_v1_version() -> String {
    "1.0".to_string()
}
fn default_member_invited_v1_version() -> String {
    "1.0".to_string()
}
fn default_member_joined_v1_version() -> String {
    "1.0".to_string()
}
fn default_member_removed_v1_version() -> String {
    "1.0".to_string()
}
fn default_invitation_expired_v1_version() -> String {
    "1.0".to_string()
}

/// 古いイベントバージョンを新しいバージョンにアップキャストするためのトレイト
pub trait EventUpcast {
    /// 生のイベントデータを最新のTodoEventにアップキャストする
    fn upcast_from_raw(raw: RawEventData) -> Result<TodoEvent, DomainError>;
}

/// イベントのアップキャスト実装
impl EventUpcast for TodoEvent {
    fn upcast_from_raw(raw: RawEventData) -> Result<TodoEvent, DomainError> {
        let _version = EventVersion::from_string(&raw.version)?;

        match raw.event_type.as_str() {
            "todo_created_v1" => {
                // TodoCreatedV1 から TodoCreatedV2 へのアップキャスト
                upcast_todo_created_v1_to_v2(raw.data)
            }
            "todo_created_v2" => {
                // 既に最新バージョン - データにバージョンとevent_typeフィールドを追加
                let mut data_map: HashMap<String, Value> = serde_json::from_value(raw.data)
                    .map_err(|e| DomainError::EventDeserialization(e.to_string()))?;
                data_map.insert("version".to_string(), Value::String("2.0".to_string()));
                data_map.insert(
                    "event_type".to_string(),
                    Value::String("todo_created_v2".to_string()),
                );

                let updated_data = serde_json::to_value(data_map)
                    .map_err(|e| DomainError::EventSerialization(e.to_string()))?;

                serde_json::from_value(updated_data)
                    .map_err(|e| DomainError::EventDeserialization(e.to_string()))
            }
            "todo_updated_v1" => {
                // TodoUpdatedV1 は現在の最新バージョン - データにバージョンとevent_typeフィールドを追加
                let mut data_map: HashMap<String, Value> = serde_json::from_value(raw.data)
                    .map_err(|e| DomainError::EventDeserialization(e.to_string()))?;
                data_map.insert("version".to_string(), Value::String("1.0".to_string()));
                data_map.insert(
                    "event_type".to_string(),
                    Value::String("todo_updated_v1".to_string()),
                );

                let updated_data = serde_json::to_value(data_map)
                    .map_err(|e| DomainError::EventSerialization(e.to_string()))?;

                serde_json::from_value(updated_data)
                    .map_err(|e| DomainError::EventDeserialization(e.to_string()))
            }
            "todo_completed_v1" => {
                // TodoCompletedV1 は現在の最新バージョン - データにバージョンとevent_typeフィールドを追加
                let mut data_map: HashMap<String, Value> = serde_json::from_value(raw.data)
                    .map_err(|e| DomainError::EventDeserialization(e.to_string()))?;
                data_map.insert("version".to_string(), Value::String("1.0".to_string()));
                data_map.insert(
                    "event_type".to_string(),
                    Value::String("todo_completed_v1".to_string()),
                );

                let updated_data = serde_json::to_value(data_map)
                    .map_err(|e| DomainError::EventSerialization(e.to_string()))?;

                serde_json::from_value(updated_data)
                    .map_err(|e| DomainError::EventDeserialization(e.to_string()))
            }
            "todo_deleted_v1" => {
                // TodoDeletedV1 は現在の最新バージョン - データにバージョンとevent_typeフィールドを追加
                let mut data_map: HashMap<String, Value> = serde_json::from_value(raw.data)
                    .map_err(|e| DomainError::EventDeserialization(e.to_string()))?;
                data_map.insert("version".to_string(), Value::String("1.0".to_string()));
                data_map.insert(
                    "event_type".to_string(),
                    Value::String("todo_deleted_v1".to_string()),
                );

                let updated_data = serde_json::to_value(data_map)
                    .map_err(|e| DomainError::EventSerialization(e.to_string()))?;

                serde_json::from_value(updated_data)
                    .map_err(|e| DomainError::EventDeserialization(e.to_string()))
            }
            _ => Err(DomainError::UnknownEventType(raw.event_type)),
        }
    }
}

/// TodoCreatedV1 から TodoCreatedV2 へのアップキャスト関数
fn upcast_todo_created_v1_to_v2(data: Value) -> Result<TodoEvent, DomainError> {
    // V1 では tags フィールドがなかったと仮定
    let mut v1_data: HashMap<String, Value> = serde_json::from_value(data)
        .map_err(|e| DomainError::EventDeserialization(e.to_string()))?;

    // V2 では tags フィールドを追加（デフォルトは空の配列）
    if !v1_data.contains_key("tags") {
        v1_data.insert("tags".to_string(), Value::Array(vec![]));
    }

    // バージョンを更新
    v1_data.insert("version".to_string(), Value::String("2.0".to_string()));

    // event_typeを追加（serdeのtagged enumのため）
    v1_data.insert(
        "event_type".to_string(),
        Value::String("todo_created_v2".to_string()),
    );

    let v2_data = serde_json::to_value(v1_data)
        .map_err(|e| DomainError::EventSerialization(e.to_string()))?;

    serde_json::from_value(v2_data).map_err(|e| DomainError::EventDeserialization(e.to_string()))
}

impl TodoEvent {
    /// イベントに関連するTodoIdを取得
    pub fn todo_id(&self) -> &TodoId {
        match self {
            TodoEvent::TodoCreatedV2 { todo_id, .. } => todo_id,
            TodoEvent::TodoUpdatedV1 { todo_id, .. } => todo_id,
            TodoEvent::TodoCompletedV1 { todo_id, .. } => todo_id,
            TodoEvent::TodoDeletedV1 { todo_id, .. } => todo_id,
        }
    }

    /// イベントIDを取得
    pub fn event_id(&self) -> &str {
        match self {
            TodoEvent::TodoCreatedV2 { event_id, .. } => event_id,
            TodoEvent::TodoUpdatedV1 { event_id, .. } => event_id,
            TodoEvent::TodoCompletedV1 { event_id, .. } => event_id,
            TodoEvent::TodoDeletedV1 { event_id, .. } => event_id,
        }
    }

    /// イベントのタイムスタンプを取得
    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            TodoEvent::TodoCreatedV2 { timestamp, .. } => timestamp,
            TodoEvent::TodoUpdatedV1 { timestamp, .. } => timestamp,
            TodoEvent::TodoCompletedV1 { timestamp, .. } => timestamp,
            TodoEvent::TodoDeletedV1 { timestamp, .. } => timestamp,
        }
    }

    /// イベントのバージョンを取得
    pub fn version(&self) -> &str {
        match self {
            TodoEvent::TodoCreatedV2 { version, .. } => version,
            TodoEvent::TodoUpdatedV1 { version, .. } => version,
            TodoEvent::TodoCompletedV1 { version, .. } => version,
            TodoEvent::TodoDeletedV1 { version, .. } => version,
        }
    }

    /// イベントタイプ名を取得
    pub fn event_type(&self) -> &'static str {
        match self {
            TodoEvent::TodoCreatedV2 { .. } => "todo_created_v2",
            TodoEvent::TodoUpdatedV1 { .. } => "todo_updated_v1",
            TodoEvent::TodoCompletedV1 { .. } => "todo_completed_v1",
            TodoEvent::TodoDeletedV1 { .. } => "todo_deleted_v1",
        }
    }

    /// 新しいTodoCreatedV2イベントを作成
    pub fn new_todo_created(
        todo_id: TodoId,
        title: String,
        description: Option<String>,
        tags: Vec<String>,
        created_by: String,
    ) -> Self {
        TodoEvent::TodoCreatedV2 {
            event_id: ulid::Ulid::new().to_string(),
            todo_id,
            title,
            description,
            tags,
            created_by,
            timestamp: Utc::now(),
            version: "2.0".to_string(),
        }
    }

    /// 新しいTodoUpdatedV1イベントを作成
    pub fn new_todo_updated(
        todo_id: TodoId,
        title: Option<String>,
        description: Option<String>,
        updated_by: String,
    ) -> Self {
        TodoEvent::TodoUpdatedV1 {
            event_id: ulid::Ulid::new().to_string(),
            todo_id,
            title,
            description,
            updated_by,
            timestamp: Utc::now(),
            version: "1.0".to_string(),
        }
    }

    /// 新しいTodoCompletedV1イベントを作成
    pub fn new_todo_completed(todo_id: TodoId, completed_by: String) -> Self {
        TodoEvent::TodoCompletedV1 {
            event_id: ulid::Ulid::new().to_string(),
            todo_id,
            completed_by,
            timestamp: Utc::now(),
            version: "1.0".to_string(),
        }
    }

    /// 新しいTodoDeletedV1イベントを作成
    pub fn new_todo_deleted(todo_id: TodoId, deleted_by: String, reason: Option<String>) -> Self {
        TodoEvent::TodoDeletedV1 {
            event_id: ulid::Ulid::new().to_string(),
            todo_id,
            deleted_by,
            reason,
            timestamp: Utc::now(),
            version: "1.0".to_string(),
        }
    }

    /// 生のJSONからイベントをデシリアライズ（アップキャスト対応）
    pub fn from_json_with_upcast(json: &str) -> Result<Self, DomainError> {
        // まず生のデータとして読み込み
        let raw: RawEventData = serde_json::from_str(json)
            .map_err(|e| DomainError::EventDeserialization(e.to_string()))?;

        // アップキャストを実行
        Self::upcast_from_raw(raw)
    }

    /// イベントをJSONにシリアライズ
    pub fn to_json(&self) -> Result<String, DomainError> {
        serde_json::to_string(self).map_err(|e| DomainError::EventSerialization(e.to_string()))
    }

    /// イベントが有効かどうかをバリデーション
    pub fn validate(&self) -> Result<(), DomainError> {
        // 共通バリデーション
        if self.event_id().is_empty() {
            return Err(DomainError::InvalidEvent(
                "Event ID cannot be empty".to_string(),
            ));
        }

        if !self.todo_id().is_valid() {
            return Err(DomainError::InvalidEvent("Invalid TodoId".to_string()));
        }

        // イベント固有のバリデーション
        match self {
            TodoEvent::TodoCreatedV2 {
                title, created_by, ..
            } => {
                if title.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Title cannot be empty".to_string(),
                    ));
                }
                if title.len() > 200 {
                    return Err(DomainError::InvalidEvent(
                        "Title cannot exceed 200 characters".to_string(),
                    ));
                }
                if created_by.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Created by cannot be empty".to_string(),
                    ));
                }
            }
            TodoEvent::TodoUpdatedV1 {
                updated_by, title, ..
            } => {
                if updated_by.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Updated by cannot be empty".to_string(),
                    ));
                }
                if let Some(title) = title {
                    if title.is_empty() {
                        return Err(DomainError::InvalidEvent(
                            "Title cannot be empty".to_string(),
                        ));
                    }
                    if title.len() > 200 {
                        return Err(DomainError::InvalidEvent(
                            "Title cannot exceed 200 characters".to_string(),
                        ));
                    }
                }
            }
            TodoEvent::TodoCompletedV1 { completed_by, .. } => {
                if completed_by.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Completed by cannot be empty".to_string(),
                    ));
                }
            }
            TodoEvent::TodoDeletedV1 { deleted_by, .. } => {
                if deleted_by.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Deleted by cannot be empty".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

impl FamilyEvent {
    /// イベントIDを取得
    pub fn event_id(&self) -> &str {
        match self {
            FamilyEvent::FamilyMemberInvitedV1 { event_id, .. } => event_id,
            FamilyEvent::FamilyMemberJoinedV1 { event_id, .. } => event_id,
            FamilyEvent::FamilyMemberRemovedV1 { event_id, .. } => event_id,
            FamilyEvent::InvitationExpiredV1 { event_id, .. } => event_id,
        }
    }

    /// 家族IDを取得
    pub fn family_id(&self) -> &str {
        match self {
            FamilyEvent::FamilyMemberInvitedV1 { family_id, .. } => family_id,
            FamilyEvent::FamilyMemberJoinedV1 { family_id, .. } => family_id,
            FamilyEvent::FamilyMemberRemovedV1 { family_id, .. } => family_id,
            FamilyEvent::InvitationExpiredV1 { family_id, .. } => family_id,
        }
    }

    /// イベントのタイムスタンプを取得
    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            FamilyEvent::FamilyMemberInvitedV1 { timestamp, .. } => timestamp,
            FamilyEvent::FamilyMemberJoinedV1 { timestamp, .. } => timestamp,
            FamilyEvent::FamilyMemberRemovedV1 { timestamp, .. } => timestamp,
            FamilyEvent::InvitationExpiredV1 { timestamp, .. } => timestamp,
        }
    }

    /// イベントのバージョンを取得
    pub fn version(&self) -> &str {
        match self {
            FamilyEvent::FamilyMemberInvitedV1 { version, .. } => version,
            FamilyEvent::FamilyMemberJoinedV1 { version, .. } => version,
            FamilyEvent::FamilyMemberRemovedV1 { version, .. } => version,
            FamilyEvent::InvitationExpiredV1 { version, .. } => version,
        }
    }

    /// イベントタイプ名を取得
    pub fn event_type(&self) -> &'static str {
        match self {
            FamilyEvent::FamilyMemberInvitedV1 { .. } => "family_member_invited_v1",
            FamilyEvent::FamilyMemberJoinedV1 { .. } => "family_member_joined_v1",
            FamilyEvent::FamilyMemberRemovedV1 { .. } => "family_member_removed_v1",
            FamilyEvent::InvitationExpiredV1 { .. } => "invitation_expired_v1",
        }
    }

    /// 新しい家族メンバー招待イベントを作成
    pub fn new_member_invited(
        family_id: String,
        invitation_token: String,
        email: String,
        role: String,
        invited_by: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        FamilyEvent::FamilyMemberInvitedV1 {
            event_id: ulid::Ulid::new().to_string(),
            family_id,
            invitation_token,
            email,
            role,
            invited_by,
            expires_at,
            timestamp: Utc::now(),
            version: "1.0".to_string(),
        }
    }

    /// 新しい家族メンバー参加イベントを作成
    pub fn new_member_joined(
        family_id: String,
        user_id: String,
        email: String,
        role: String,
        display_name: String,
        invitation_token: String,
    ) -> Self {
        FamilyEvent::FamilyMemberJoinedV1 {
            event_id: ulid::Ulid::new().to_string(),
            family_id,
            user_id,
            email,
            role,
            display_name,
            invitation_token,
            timestamp: Utc::now(),
            version: "1.0".to_string(),
        }
    }

    /// 新しい家族メンバー削除イベントを作成
    pub fn new_member_removed(
        family_id: String,
        user_id: String,
        removed_by: String,
        reason: Option<String>,
    ) -> Self {
        FamilyEvent::FamilyMemberRemovedV1 {
            event_id: ulid::Ulid::new().to_string(),
            family_id,
            user_id,
            removed_by,
            reason,
            timestamp: Utc::now(),
            version: "1.0".to_string(),
        }
    }

    /// 新しい招待期限切れイベントを作成
    pub fn new_invitation_expired(
        family_id: String,
        invitation_token: String,
        email: String,
    ) -> Self {
        FamilyEvent::InvitationExpiredV1 {
            event_id: ulid::Ulid::new().to_string(),
            family_id,
            invitation_token,
            email,
            timestamp: Utc::now(),
            version: "1.0".to_string(),
        }
    }

    /// イベントをJSONにシリアライズ
    pub fn to_json(&self) -> Result<String, DomainError> {
        serde_json::to_string(self).map_err(|e| DomainError::EventSerialization(e.to_string()))
    }

    /// イベントが有効かどうかをバリデーション
    pub fn validate(&self) -> Result<(), DomainError> {
        // 共通バリデーション
        if self.event_id().is_empty() {
            return Err(DomainError::InvalidEvent(
                "Event ID cannot be empty".to_string(),
            ));
        }

        if self.family_id().is_empty() {
            return Err(DomainError::InvalidEvent(
                "Family ID cannot be empty".to_string(),
            ));
        }

        // イベント固有のバリデーション
        match self {
            FamilyEvent::FamilyMemberInvitedV1 {
                email,
                role,
                invited_by,
                invitation_token,
                expires_at,
                ..
            } => {
                if email.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Email cannot be empty".to_string(),
                    ));
                }
                if !email.contains('@') {
                    return Err(DomainError::InvalidEvent(
                        "Invalid email format".to_string(),
                    ));
                }
                if role != "admin" && role != "member" {
                    return Err(DomainError::InvalidEvent(
                        "Role must be 'admin' or 'member'".to_string(),
                    ));
                }
                if invited_by.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Invited by cannot be empty".to_string(),
                    ));
                }
                if invitation_token.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Invitation token cannot be empty".to_string(),
                    ));
                }
                if expires_at <= &Utc::now() {
                    return Err(DomainError::InvalidEvent(
                        "Expiration date must be in the future".to_string(),
                    ));
                }
            }
            FamilyEvent::FamilyMemberJoinedV1 {
                user_id,
                email,
                role,
                display_name,
                invitation_token,
                ..
            } => {
                if user_id.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "User ID cannot be empty".to_string(),
                    ));
                }
                if email.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Email cannot be empty".to_string(),
                    ));
                }
                if !email.contains('@') {
                    return Err(DomainError::InvalidEvent(
                        "Invalid email format".to_string(),
                    ));
                }
                if role != "admin" && role != "member" {
                    return Err(DomainError::InvalidEvent(
                        "Role must be 'admin' or 'member'".to_string(),
                    ));
                }
                if display_name.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Display name cannot be empty".to_string(),
                    ));
                }
                if invitation_token.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Invitation token cannot be empty".to_string(),
                    ));
                }
            }
            FamilyEvent::FamilyMemberRemovedV1 {
                user_id,
                removed_by,
                ..
            } => {
                if user_id.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "User ID cannot be empty".to_string(),
                    ));
                }
                if removed_by.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Removed by cannot be empty".to_string(),
                    ));
                }
            }
            FamilyEvent::InvitationExpiredV1 {
                invitation_token,
                email,
                ..
            } => {
                if invitation_token.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Invitation token cannot be empty".to_string(),
                    ));
                }
                if email.is_empty() {
                    return Err(DomainError::InvalidEvent(
                        "Email cannot be empty".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::todo::TodoId;

    #[test]
    fn test_event_version_creation() {
        let version = EventVersion::new(2, 1);
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 1);
        assert_eq!(version.as_string(), "2.1");
    }

    #[test]
    fn test_event_version_from_string() {
        let version = EventVersion::from_string("1.5").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 5);

        // 無効なバージョン文字列
        assert!(EventVersion::from_string("invalid").is_err());
        assert!(EventVersion::from_string("1").is_err());
        assert!(EventVersion::from_string("1.2.3").is_err());
    }

    #[test]
    fn test_event_version_compatibility() {
        let v1_0 = EventVersion::new(1, 0);
        let v1_1 = EventVersion::new(1, 1);
        let v2_0 = EventVersion::new(2, 0);

        assert!(v1_1.is_compatible_with(&v1_0));
        assert!(!v1_0.is_compatible_with(&v1_1));
        assert!(!v2_0.is_compatible_with(&v1_0));
        assert!(!v1_0.is_compatible_with(&v2_0));
    }

    #[test]
    fn test_todo_created_v2_creation() {
        let todo_id = TodoId::new();
        let event = TodoEvent::new_todo_created(
            todo_id.clone(),
            "Test Todo".to_string(),
            Some("Test Description".to_string()),
            vec!["tag1".to_string(), "tag2".to_string()],
            "user123".to_string(),
        );

        match event {
            TodoEvent::TodoCreatedV2 {
                todo_id: id,
                title,
                description,
                tags,
                created_by,
                version,
                ..
            } => {
                assert_eq!(id, todo_id);
                assert_eq!(title, "Test Todo");
                assert_eq!(description, Some("Test Description".to_string()));
                assert_eq!(tags, vec!["tag1", "tag2"]);
                assert_eq!(created_by, "user123");
                assert_eq!(version, "2.0");
            }
            _ => panic!("Expected TodoCreatedV2 event"),
        }
    }

    #[test]
    fn test_todo_event_validation() {
        let todo_id = TodoId::new();
        let event = TodoEvent::new_todo_created(
            todo_id,
            "Valid Title".to_string(),
            Some("Valid Description".to_string()),
            vec!["tag1".to_string()],
            "user123".to_string(),
        );

        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_family_event_creation() {
        let event = FamilyEvent::new_member_invited(
            "family123".to_string(),
            "token123".to_string(),
            "test@example.com".to_string(),
            "member".to_string(),
            "admin123".to_string(),
            Utc::now() + chrono::Duration::days(7),
        );

        assert_eq!(event.family_id(), "family123");
        assert_eq!(event.event_type(), "family_member_invited_v1");
    }
}

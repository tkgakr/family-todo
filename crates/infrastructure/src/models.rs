use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Utc};
use domain::{TodoEvent, TodoId, Todo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// DynamoDB アイテムのエンティティタイプ
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Event,
    Projection,
    Snapshot,
    Family,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Event => "Event",
            EntityType::Projection => "Projection",
            EntityType::Snapshot => "Snapshot",
            EntityType::Family => "Family",
        }
    }
}

impl From<&str> for EntityType {
    fn from(s: &str) -> Self {
        match s {
            "Event" => EntityType::Event,
            "Projection" => EntityType::Projection,
            "Snapshot" => EntityType::Snapshot,
            "Family" => EntityType::Family,
            _ => EntityType::Event, // デフォルト
        }
    }
}

/// DynamoDB Single Table Design のキー構造
#[derive(Debug, Clone)]
pub struct DynamoDbKeys {
    pub pk: String,      // パーティションキー
    pub sk: String,      // ソートキー
    pub gsi1_pk: Option<String>, // GSI1 パーティションキー
    pub gsi1_sk: Option<String>, // GSI1 ソートキー
}

impl DynamoDbKeys {
    /// イベント用のキーを生成
    pub fn for_event(family_id: &str, event_ulid: &str) -> Self {
        Self {
            pk: format!("FAMILY#{}", family_id),
            sk: format!("EVENT#{}", event_ulid),
            gsi1_pk: None,
            gsi1_sk: None,
        }
    }

    /// 現在のToDo状態用のキーを生成
    pub fn for_todo_projection(family_id: &str, todo_id: &TodoId) -> Self {
        Self {
            pk: format!("FAMILY#{}", family_id),
            sk: format!("TODO#CURRENT#{}", todo_id.as_str()),
            gsi1_pk: None,
            gsi1_sk: None,
        }
    }

    /// アクティブなToDo用のGSIキーを生成
    pub fn for_active_todo(family_id: &str, todo_ulid: &str) -> Self {
        Self {
            pk: format!("FAMILY#{}", family_id),
            sk: format!("TODO#CURRENT#{}", todo_ulid),
            gsi1_pk: Some(format!("FAMILY#{}#ACTIVE", family_id)),
            gsi1_sk: Some(todo_ulid.to_string()),
        }
    }

    /// ToDo履歴用のキーを生成
    pub fn for_todo_history(family_id: &str, todo_id: &TodoId, event_ulid: &str) -> Self {
        Self {
            pk: format!("FAMILY#{}", family_id),
            sk: format!("TODO#EVENT#{}#{}", todo_id.as_str(), event_ulid),
            gsi1_pk: None,
            gsi1_sk: None,
        }
    }

    /// スナップショット用のキーを生成
    pub fn for_snapshot(family_id: &str, todo_id: &TodoId, snapshot_ulid: &str) -> Self {
        Self {
            pk: format!("FAMILY#{}", family_id),
            sk: format!("TODO#SNAPSHOT#{}#{}", todo_id.as_str(), snapshot_ulid),
            gsi1_pk: None,
            gsi1_sk: None,
        }
    }
}

/// DynamoDB アイテムの基本構造
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamoDbItem {
    pub pk: String,
    pub sk: String,
    pub entity_type: EntityType,
    pub gsi1_pk: Option<String>,
    pub gsi1_sk: Option<String>,
    pub data: serde_json::Value,
    pub version: u64,
    pub ttl: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DynamoDbItem {
    /// 新しいDynamoDBアイテムを作成
    pub fn new(
        keys: DynamoDbKeys,
        entity_type: EntityType,
        data: serde_json::Value,
        version: u64,
        ttl: Option<i64>,
    ) -> Self {
        let now = Utc::now();
        Self {
            pk: keys.pk,
            sk: keys.sk,
            entity_type,
            gsi1_pk: keys.gsi1_pk,
            gsi1_sk: keys.gsi1_sk,
            data,
            version,
            ttl,
            created_at: now,
            updated_at: now,
        }
    }

    /// DynamoDB AttributeValue マップに変換
    pub fn to_attribute_map(&self) -> HashMap<String, AttributeValue> {
        let mut map = HashMap::new();

        map.insert("PK".to_string(), AttributeValue::S(self.pk.clone()));
        map.insert("SK".to_string(), AttributeValue::S(self.sk.clone()));
        map.insert("EntityType".to_string(), AttributeValue::S(self.entity_type.as_str().to_string()));
        
        if let Some(gsi1_pk) = &self.gsi1_pk {
            map.insert("GSI1PK".to_string(), AttributeValue::S(gsi1_pk.clone()));
        }
        
        if let Some(gsi1_sk) = &self.gsi1_sk {
            map.insert("GSI1SK".to_string(), AttributeValue::S(gsi1_sk.clone()));
        }

        map.insert("Data".to_string(), AttributeValue::S(self.data.to_string()));
        map.insert("Version".to_string(), AttributeValue::N(self.version.to_string()));
        
        if let Some(ttl) = self.ttl {
            map.insert("TTL".to_string(), AttributeValue::N(ttl.to_string()));
        }

        map.insert("CreatedAt".to_string(), AttributeValue::S(self.created_at.to_rfc3339()));
        map.insert("UpdatedAt".to_string(), AttributeValue::S(self.updated_at.to_rfc3339()));

        map
    }

    /// DynamoDB AttributeValue マップから復元
    pub fn from_attribute_map(map: &HashMap<String, AttributeValue>) -> Result<Self, String> {
        let pk = map.get("PK")
            .and_then(|v| v.as_s().ok())
            .ok_or("Missing PK")?
            .clone();

        let sk = map.get("SK")
            .and_then(|v| v.as_s().ok())
            .ok_or("Missing SK")?
            .clone();

        let entity_type = map.get("EntityType")
            .and_then(|v| v.as_s().ok())
            .map(|s| EntityType::from(s.as_str()))
            .ok_or("Missing EntityType")?;

        let gsi1_pk = map.get("GSI1PK")
            .and_then(|v| v.as_s().ok())
            .cloned();

        let gsi1_sk = map.get("GSI1SK")
            .and_then(|v| v.as_s().ok())
            .cloned();

        let data_str = map.get("Data")
            .and_then(|v| v.as_s().ok())
            .ok_or("Missing Data")?;

        let data: serde_json::Value = serde_json::from_str(data_str)
            .map_err(|e| format!("Failed to parse Data JSON: {}", e))?;

        let version = map.get("Version")
            .and_then(|v| v.as_n().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or("Missing or invalid Version")?;

        let ttl = map.get("TTL")
            .and_then(|v| v.as_n().ok())
            .and_then(|s| s.parse::<i64>().ok());

        let created_at = map.get("CreatedAt")
            .and_then(|v| v.as_s().ok())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .ok_or("Missing or invalid CreatedAt")?;

        let updated_at = map.get("UpdatedAt")
            .and_then(|v| v.as_s().ok())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .ok_or("Missing or invalid UpdatedAt")?;

        Ok(Self {
            pk,
            sk,
            entity_type,
            gsi1_pk,
            gsi1_sk,
            data,
            version,
            ttl,
            created_at,
            updated_at,
        })
    }
}

/// イベント用のDynamoDBアイテム
#[derive(Debug, Clone)]
pub struct EventItem {
    pub family_id: String,
    pub event: TodoEvent,
    pub version: u64,
}

impl EventItem {
    pub fn new(family_id: String, event: TodoEvent, version: u64) -> Self {
        Self {
            family_id,
            event,
            version,
        }
    }

    /// DynamoDbItem に変換
    pub fn to_dynamodb_item(&self) -> Result<DynamoDbItem, String> {
        let event_ulid = self.event.event_id();
        let keys = DynamoDbKeys::for_event(&self.family_id, event_ulid);
        
        let data = serde_json::to_value(&self.event)
            .map_err(|e| format!("Failed to serialize event: {}", e))?;

        Ok(DynamoDbItem::new(
            keys,
            EntityType::Event,
            data,
            self.version,
            None, // イベントは永続化（TTLなし）
        ))
    }

    /// DynamoDbItem から復元
    pub fn from_dynamodb_item(item: &DynamoDbItem) -> Result<Self, String> {
        if item.entity_type != EntityType::Event {
            return Err("Item is not an Event".to_string());
        }

        let event: TodoEvent = serde_json::from_value(item.data.clone())
            .map_err(|e| format!("Failed to deserialize event: {}", e))?;

        // PKからfamily_idを抽出
        let family_id = item.pk.strip_prefix("FAMILY#")
            .ok_or("Invalid PK format for Event")?
            .to_string();

        Ok(Self {
            family_id,
            event,
            version: item.version,
        })
    }
}

/// プロジェクション用のDynamoDBアイテム
#[derive(Debug, Clone)]
pub struct ProjectionItem {
    pub family_id: String,
    pub todo: Todo,
    pub is_active: bool,
}

impl ProjectionItem {
    pub fn new(family_id: String, todo: Todo) -> Self {
        let is_active = todo.is_active();
        Self {
            family_id,
            todo,
            is_active,
        }
    }

    /// DynamoDbItem に変換
    pub fn to_dynamodb_item(&self) -> Result<DynamoDbItem, String> {
        let keys = if self.is_active {
            DynamoDbKeys::for_active_todo(&self.family_id, self.todo.id.as_str())
        } else {
            DynamoDbKeys::for_todo_projection(&self.family_id, &self.todo.id)
        };
        
        let data = serde_json::to_value(&self.todo)
            .map_err(|e| format!("Failed to serialize todo: {}", e))?;

        Ok(DynamoDbItem::new(
            keys,
            EntityType::Projection,
            data,
            self.todo.version,
            None, // プロジェクションは永続化
        ))
    }

    /// DynamoDbItem から復元
    pub fn from_dynamodb_item(item: &DynamoDbItem) -> Result<Self, String> {
        if item.entity_type != EntityType::Projection {
            return Err("Item is not a Projection".to_string());
        }

        let todo: Todo = serde_json::from_value(item.data.clone())
            .map_err(|e| format!("Failed to deserialize todo: {}", e))?;

        // PKからfamily_idを抽出
        let family_id = item.pk.strip_prefix("FAMILY#")
            .ok_or("Invalid PK format for Projection")?
            .to_string();

        Ok(Self::new(family_id, todo))
    }
}

/// スナップショット用のDynamoDBアイテム
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    pub todo: Todo,
    pub event_count: usize,
    pub last_event_id: String,
}

#[derive(Debug, Clone)]
pub struct SnapshotItem {
    pub family_id: String,
    pub todo_id: TodoId,
    pub snapshot_id: String,
    pub data: SnapshotData,
    pub ttl: Option<i64>,
}

impl SnapshotItem {
    pub fn new(
        family_id: String,
        todo_id: TodoId,
        data: SnapshotData,
        ttl_days: Option<u32>,
    ) -> Self {
        let snapshot_id = ulid::Ulid::new().to_string();
        let ttl = ttl_days.map(|days| {
            Utc::now().timestamp() + (days as i64 * 24 * 60 * 60)
        });

        Self {
            family_id,
            todo_id,
            snapshot_id,
            data,
            ttl,
        }
    }

    /// DynamoDbItem に変換
    pub fn to_dynamodb_item(&self) -> Result<DynamoDbItem, String> {
        let keys = DynamoDbKeys::for_snapshot(&self.family_id, &self.todo_id, &self.snapshot_id);
        
        let data = serde_json::to_value(&self.data)
            .map_err(|e| format!("Failed to serialize snapshot: {}", e))?;

        Ok(DynamoDbItem::new(
            keys,
            EntityType::Snapshot,
            data,
            self.data.todo.version,
            self.ttl,
        ))
    }

    /// DynamoDbItem から復元
    pub fn from_dynamodb_item(item: &DynamoDbItem) -> Result<Self, String> {
        if item.entity_type != EntityType::Snapshot {
            return Err("Item is not a Snapshot".to_string());
        }

        let data: SnapshotData = serde_json::from_value(item.data.clone())
            .map_err(|e| format!("Failed to deserialize snapshot: {}", e))?;

        // PKからfamily_idを抽出
        let family_id = item.pk.strip_prefix("FAMILY#")
            .ok_or("Invalid PK format for Snapshot")?
            .to_string();

        // SKからtodo_idとsnapshot_idを抽出
        let sk_parts: Vec<&str> = item.sk.split('#').collect();
        if sk_parts.len() != 4 || sk_parts[0] != "TODO" || sk_parts[1] != "SNAPSHOT" {
            return Err("Invalid SK format for Snapshot".to_string());
        }

        let todo_id = TodoId::from_string(sk_parts[2].to_string())
            .map_err(|e| format!("Invalid TodoId in SK: {}", e))?;
        let snapshot_id = sk_parts[3].to_string();

        Ok(Self {
            family_id,
            todo_id,
            snapshot_id,
            data,
            ttl: item.ttl,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::TodoEvent;

    #[test]
    fn test_entity_type_conversion() {
        assert_eq!(EntityType::Event.as_str(), "Event");
        assert_eq!(EntityType::from("Event"), EntityType::Event);
        assert_eq!(EntityType::from("Unknown"), EntityType::Event); // デフォルト
    }

    #[test]
    fn test_dynamodb_keys_for_event() {
        let keys = DynamoDbKeys::for_event("family123", "01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert_eq!(keys.pk, "FAMILY#family123");
        assert_eq!(keys.sk, "EVENT#01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert!(keys.gsi1_pk.is_none());
        assert!(keys.gsi1_sk.is_none());
    }

    #[test]
    fn test_dynamodb_keys_for_active_todo() {
        let keys = DynamoDbKeys::for_active_todo("family123", "01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert_eq!(keys.pk, "FAMILY#family123");
        assert_eq!(keys.sk, "TODO#CURRENT#01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert_eq!(keys.gsi1_pk, Some("FAMILY#family123#ACTIVE".to_string()));
        assert_eq!(keys.gsi1_sk, Some("01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string()));
    }

    #[test]
    fn test_dynamodb_item_serialization() {
        let keys = DynamoDbKeys::for_event("family123", "event123");
        let data = serde_json::json!({"test": "data"});
        let item = DynamoDbItem::new(keys, EntityType::Event, data, 1, None);

        let attr_map = item.to_attribute_map();
        assert!(attr_map.contains_key("PK"));
        assert!(attr_map.contains_key("SK"));
        assert!(attr_map.contains_key("EntityType"));
        assert!(attr_map.contains_key("Data"));
        assert!(attr_map.contains_key("Version"));

        let restored_item = DynamoDbItem::from_attribute_map(&attr_map).unwrap();
        assert_eq!(restored_item.pk, item.pk);
        assert_eq!(restored_item.sk, item.sk);
        assert_eq!(restored_item.entity_type, item.entity_type);
        assert_eq!(restored_item.version, item.version);
    }

    #[test]
    fn test_event_item_conversion() {
        let todo_id = TodoId::new();
        let event = TodoEvent::new_todo_created(
            todo_id,
            "Test Todo".to_string(),
            None,
            vec![],
            "user123".to_string(),
        );

        let event_item = EventItem::new("family123".to_string(), event.clone(), 1);
        let dynamodb_item = event_item.to_dynamodb_item().unwrap();

        assert_eq!(dynamodb_item.entity_type, EntityType::Event);
        assert!(dynamodb_item.pk.starts_with("FAMILY#family123"));
        assert!(dynamodb_item.sk.starts_with("EVENT#"));

        let restored_event_item = EventItem::from_dynamodb_item(&dynamodb_item).unwrap();
        assert_eq!(restored_event_item.family_id, "family123");
        assert_eq!(restored_event_item.event.todo_id(), event.todo_id());
    }
}
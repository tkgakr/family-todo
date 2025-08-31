use crate::errors::DomainError;
use crate::events::TodoEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// ULID-based identifier for Todo entities
///
/// ULIDs provide lexicographically sortable, globally unique identifiers
/// with embedded timestamp information for natural ordering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TodoId(String);

impl TodoId {
    /// Creates a new TodoId with a fresh ULID
    pub fn new() -> Self {
        Self(ulid::Ulid::new().to_string())
    }

    /// Creates a TodoId from a validated ULID string
    ///
    /// # Arguments
    /// * `id` - A valid ULID string
    ///
    /// # Returns
    /// * `Ok(TodoId)` if the string is a valid ULID
    /// * `Err(DomainError)` if the string is not a valid ULID
    pub fn from_string(id: String) -> Result<Self, DomainError> {
        // Validate that the string is a valid ULID
        ulid::Ulid::from_string(&id).map_err(|_| DomainError::InvalidTodoId(id.clone()))?;
        Ok(Self(id))
    }

    /// Creates a TodoId from a ULID without validation (unsafe)
    ///
    /// This should only be used when you're certain the string is valid,
    /// such as when deserializing from trusted sources.
    pub fn from_string_unchecked(id: String) -> Self {
        Self(id)
    }

    /// Extracts the timestamp from the ULID
    ///
    /// # Returns
    /// * `Some(timestamp_ms)` if the ULID is valid
    /// * `None` if the ULID is invalid
    pub fn timestamp_ms(&self) -> Option<u64> {
        ulid::Ulid::from_string(&self.0)
            .ok()
            .map(|ulid| ulid.timestamp_ms())
    }

    /// Extracts the timestamp as a DateTime<Utc>
    ///
    /// # Returns
    /// * `Some(DateTime<Utc>)` if the ULID is valid and timestamp can be converted
    /// * `None` if the ULID is invalid or timestamp conversion fails
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamp_ms()
            .and_then(|ms| DateTime::from_timestamp_millis(ms as i64))
    }

    /// Returns the string representation of the TodoId
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validates that the TodoId contains a valid ULID
    pub fn is_valid(&self) -> bool {
        ulid::Ulid::from_string(&self.0).is_ok()
    }
}

impl Default for TodoId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TodoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TodoId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s.to_string())
    }
}

impl From<TodoId> for String {
    fn from(id: TodoId) -> Self {
        id.0
    }
}

impl AsRef<str> for TodoId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod todo_id_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_new_todo_id_creates_valid_ulid() {
        let id = TodoId::new();
        assert!(id.is_valid());
        assert_eq!(id.as_str().len(), 26); // ULID length is 26 characters
    }

    #[test]
    fn test_new_todo_ids_are_unique() {
        let mut ids = HashSet::new();
        for _ in 0..1000 {
            let id = TodoId::new();
            assert!(
                ids.insert(id.as_str().to_string()),
                "Generated duplicate TodoId"
            );
        }
    }

    #[test]
    fn test_from_string_with_valid_ulid() {
        let ulid_str = ulid::Ulid::new().to_string();
        let result = TodoId::from_string(ulid_str.clone());

        assert!(result.is_ok());
        let todo_id = result.unwrap();
        assert_eq!(todo_id.as_str(), ulid_str);
        assert!(todo_id.is_valid());
    }

    #[test]
    fn test_from_string_with_invalid_ulid() {
        let invalid_ids = vec![
            "invalid",
            "123",
            "01ARZ3NDEKTSV4RRFFQ69G5FA",   // Too short
            "01ARZ3NDEKTSV4RRFFQ69G5FAVV", // Too long
            "01ARZ3NDEKTSV4RRFFQ69G5F@V",  // Invalid character
            "",
        ];

        for invalid_id in invalid_ids {
            let result = TodoId::from_string(invalid_id.to_string());
            assert!(
                result.is_err(),
                "Expected error for invalid ULID: {invalid_id}"
            );
        }
    }

    #[test]
    fn test_from_string_unchecked() {
        let ulid_str = ulid::Ulid::new().to_string();
        let todo_id = TodoId::from_string_unchecked(ulid_str.clone());
        assert_eq!(todo_id.as_str(), ulid_str);
    }

    #[test]
    fn test_timestamp_ms_extraction() {
        let ulid = ulid::Ulid::new();
        let expected_timestamp = ulid.timestamp_ms();
        let todo_id = TodoId::from_string_unchecked(ulid.to_string());

        let extracted_timestamp = todo_id.timestamp_ms();
        assert_eq!(extracted_timestamp, Some(expected_timestamp));
    }

    #[test]
    fn test_timestamp_extraction() {
        let ulid = ulid::Ulid::new();
        let todo_id = TodoId::from_string_unchecked(ulid.to_string());

        let timestamp = todo_id.timestamp();
        assert!(timestamp.is_some());

        let timestamp = timestamp.unwrap();
        let expected_ms = ulid.timestamp_ms();
        let actual_ms = timestamp.timestamp_millis() as u64;

        // Allow for small differences due to precision
        assert!((actual_ms as i64 - expected_ms as i64).abs() < 1000);
    }

    #[test]
    fn test_timestamp_with_invalid_ulid() {
        let invalid_todo_id = TodoId("invalid".to_string());
        assert_eq!(invalid_todo_id.timestamp_ms(), None);
        assert_eq!(invalid_todo_id.timestamp(), None);
    }

    #[test]
    fn test_is_valid() {
        let valid_id = TodoId::new();
        assert!(valid_id.is_valid());

        let invalid_id = TodoId("invalid".to_string());
        assert!(!invalid_id.is_valid());
    }

    #[test]
    fn test_display_trait() {
        let ulid_str = ulid::Ulid::new().to_string();
        let todo_id = TodoId::from_string_unchecked(ulid_str.clone());
        assert_eq!(format!("{todo_id}"), ulid_str);
    }

    #[test]
    fn test_from_str_trait() {
        let ulid_str = ulid::Ulid::new().to_string();
        let result: Result<TodoId, _> = ulid_str.parse();

        assert!(result.is_ok());
        let todo_id = result.unwrap();
        assert_eq!(todo_id.as_str(), ulid_str);
    }

    #[test]
    fn test_from_str_trait_with_invalid() {
        let result: Result<TodoId, _> = "invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_into_string() {
        let ulid_str = ulid::Ulid::new().to_string();
        let todo_id = TodoId::from_string_unchecked(ulid_str.clone());
        let converted: String = todo_id.into();
        assert_eq!(converted, ulid_str);
    }

    #[test]
    fn test_as_ref() {
        let ulid_str = ulid::Ulid::new().to_string();
        let todo_id = TodoId::from_string_unchecked(ulid_str.clone());
        let as_ref: &str = todo_id.as_ref();
        assert_eq!(as_ref, ulid_str);
    }

    #[test]
    fn test_equality_and_hashing() {
        let ulid_str = ulid::Ulid::new().to_string();
        let id1 = TodoId::from_string_unchecked(ulid_str.clone());
        let id2 = TodoId::from_string_unchecked(ulid_str.clone());
        let id3 = TodoId::new();

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);

        // Test that equal TodoIds have the same hash
        let mut set = HashSet::new();
        set.insert(id1.clone());
        assert!(set.contains(&id2));
        assert!(!set.contains(&id3));
    }

    #[test]
    fn test_serde_serialization() {
        let todo_id = TodoId::new();
        let serialized = serde_json::to_string(&todo_id).unwrap();
        let deserialized: TodoId = serde_json::from_str(&serialized).unwrap();

        assert_eq!(todo_id, deserialized);
    }

    #[test]
    fn test_chronological_ordering() {
        // Create TodoIds with a small delay to ensure different timestamps
        let id1 = TodoId::new();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = TodoId::new();

        let timestamp1 = id1.timestamp_ms().unwrap();
        let timestamp2 = id2.timestamp_ms().unwrap();

        assert!(
            timestamp1 <= timestamp2,
            "TodoIds should be chronologically ordered"
        );
    }

    #[test]
    fn test_default_trait() {
        let default_id = TodoId::default();
        assert!(default_id.is_valid());
        assert_eq!(default_id.as_str().len(), 26);
    }
}

#[cfg(test)]
mod todo_aggregate_tests {
    use super::*;
    use crate::events::TodoEvent;

    fn create_test_todo_id() -> TodoId {
        TodoId::new()
    }

    fn create_test_created_event(todo_id: TodoId) -> TodoEvent {
        TodoEvent::new_todo_created(
            todo_id,
            "テストToDo".to_string(),
            Some("テスト説明".to_string()),
            vec!["タグ1".to_string(), "タグ2".to_string()],
            "user123".to_string(),
        )
    }

    #[test]
    fn test_todo_from_created_event() {
        let todo_id = create_test_todo_id();
        let event = create_test_created_event(todo_id.clone());

        let todo = Todo::from_created_event(&event).unwrap();

        assert_eq!(todo.id, todo_id);
        assert_eq!(todo.title, "テストToDo");
        assert_eq!(todo.description, Some("テスト説明".to_string()));
        assert_eq!(todo.tags, vec!["タグ1", "タグ2"]);
        assert!(!todo.completed);
        assert!(!todo.deleted);
        assert_eq!(todo.created_by, "user123");
        assert_eq!(todo.version, 1);
        assert!(todo.is_valid());
        assert!(todo.is_active());
    }

    #[test]
    fn test_todo_from_created_event_with_wrong_event_type() {
        let todo_id = create_test_todo_id();
        let event = TodoEvent::new_todo_updated(
            todo_id,
            Some("更新されたタイトル".to_string()),
            None,
            "user123".to_string(),
        );

        let result = Todo::from_created_event(&event);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_created_event() {
        let todo_id = create_test_todo_id();
        let event = create_test_created_event(todo_id.clone());

        let mut todo = Todo::new(
            todo_id.clone(),
            "".to_string(),
            None,
            vec![],
            "".to_string(),
            Utc::now(),
        );

        let result = todo.apply(event.clone());
        assert!(result.is_ok());

        assert_eq!(todo.id, todo_id);
        assert_eq!(todo.title, "テストToDo");
        assert_eq!(todo.description, Some("テスト説明".to_string()));
        assert_eq!(todo.tags, vec!["タグ1", "タグ2"]);
        assert!(!todo.completed);
        assert!(!todo.deleted);
        assert_eq!(todo.created_by, "user123");
        assert_eq!(todo.version, 1);
    }

    #[test]
    fn test_apply_created_event_twice_fails() {
        let todo_id = create_test_todo_id();
        let event = create_test_created_event(todo_id.clone());

        let mut todo = Todo::from_created_event(&event).unwrap();

        // 2回目の作成イベント適用は失敗する
        let result = todo.apply(event);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_updated_event() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id.clone());
        let mut todo = Todo::from_created_event(&created_event).unwrap();

        let updated_event = TodoEvent::new_todo_updated(
            todo_id,
            Some("更新されたタイトル".to_string()),
            Some("更新された説明".to_string()),
            "user456".to_string(),
        );

        let result = todo.apply(updated_event);
        assert!(result.is_ok());

        assert_eq!(todo.title, "更新されたタイトル");
        assert_eq!(todo.description, Some("更新された説明".to_string()));
        assert_eq!(todo.version, 2);
        assert!(todo.is_active());
    }

    #[test]
    fn test_apply_updated_event_with_partial_update() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id.clone());
        let mut todo = Todo::from_created_event(&created_event).unwrap();

        // タイトルのみ更新、説明はクリア
        let updated_event = TodoEvent::new_todo_updated(
            todo_id,
            Some("新しいタイトル".to_string()),
            None,
            "user456".to_string(),
        );

        let result = todo.apply(updated_event);
        assert!(result.is_ok());

        assert_eq!(todo.title, "新しいタイトル");
        assert_eq!(todo.description, None); // 説明がクリアされる
        assert_eq!(todo.version, 2);
    }

    #[test]
    fn test_apply_completed_event() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id.clone());
        let mut todo = Todo::from_created_event(&created_event).unwrap();

        let completed_event = TodoEvent::new_todo_completed(todo_id, "user789".to_string());

        let result = todo.apply(completed_event);
        assert!(result.is_ok());

        assert!(todo.completed);
        assert!(!todo.is_active()); // 完了済みなのでアクティブではない
        assert!(todo.is_completed());
        assert_eq!(todo.version, 2);
    }

    #[test]
    fn test_apply_completed_event_twice_fails() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id.clone());
        let mut todo = Todo::from_created_event(&created_event).unwrap();

        let completed_event = TodoEvent::new_todo_completed(todo_id.clone(), "user789".to_string());

        // 1回目は成功
        let result = todo.apply(completed_event.clone());
        assert!(result.is_ok());

        // 2回目は失敗
        let result = todo.apply(completed_event);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_deleted_event() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id.clone());
        let mut todo = Todo::from_created_event(&created_event).unwrap();

        let deleted_event = TodoEvent::new_todo_deleted(
            todo_id,
            "user999".to_string(),
            Some("不要になったため".to_string()),
        );

        let result = todo.apply(deleted_event);
        assert!(result.is_ok());

        assert!(todo.deleted);
        assert!(!todo.is_active()); // 削除済みなのでアクティブではない
        assert!(todo.is_deleted());
        assert_eq!(todo.version, 2);
    }

    #[test]
    fn test_apply_deleted_event_twice_fails() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id.clone());
        let mut todo = Todo::from_created_event(&created_event).unwrap();

        let deleted_event = TodoEvent::new_todo_deleted(
            todo_id.clone(),
            "user999".to_string(),
            Some("不要になったため".to_string()),
        );

        // 1回目は成功
        let result = todo.apply(deleted_event.clone());
        assert!(result.is_ok());

        // 2回目は失敗
        let result = todo.apply(deleted_event);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_event_to_deleted_todo_fails() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id.clone());
        let mut todo = Todo::from_created_event(&created_event).unwrap();

        // まず削除
        let deleted_event = TodoEvent::new_todo_deleted(
            todo_id.clone(),
            "user999".to_string(),
            Some("削除".to_string()),
        );
        todo.apply(deleted_event).unwrap();

        // 削除後に更新イベントを適用しようとすると失敗
        let updated_event = TodoEvent::new_todo_updated(
            todo_id,
            Some("更新".to_string()),
            None,
            "user123".to_string(),
        );

        let result = todo.apply(updated_event);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_event_with_wrong_todo_id_fails() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id);
        let mut todo = Todo::from_created_event(&created_event).unwrap();

        // 異なるTodoIdのイベントを適用しようとすると失敗
        let wrong_todo_id = create_test_todo_id();
        let updated_event = TodoEvent::new_todo_updated(
            wrong_todo_id,
            Some("更新".to_string()),
            None,
            "user123".to_string(),
        );

        let result = todo.apply(updated_event);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_events_complete_lifecycle() {
        let todo_id = create_test_todo_id();

        let events = vec![
            TodoEvent::new_todo_created(
                todo_id.clone(),
                "初期タイトル".to_string(),
                Some("初期説明".to_string()),
                vec!["タグ1".to_string()],
                "user123".to_string(),
            ),
            TodoEvent::new_todo_updated(
                todo_id.clone(),
                Some("更新されたタイトル".to_string()),
                None,
                "user456".to_string(),
            ),
            TodoEvent::new_todo_completed(todo_id.clone(), "user789".to_string()),
        ];

        let todo = Todo::from_events(events).unwrap();

        assert_eq!(todo.title, "更新されたタイトル");
        assert_eq!(todo.description, None);
        assert!(todo.completed);
        assert!(!todo.deleted);
        assert_eq!(todo.version, 3);
        assert!(todo.is_completed());
        assert!(!todo.is_active());
    }

    #[test]
    fn test_from_events_empty_fails() {
        let result = Todo::from_events(vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_events_without_created_event_fails() {
        let todo_id = create_test_todo_id();
        let events = vec![TodoEvent::new_todo_updated(
            todo_id,
            Some("更新".to_string()),
            None,
            "user123".to_string(),
        )];

        let result = Todo::from_events(events);
        assert!(result.is_err());
    }

    #[test]
    fn test_todo_validation() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id);
        let todo = Todo::from_created_event(&created_event).unwrap();

        assert!(todo.is_valid());

        // 無効なTodoを作成してテスト
        let mut invalid_todo = todo.clone();
        invalid_todo.title = "".to_string(); // 空のタイトル
        assert!(!invalid_todo.is_valid());

        let mut invalid_todo2 = todo.clone();
        invalid_todo2.title = "a".repeat(201); // 長すぎるタイトル
        assert!(!invalid_todo2.is_valid());

        let mut invalid_todo3 = todo.clone();
        invalid_todo3.version = 0; // 無効なバージョン
        assert!(!invalid_todo3.is_valid());
    }

    #[test]
    fn test_todo_state_checks() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id.clone());
        let mut todo = Todo::from_created_event(&created_event).unwrap();

        // 初期状態
        assert!(todo.is_active());
        assert!(!todo.is_completed());
        assert!(!todo.is_deleted());

        // 完了状態
        let completed_event = TodoEvent::new_todo_completed(todo_id.clone(), "user123".to_string());
        todo.apply(completed_event).unwrap();

        assert!(!todo.is_active());
        assert!(todo.is_completed());
        assert!(!todo.is_deleted());

        // 新しいTodoで削除状態をテスト
        let mut todo2 = Todo::from_created_event(&created_event).unwrap();
        let deleted_event =
            TodoEvent::new_todo_deleted(todo_id, "user123".to_string(), Some("削除".to_string()));
        todo2.apply(deleted_event).unwrap();

        assert!(!todo2.is_active());
        assert!(!todo2.is_completed());
        assert!(todo2.is_deleted());
    }

    #[test]
    fn test_todo_version_tracking() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id.clone());
        let mut todo = Todo::from_created_event(&created_event).unwrap();

        assert_eq!(todo.current_version(), 1);

        let updated_event = TodoEvent::new_todo_updated(
            todo_id.clone(),
            Some("更新".to_string()),
            None,
            "user123".to_string(),
        );
        todo.apply(updated_event).unwrap();

        assert_eq!(todo.current_version(), 2);

        let completed_event = TodoEvent::new_todo_completed(todo_id, "user123".to_string());
        todo.apply(completed_event).unwrap();

        assert_eq!(todo.current_version(), 3);
    }

    #[test]
    fn test_todo_serde_serialization() {
        let todo_id = create_test_todo_id();
        let created_event = create_test_created_event(todo_id);
        let todo = Todo::from_created_event(&created_event).unwrap();

        // シリアライゼーション
        let serialized = serde_json::to_string(&todo).unwrap();

        // デシリアライゼーション
        let deserialized: Todo = serde_json::from_str(&serialized).unwrap();

        assert_eq!(todo.id, deserialized.id);
        assert_eq!(todo.title, deserialized.title);
        assert_eq!(todo.description, deserialized.description);
        assert_eq!(todo.tags, deserialized.tags);
        assert_eq!(todo.completed, deserialized.completed);
        assert_eq!(todo.deleted, deserialized.deleted);
        assert_eq!(todo.version, deserialized.version);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: TodoId,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub completed: bool,
    pub created_by: String, // UserId
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u64,
    pub deleted: bool, // 論理削除フラグ
}

impl Todo {
    /// 新しいTodoを作成（初期状態）
    pub fn new(
        id: TodoId,
        title: String,
        description: Option<String>,
        tags: Vec<String>,
        created_by: String,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            title,
            description,
            tags,
            completed: false,
            created_by,
            created_at,
            updated_at: created_at,
            version: 0, // イベント適用前は0
            deleted: false,
        }
    }

    /// TodoCreatedV2イベントからTodoを作成
    pub fn from_created_event(event: &TodoEvent) -> Result<Self, DomainError> {
        match event {
            TodoEvent::TodoCreatedV2 {
                todo_id,
                title,
                description,
                tags,
                created_by,
                timestamp,
                ..
            } => {
                let mut todo = Self::new(
                    todo_id.clone(),
                    title.clone(),
                    description.clone(),
                    tags.clone(),
                    created_by.clone(),
                    *timestamp,
                );
                todo.apply(event.clone())?;
                Ok(todo)
            }
            _ => Err(DomainError::InvalidEvent(
                "Expected TodoCreatedV2 event".to_string(),
            )),
        }
    }

    /// イベントストリームからTodoを再構築
    pub fn from_events(events: Vec<TodoEvent>) -> Result<Self, DomainError> {
        if events.is_empty() {
            return Err(DomainError::InvalidEvent(
                "Cannot create Todo from empty event stream".to_string(),
            ));
        }

        // 最初のイベントはTodoCreatedでなければならない
        let first_event = &events[0];
        let mut todo = Self::from_created_event(first_event)?;

        // 残りのイベントを順次適用
        for event in events.iter().skip(1) {
            todo.apply(event.clone())?;
        }

        Ok(todo)
    }

    /// イベントをTodoの状態に適用する
    pub fn apply(&mut self, event: TodoEvent) -> Result<(), DomainError> {
        // イベントのバリデーション
        event.validate()?;

        // TodoIdの一致確認
        if event.todo_id() != &self.id {
            return Err(DomainError::InvalidEvent(format!(
                "Event TodoId {} does not match aggregate TodoId {}",
                event.todo_id(),
                self.id
            )));
        }

        // 削除済みのTodoには新しいイベントを適用できない（削除イベント以外）
        if self.deleted && !matches!(event, TodoEvent::TodoDeletedV1 { .. }) {
            return Err(DomainError::InvalidEvent(
                "Cannot apply events to deleted Todo".to_string(),
            ));
        }

        match event {
            TodoEvent::TodoCreatedV2 {
                todo_id,
                title,
                description,
                tags,
                created_by,
                timestamp,
                ..
            } => {
                // 作成イベントは初回のみ適用可能
                if self.version > 0 {
                    return Err(DomainError::InvalidEvent(
                        "TodoCreated event can only be applied to new Todo".to_string(),
                    ));
                }

                self.id = todo_id;
                self.title = title;
                self.description = description;
                self.tags = tags;
                self.created_by = created_by;
                self.created_at = timestamp;
                self.updated_at = timestamp;
                self.completed = false;
                self.deleted = false;
                self.version = 1;
            }
            TodoEvent::TodoUpdatedV1 {
                title,
                description,
                timestamp,
                ..
            } => {
                // タイトルが指定されている場合のみ更新
                if let Some(new_title) = title {
                    self.title = new_title;
                }

                // 説明の更新（Noneの場合は説明をクリア）
                self.description = description;

                self.updated_at = timestamp;
                self.version += 1;
            }
            TodoEvent::TodoCompletedV1 { timestamp, .. } => {
                // 既に完了している場合はエラー
                if self.completed {
                    return Err(DomainError::InvalidEvent(
                        "Todo is already completed".to_string(),
                    ));
                }

                self.completed = true;
                self.updated_at = timestamp;
                self.version += 1;
            }
            TodoEvent::TodoDeletedV1 { timestamp, .. } => {
                // 既に削除されている場合はエラー
                if self.deleted {
                    return Err(DomainError::InvalidEvent(
                        "Todo is already deleted".to_string(),
                    ));
                }

                self.deleted = true;
                self.updated_at = timestamp;
                self.version += 1;
            }
        }

        Ok(())
    }

    /// Todoが有効な状態かどうかをチェック
    pub fn is_valid(&self) -> bool {
        !self.title.is_empty()
            && self.title.len() <= 200
            && !self.created_by.is_empty()
            && self.id.is_valid()
            && self.version > 0
    }

    /// Todoがアクティブ（未完了かつ未削除）かどうかをチェック
    pub fn is_active(&self) -> bool {
        !self.completed && !self.deleted
    }

    /// Todoが削除されているかどうかをチェック
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    /// Todoが完了しているかどうかをチェック
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Todoの現在のバージョンを取得
    pub fn current_version(&self) -> u64 {
        self.version
    }
}

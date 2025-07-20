use crate::TodoId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Todo {
    pub id: TodoId,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub last_modified_at: DateTime<Utc>,
    pub last_modified_by: String,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoEvent {
    TodoCreated {
        todo_id: TodoId,
        title: String,
        description: Option<String>,
        created_by: String,
        timestamp: DateTime<Utc>,
    },
    TodoUpdated {
        todo_id: TodoId,
        title: Option<String>,
        description: Option<String>,
        updated_by: String,
        timestamp: DateTime<Utc>,
    },
    TodoCompleted {
        todo_id: TodoId,
        completed_by: String,
        timestamp: DateTime<Utc>,
    },
    TodoDeleted {
        todo_id: TodoId,
        deleted_by: String,
        timestamp: DateTime<Utc>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_creation() {
        // Arrange: テスト用データ
        let todo_id = TodoId::new();
        let title = "テストTODO".to_string();
        let created_by = "user123".to_string();
        let now = Utc::now();

        // Act: Todo構造体を作成
        let todo = Todo {
            id: todo_id.clone(),
            title: title.clone(),
            description: None,
            completed: false,
            created_at: now,
            created_by: created_by.clone(),
            last_modified_at: now,
            last_modified_by: created_by.clone(),
            version: 1,
        };

        // Assert: フィールドが正しく設定されていることを確認
        assert_eq!(todo.id, todo_id);
        assert_eq!(todo.title, title);
        assert_eq!(todo.description, None);
        assert!(!todo.completed);
        assert_eq!(todo.created_by, created_by);
        assert_eq!(todo.version, 1);
    }

    #[test]
    fn test_todo_created_event() {
        // Arrange: イベント用データ
        let todo_id = TodoId::new();
        let title = "新しいTODO".to_string();
        let created_by = "user456".to_string();
        let timestamp = Utc::now();

        // Act: TodoCreatedイベントを作成
        let event = TodoEvent::TodoCreated {
            todo_id: todo_id.clone(),
            title: title.clone(),
            description: None,
            created_by: created_by.clone(),
            timestamp,
        };

        // Assert: イベントが正しく作成されることを確認
        match event {
            TodoEvent::TodoCreated { todo_id: id, title: t, created_by: cb, .. } => {
                assert_eq!(id, todo_id);
                assert_eq!(t, title);
                assert_eq!(cb, created_by);
            }
            _ => panic!("Expected TodoCreated event"),
        }
    }
}
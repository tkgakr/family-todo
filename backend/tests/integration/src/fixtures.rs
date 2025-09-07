use chrono::Utc;
use shared::domain::{
    aggregates::{Todo, TodoSnapshot},
    events::TodoEvent,
    identifiers::{EventId, FamilyId, TodoId, UserId},
};

pub struct TodoFixtures;

impl TodoFixtures {
    pub fn create_todo_event(
        user_id: UserId,
        todo_id: TodoId,
        title: &str,
        description: Option<&str>,
    ) -> TodoEvent {
        TodoEvent::TodoCreatedV2 {
            event_id: EventId::new(),
            todo_id,
            title: title.to_string(),
            description: description.map(|s| s.to_string()),
            tags: Vec::new(),
            created_by: user_id,
            timestamp: Utc::now(),
        }
    }

    pub fn update_todo_event(
        todo_id: TodoId,
        user_id: UserId,
        title: Option<&str>,
        description: Option<&str>,
    ) -> TodoEvent {
        TodoEvent::TodoUpdatedV1 {
            event_id: EventId::new(),
            todo_id,
            title: title.map(|s| s.to_string()),
            description: description.map(|s| s.to_string()),
            updated_by: user_id,
            timestamp: Utc::now(),
        }
    }

    pub fn complete_todo_event(todo_id: TodoId, user_id: UserId) -> TodoEvent {
        TodoEvent::TodoCompletedV1 {
            event_id: EventId::new(),
            todo_id,
            completed_by: user_id,
            timestamp: Utc::now(),
        }
    }

    pub fn delete_todo_event(todo_id: TodoId, user_id: UserId) -> TodoEvent {
        TodoEvent::TodoDeletedV1 {
            event_id: EventId::new(),
            todo_id,
            deleted_by: user_id,
            reason: None,
            timestamp: Utc::now(),
        }
    }

    pub fn sample_todo(
        user_id: UserId,
        todo_id: TodoId,
    ) -> Todo {
        Todo::new(
            todo_id,
            "Sample Todo".to_string(),
            Some("Sample Description".to_string()),
            vec!["sample".to_string()],
            user_id,
        ).unwrap()
    }
}

// ヘルパー関数
pub fn create_sample_family_id() -> FamilyId {
    FamilyId::new()
}

pub fn create_sample_todo_id() -> TodoId {
    TodoId::new()
}

pub fn create_sample_todo() -> Todo {
    let user_id = UserId::new();
    let todo_id = TodoId::new();
    TodoFixtures::sample_todo(user_id, todo_id)
}
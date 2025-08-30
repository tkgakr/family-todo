use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::todo::TodoId;

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
    },
    TodoUpdatedV1 {
        event_id: String,
        todo_id: TodoId,
        title: Option<String>,
        description: Option<String>,
        updated_by: String, // UserId
        timestamp: DateTime<Utc>,
    },
    TodoCompletedV1 {
        event_id: String,
        todo_id: TodoId,
        completed_by: String, // UserId
        timestamp: DateTime<Utc>,
    },
    TodoDeletedV1 {
        event_id: String,
        todo_id: TodoId,
        deleted_by: String, // UserId
        reason: Option<String>,
        timestamp: DateTime<Utc>,
    },
}

impl TodoEvent {
    pub fn todo_id(&self) -> &TodoId {
        match self {
            TodoEvent::TodoCreatedV2 { todo_id, .. } => todo_id,
            TodoEvent::TodoUpdatedV1 { todo_id, .. } => todo_id,
            TodoEvent::TodoCompletedV1 { todo_id, .. } => todo_id,
            TodoEvent::TodoDeletedV1 { todo_id, .. } => todo_id,
        }
    }
    
    pub fn event_id(&self) -> &str {
        match self {
            TodoEvent::TodoCreatedV2 { event_id, .. } => event_id,
            TodoEvent::TodoUpdatedV1 { event_id, .. } => event_id,
            TodoEvent::TodoCompletedV1 { event_id, .. } => event_id,
            TodoEvent::TodoDeletedV1 { event_id, .. } => event_id,
        }
    }
}
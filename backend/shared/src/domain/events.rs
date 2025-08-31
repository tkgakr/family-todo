use crate::domain::identifiers::{EventId, TodoId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum TodoEvent {
    #[serde(rename = "todo_created_v2")]
    TodoCreatedV2 {
        event_id: EventId,
        todo_id: TodoId,
        title: String,
        description: Option<String>,
        tags: Vec<String>,
        created_by: UserId,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "todo_updated_v1")]
    TodoUpdatedV1 {
        event_id: EventId,
        todo_id: TodoId,
        title: Option<String>,
        description: Option<String>,
        updated_by: UserId,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "todo_completed_v1")]
    TodoCompletedV1 {
        event_id: EventId,
        todo_id: TodoId,
        completed_by: UserId,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "todo_deleted_v1")]
    TodoDeletedV1 {
        event_id: EventId,
        todo_id: TodoId,
        deleted_by: UserId,
        reason: Option<String>,
        timestamp: DateTime<Utc>,
    },
}

impl TodoEvent {
    pub fn event_id(&self) -> &EventId {
        match self {
            TodoEvent::TodoCreatedV2 { event_id, .. } => event_id,
            TodoEvent::TodoUpdatedV1 { event_id, .. } => event_id,
            TodoEvent::TodoCompletedV1 { event_id, .. } => event_id,
            TodoEvent::TodoDeletedV1 { event_id, .. } => event_id,
        }
    }

    pub fn todo_id(&self) -> &TodoId {
        match self {
            TodoEvent::TodoCreatedV2 { todo_id, .. } => todo_id,
            TodoEvent::TodoUpdatedV1 { todo_id, .. } => todo_id,
            TodoEvent::TodoCompletedV1 { todo_id, .. } => todo_id,
            TodoEvent::TodoDeletedV1 { todo_id, .. } => todo_id,
        }
    }

    pub fn timestamp(&self) -> &DateTime<Utc> {
        match self {
            TodoEvent::TodoCreatedV2 { timestamp, .. } => timestamp,
            TodoEvent::TodoUpdatedV1 { timestamp, .. } => timestamp,
            TodoEvent::TodoCompletedV1 { timestamp, .. } => timestamp,
            TodoEvent::TodoDeletedV1 { timestamp, .. } => timestamp,
        }
    }

    pub fn upcast(self) -> TodoEvent {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEventEnvelope {
    pub event: TodoEvent,
    pub stream_id: TodoId,
    pub stream_version: u64,
    pub occurred_at: DateTime<Utc>,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}

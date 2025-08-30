use crate::events::TodoEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoId(pub String);

impl TodoId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new().to_string())
    }

    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    pub fn timestamp_ms(&self) -> Option<u64> {
        ulid::Ulid::from_string(&self.0)
            .ok()
            .map(|ulid| ulid.timestamp_ms())
    }
}

impl Default for TodoId {
    fn default() -> Self {
        Self::new()
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
}

impl Todo {
    pub fn apply(&mut self, event: TodoEvent) {
        // Event application logic will be implemented in task 2.3
        match event {
            TodoEvent::TodoCreatedV2 { .. } => {
                // Implementation placeholder
            }
            TodoEvent::TodoUpdatedV1 { .. } => {
                // Implementation placeholder
            }
            TodoEvent::TodoCompletedV1 { .. } => {
                // Implementation placeholder
            }
            TodoEvent::TodoDeletedV1 { .. } => {
                // Implementation placeholder
            }
        }
    }
}

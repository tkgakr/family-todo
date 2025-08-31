use crate::domain::{
    error::{DomainError, DomainResult},
    events::TodoEvent,
    identifiers::{TodoId, UserId},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum TodoStatus {
    #[default]
    Active,
    Completed,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: TodoId,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub status: TodoStatus,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub version: u64,
}

impl Todo {
    pub fn new(
        id: TodoId,
        title: String,
        description: Option<String>,
        tags: Vec<String>,
        created_by: UserId,
    ) -> DomainResult<Self> {
        if title.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "Title cannot be empty".to_string(),
            ));
        }

        if title.len() > 200 {
            return Err(DomainError::ValidationError(
                "Title too long (max 200 characters)".to_string(),
            ));
        }

        if let Some(ref desc) = description {
            if desc.len() > 1000 {
                return Err(DomainError::ValidationError(
                    "Description too long (max 1000 characters)".to_string(),
                ));
            }
        }

        if tags.len() > 10 {
            return Err(DomainError::ValidationError(
                "Too many tags (max 10)".to_string(),
            ));
        }

        Ok(Self {
            id,
            title: title.trim().to_string(),
            description: description
                .map(|d| d.trim().to_string())
                .filter(|d| !d.is_empty()),
            tags,
            status: TodoStatus::Active,
            created_by,
            created_at: Utc::now(),
            updated_at: None,
            completed_at: None,
            version: 0,
        })
    }

    pub fn apply(&mut self, event: TodoEvent) {
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
                self.id = todo_id;
                self.title = title;
                self.description = description;
                self.tags = tags;
                self.created_by = created_by;
                self.created_at = timestamp;
                self.status = TodoStatus::Active;
            }
            TodoEvent::TodoUpdatedV1 {
                title,
                description,
                timestamp,
                ..
            } => {
                if let Some(new_title) = title {
                    self.title = new_title;
                }
                if let Some(new_description) = description {
                    self.description = Some(new_description);
                }
                self.updated_at = Some(timestamp);
            }
            TodoEvent::TodoCompletedV1 { timestamp, .. } => {
                self.status = TodoStatus::Completed;
                self.completed_at = Some(timestamp);
                self.updated_at = Some(timestamp);
            }
            TodoEvent::TodoDeletedV1 { timestamp, .. } => {
                self.status = TodoStatus::Deleted;
                self.updated_at = Some(timestamp);
            }
        }
        self.version += 1;
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, TodoStatus::Active)
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.status, TodoStatus::Completed)
    }

    pub fn is_deleted(&self) -> bool {
        matches!(self.status, TodoStatus::Deleted)
    }

    pub fn can_be_updated(&self) -> bool {
        self.is_active()
    }

    pub fn can_be_completed(&self) -> bool {
        self.is_active()
    }

    pub fn can_be_deleted(&self) -> bool {
        !self.is_deleted()
    }
}

impl Default for Todo {
    fn default() -> Self {
        Self {
            id: TodoId::default(),
            title: String::new(),
            description: None,
            tags: Vec::new(),
            status: TodoStatus::default(),
            created_by: UserId::default(),
            created_at: Utc::now(),
            updated_at: None,
            completed_at: None,
            version: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoUpdates {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoSnapshot {
    pub todo_id: TodoId,
    pub state: Todo,
    pub last_event_id: String,
    pub stream_version: u64,
    pub created_at: DateTime<Utc>,
}

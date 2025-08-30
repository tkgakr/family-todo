use anyhow::Result;
use std::collections::VecDeque;
use tracing::{error, info};

use crate::domain::{
    aggregates::{Todo, TodoSnapshot},
    events::TodoEvent,
    identifiers::{FamilyId, TodoId},
    error::DomainResult,
};
use crate::infra::DynamoDbRepository;

const SNAPSHOT_EVENT_THRESHOLD: usize = 100;
const SNAPSHOT_AGE_THRESHOLD_DAYS: i64 = 7;

pub struct EventStore {
    repository: DynamoDbRepository,
}

impl EventStore {
    pub fn new(table_name: String) -> Self {
        Self {
            repository: DynamoDbRepository::new(table_name),
        }
    }

    pub async fn append_event(
        &self,
        family_id: &FamilyId,
        event: TodoEvent,
    ) -> Result<()> {
        self.repository.save_event(family_id, &event).await
    }

    pub async fn get_events_after(
        &self,
        family_id: &FamilyId,
        todo_id: &TodoId,
        last_event_id: &str,
    ) -> Result<Vec<TodoEvent>> {
        let all_events = self.repository.get_events_for_todo(family_id, todo_id).await?;
        
        let mut found_last = false;
        let mut events_after = Vec::new();

        for event in all_events {
            if found_last {
                events_after.push(event);
            } else if event.event_id().as_str() == last_event_id {
                found_last = true;
            }
        }

        Ok(events_after)
    }

    pub async fn get_all_events(
        &self,
        family_id: &FamilyId,
        todo_id: &TodoId,
    ) -> Result<Vec<TodoEvent>> {
        self.repository.get_events_for_todo(family_id, todo_id).await
    }

    pub async fn rebuild_with_snapshot(
        &self,
        family_id: &FamilyId,
        todo_id: &TodoId,
    ) -> DomainResult<Todo> {
        let snapshot = self.repository.get_latest_snapshot(family_id, todo_id).await
            .map_err(|e| crate::domain::error::DomainError::ValidationError(e.to_string()))?;
        
        let events = if let Some(ref snap) = snapshot {
            self.get_events_after(family_id, todo_id, &snap.last_event_id).await
                .map_err(|e| crate::domain::error::DomainError::ValidationError(e.to_string()))?
        } else {
            self.get_all_events(family_id, todo_id).await
                .map_err(|e| crate::domain::error::DomainError::ValidationError(e.to_string()))?
        };

        let mut todo = snapshot.map(|s| s.state).unwrap_or_default();
        let event_count = events.len();

        for event in events {
            todo.apply(event.upcast());
        }

        if event_count >= SNAPSHOT_EVENT_THRESHOLD {
            let family_id_clone = family_id.clone();
            let todo_id_clone = todo_id.clone();
            
            tokio::spawn(async move {
                let event_store = EventStore::new(std::env::var("TABLE_NAME").unwrap_or_else(|_| "MainTable".to_string()));
                if let Err(e) = event_store.create_snapshot_if_needed(&family_id_clone, &todo_id_clone, event_count, None).await {
                    error!(error = %e, "Failed to create snapshot");
                }
            });
        }

        Ok(todo)
    }

    pub async fn create_snapshot_if_needed(
        &self,
        family_id: &FamilyId,
        todo_id: &TodoId,
        event_count: usize,
        last_snapshot_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        let should_create_snapshot = event_count >= SNAPSHOT_EVENT_THRESHOLD ||
            last_snapshot_date.map_or(true, |date| {
                chrono::Utc::now().signed_duration_since(date).num_days() >= SNAPSHOT_AGE_THRESHOLD_DAYS
            });

        if should_create_snapshot {
            let snapshot = self.build_snapshot(family_id, todo_id).await?;
            self.repository.save_snapshot(family_id, &snapshot).await?;
            
            info!(
                todo_id = todo_id.as_str(),
                family_id = family_id.as_str(),
                "Snapshot created successfully"
            );
        }

        Ok(())
    }

    async fn build_snapshot(
        &self,
        family_id: &FamilyId,
        todo_id: &TodoId,
    ) -> Result<TodoSnapshot> {
        let events = self.get_all_events(family_id, todo_id).await?;
        
        if events.is_empty() {
            return Err(anyhow::anyhow!("No events found for todo"));
        }

        let mut todo = Todo::default();
        let mut last_event_id = String::new();
        let mut stream_version = 0;

        for (index, event) in events.iter().enumerate() {
            todo.apply(event.clone());
            last_event_id = event.event_id().as_str().to_string();
            stream_version = index as u64 + 1;
        }

        Ok(TodoSnapshot {
            todo_id: todo_id.clone(),
            state: todo,
            last_event_id,
            stream_version,
            created_at: chrono::Utc::now(),
        })
    }
}

pub struct EventProcessor {
    repository: DynamoDbRepository,
}

impl EventProcessor {
    pub fn new(table_name: String) -> Self {
        Self {
            repository: DynamoDbRepository::new(table_name),
        }
    }

    pub async fn process_event(
        &self,
        family_id: &FamilyId,
        event: TodoEvent,
    ) -> Result<()> {
        let todo_id = event.todo_id();
        
        let current_todo = self.repository.get_todo(family_id, todo_id).await
            .unwrap_or_else(|_| Todo::default());
        
        let mut updated_todo = current_todo.clone();
        updated_todo.apply(event);
        
        self.repository.save_todo_projection(family_id, &updated_todo).await?;
        
        info!(
            todo_id = todo_id.as_str(),
            family_id = family_id.as_str(),
            "Event processed and projection updated"
        );

        Ok(())
    }
}
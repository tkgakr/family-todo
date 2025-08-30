use crate::DynamoDbClient;
use domain::{TodoError, TodoEvent, TodoId};

pub struct EventRepository {
    #[allow(dead_code)]
    db: DynamoDbClient,
}

impl EventRepository {
    pub fn new(db: DynamoDbClient) -> Self {
        Self { db }
    }

    pub async fn save_event(&self, _family_id: &str, _event: TodoEvent) -> Result<(), TodoError> {
        // Implementation placeholder for task 3.2
        Ok(())
    }

    pub async fn get_events(
        &self,
        _family_id: &str,
        _todo_id: &TodoId,
    ) -> Result<Vec<TodoEvent>, TodoError> {
        // Implementation placeholder for task 3.2
        Ok(vec![])
    }
}

pub struct ProjectionRepository {
    #[allow(dead_code)]
    db: DynamoDbClient,
}

impl ProjectionRepository {
    pub fn new(db: DynamoDbClient) -> Self {
        Self { db }
    }

    // Implementation placeholder for task 3.2
}

pub struct SnapshotRepository {
    #[allow(dead_code)]
    db: DynamoDbClient,
}

impl SnapshotRepository {
    pub fn new(db: DynamoDbClient) -> Self {
        Self { db }
    }

    // Implementation placeholder for task 3.2
}

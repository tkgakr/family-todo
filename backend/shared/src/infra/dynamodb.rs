use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::{
    types::{AttributeValue, ReturnValue},
    Client as DynamoDbClient,
};
use once_cell::sync::Lazy;
use serde_json;
use std::collections::HashMap;
use tracing::{error, info};

use crate::domain::{
    aggregates::{Todo, TodoSnapshot, TodoUpdates},
    error::{DomainError, DomainResult, UpdateError},
    events::TodoEvent,
    identifiers::{FamilyId, TodoId},
};

static DYNAMODB_CLIENT: Lazy<DynamoDbClient> = Lazy::new(|| {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let config = aws_config::defaults(BehaviorVersion::latest()).load().await;
        DynamoDbClient::new(&config)
    })
});

pub struct DynamoDbRepository {
    client: &'static DynamoDbClient,
    table_name: String,
}

impl DynamoDbRepository {
    pub fn new(table_name: String) -> Self {
        Self {
            client: &*DYNAMODB_CLIENT,
            table_name,
        }
    }

    pub async fn save_event(&self, family_id: &FamilyId, event: &TodoEvent) -> Result<()> {
        let event_id = event.event_id().as_str();
        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk = format!("EVENT#{event_id}");

        let event_json = serde_json::to_string(event)?;

        let request = self
            .client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(pk))
            .item("SK", AttributeValue::S(sk))
            .item("EntityType", AttributeValue::S("Event".to_string()))
            .item("Data", AttributeValue::S(event_json))
            .item(
                "CreatedAt",
                AttributeValue::S(chrono::Utc::now().to_rfc3339()),
            )
            .item(
                "TTL",
                AttributeValue::N((chrono::Utc::now().timestamp() + 86400 * 365).to_string()),
            );

        request.send().await?;

        info!(
            event_id = event_id,
            family_id = family_id.as_str(),
            "Event saved successfully"
        );

        Ok(())
    }

    pub async fn get_events_for_todo(
        &self,
        family_id: &FamilyId,
        todo_id: &TodoId,
    ) -> Result<Vec<TodoEvent>> {
        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk_prefix = format!("TODO#EVENT#{}#", todo_id.as_str());

        let query_output = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .expression_attribute_values(":pk", AttributeValue::S(pk))
            .expression_attribute_values(":sk_prefix", AttributeValue::S(sk_prefix))
            .send()
            .await?;

        let mut events = Vec::new();

        if let Some(items) = query_output.items {
            for item in items {
                if let Some(AttributeValue::S(event_data)) = item.get("Data") {
                    match serde_json::from_str::<TodoEvent>(event_data) {
                        Ok(event) => events.push(event),
                        Err(e) => {
                            error!(error = %e, "Failed to deserialize event");
                        }
                    }
                }
            }
        }

        events.sort_by(|a, b| a.timestamp().cmp(b.timestamp()));
        Ok(events)
    }

    pub async fn save_todo_projection(&self, family_id: &FamilyId, todo: &Todo) -> Result<()> {
        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk = format!("TODO#CURRENT#{}", todo.id.as_str());

        let todo_json = serde_json::to_string(todo)?;

        let mut request = self
            .client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(pk))
            .item("SK", AttributeValue::S(sk))
            .item("EntityType", AttributeValue::S("Projection".to_string()))
            .item("Data", AttributeValue::S(todo_json))
            .item("Version", AttributeValue::N(todo.version.to_string()))
            .item(
                "UpdatedAt",
                AttributeValue::S(chrono::Utc::now().to_rfc3339()),
            );

        if todo.is_active() {
            let gsi1_pk = format!("FAMILY#{}#ACTIVE", family_id.as_str());
            let gsi1_sk = todo.id.as_str().to_string();

            request = request
                .item("GSI1PK", AttributeValue::S(gsi1_pk))
                .item("GSI1SK", AttributeValue::S(gsi1_sk));
        }

        request.send().await?;

        info!(
            todo_id = todo.id.as_str(),
            family_id = family_id.as_str(),
            version = todo.version,
            "Todo projection saved successfully"
        );

        Ok(())
    }

    pub async fn get_todo(&self, family_id: &FamilyId, todo_id: &TodoId) -> DomainResult<Todo> {
        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk = format!("TODO#CURRENT#{}", todo_id.as_str());

        let get_item_output = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk))
            .send()
            .await
            .map_err(|e| DomainError::ValidationError(e.to_string()))?;

        if let Some(item) = get_item_output.item {
            if let Some(AttributeValue::S(todo_data)) = item.get("Data") {
                let todo: Todo = serde_json::from_str(todo_data)
                    .map_err(|e| DomainError::ValidationError(e.to_string()))?;
                return Ok(todo);
            }
        }

        Err(DomainError::TodoNotFound(todo_id.as_str().to_string()))
    }

    pub async fn get_active_todos(
        &self,
        family_id: &FamilyId,
        limit: Option<i32>,
    ) -> Result<Vec<Todo>> {
        let gsi1_pk = format!("FAMILY#{}#ACTIVE", family_id.as_str());

        let mut query = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("GSI1")
            .key_condition_expression("GSI1PK = :gsi1_pk")
            .expression_attribute_values(":gsi1_pk", AttributeValue::S(gsi1_pk));

        if let Some(limit_value) = limit {
            query = query.limit(limit_value);
        }

        let query_output = query.send().await?;

        let mut todos = Vec::new();

        if let Some(items) = query_output.items {
            for item in items {
                if let Some(AttributeValue::S(todo_data)) = item.get("Data") {
                    match serde_json::from_str::<Todo>(todo_data) {
                        Ok(todo) => todos.push(todo),
                        Err(e) => {
                            error!(error = %e, "Failed to deserialize todo");
                        }
                    }
                }
            }
        }

        Ok(todos)
    }

    pub async fn update_todo_with_lock(
        &self,
        family_id: &FamilyId,
        todo: &Todo,
        updates: TodoUpdates,
    ) -> Result<Todo, UpdateError> {
        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk = format!("TODO#CURRENT#{}", todo.id.as_str());

        let update_expression = self.build_update_expression(&updates);
        let expression_attribute_values =
            self.build_expression_attribute_values(&updates, todo.version);

        let result = self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk))
            .update_expression(update_expression)
            .condition_expression("attribute_exists(PK) AND Version = :current_version")
            .set_expression_attribute_values(Some(expression_attribute_values))
            .return_values(ReturnValue::AllNew)
            .send()
            .await;

        match result {
            Ok(output) => {
                if let Some(item) = output.attributes {
                    if let Some(AttributeValue::S(todo_data)) = item.get("Data") {
                        let updated_todo: Todo = serde_json::from_str(todo_data)
                            .map_err(|e| UpdateError::DynamoDb(e.to_string()))?;
                        return Ok(updated_todo);
                    }
                }
                Err(UpdateError::NotFound)
            }
            Err(sdk_error) => {
                if sdk_error
                    .to_string()
                    .contains("ConditionalCheckFailedException")
                {
                    Err(UpdateError::ConcurrentModification)
                } else {
                    Err(UpdateError::DynamoDb(sdk_error.to_string()))
                }
            }
        }
    }

    pub async fn save_snapshot(&self, family_id: &FamilyId, snapshot: &TodoSnapshot) -> Result<()> {
        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk = format!(
            "TODO#SNAPSHOT#{}#{}",
            snapshot.todo_id.as_str(),
            snapshot.last_event_id
        );

        let snapshot_json = serde_json::to_string(snapshot)?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(pk))
            .item("SK", AttributeValue::S(sk))
            .item("EntityType", AttributeValue::S("Snapshot".to_string()))
            .item("Data", AttributeValue::S(snapshot_json))
            .item(
                "CreatedAt",
                AttributeValue::S(chrono::Utc::now().to_rfc3339()),
            )
            .item(
                "TTL",
                AttributeValue::N((chrono::Utc::now().timestamp() + 86400 * 30).to_string()),
            )
            .send()
            .await?;

        Ok(())
    }

    pub async fn get_latest_snapshot(
        &self,
        family_id: &FamilyId,
        todo_id: &TodoId,
    ) -> Result<Option<TodoSnapshot>> {
        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk_prefix = format!("TODO#SNAPSHOT#{}#", todo_id.as_str());

        let query_output = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .expression_attribute_values(":pk", AttributeValue::S(pk))
            .expression_attribute_values(":sk_prefix", AttributeValue::S(sk_prefix))
            .scan_index_forward(false)
            .limit(1)
            .send()
            .await?;

        if let Some(items) = query_output.items {
            if let Some(item) = items.into_iter().next() {
                if let Some(AttributeValue::S(snapshot_data)) = item.get("Data") {
                    let snapshot: TodoSnapshot = serde_json::from_str(snapshot_data)?;
                    return Ok(Some(snapshot));
                }
            }
        }

        Ok(None)
    }

    pub async fn rebuild_todo_from_snapshot(
        &self,
        family_id: &FamilyId,
        todo_id: &TodoId,
    ) -> DomainResult<Todo> {
        if let Some(snapshot) = self
            .get_latest_snapshot(family_id, todo_id)
            .await
            .map_err(|e| DomainError::ValidationError(e.to_string()))?
        {
            let events_after_snapshot = self
                .get_events_after_snapshot(family_id, todo_id, &snapshot.last_event_id)
                .await
                .map_err(|e| DomainError::ValidationError(e.to_string()))?;

            let mut todo = snapshot.state;
            for event in events_after_snapshot {
                todo.apply(event);
            }

            Ok(todo)
        } else {
            let events = self
                .get_events_for_todo(family_id, todo_id)
                .await
                .map_err(|e| DomainError::ValidationError(e.to_string()))?;

            if events.is_empty() {
                return Err(DomainError::TodoNotFound(todo_id.as_str().to_string()));
            }

            let mut todo = Todo::default();
            for event in events {
                todo.apply(event);
            }

            Ok(todo)
        }
    }

    async fn get_events_after_snapshot(
        &self,
        family_id: &FamilyId,
        todo_id: &TodoId,
        last_event_id: &str,
    ) -> Result<Vec<TodoEvent>> {
        let pk = format!("FAMILY#{}", family_id.as_str());
        let sk_prefix = format!("TODO#EVENT#{}#", todo_id.as_str());

        let query_output = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .expression_attribute_values(":pk", AttributeValue::S(pk))
            .expression_attribute_values(":sk_prefix", AttributeValue::S(sk_prefix))
            .send()
            .await?;

        let mut events = Vec::new();
        let mut found_last_event = false;

        if let Some(items) = query_output.items {
            for item in items {
                if let Some(AttributeValue::S(event_data)) = item.get("Data") {
                    match serde_json::from_str::<TodoEvent>(event_data) {
                        Ok(event) => {
                            if event.event_id().as_str() == last_event_id {
                                found_last_event = true;
                                continue;
                            }
                            if found_last_event {
                                events.push(event);
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to deserialize event");
                        }
                    }
                }
            }
        }

        events.sort_by(|a, b| a.timestamp().cmp(b.timestamp()));
        Ok(events)
    }

    fn build_update_expression(&self, updates: &TodoUpdates) -> String {
        let mut set_clauses = vec!["Version = Version + :inc".to_string()];
        set_clauses.push("UpdatedAt = :updated_at".to_string());

        if updates.title.is_some() {
            set_clauses.push("Data.title = :title".to_string());
        }

        if updates.description.is_some() {
            set_clauses.push("Data.description = :description".to_string());
        }

        if updates.tags.is_some() {
            set_clauses.push("Data.tags = :tags".to_string());
        }

        format!("SET {}", set_clauses.join(", "))
    }

    fn build_expression_attribute_values(
        &self,
        updates: &TodoUpdates,
        current_version: u64,
    ) -> HashMap<String, AttributeValue> {
        let mut values = HashMap::new();

        values.insert(
            ":current_version".to_string(),
            AttributeValue::N(current_version.to_string()),
        );
        values.insert(":inc".to_string(), AttributeValue::N("1".to_string()));
        values.insert(
            ":updated_at".to_string(),
            AttributeValue::S(chrono::Utc::now().to_rfc3339()),
        );

        if let Some(ref title) = updates.title {
            values.insert(":title".to_string(), AttributeValue::S(title.clone()));
        }

        if let Some(ref description) = updates.description {
            values.insert(
                ":description".to_string(),
                AttributeValue::S(description.clone()),
            );
        }

        if let Some(ref tags) = updates.tags {
            let tag_list: Vec<AttributeValue> = tags
                .iter()
                .map(|tag| AttributeValue::S(tag.clone()))
                .collect();
            values.insert(":tags".to_string(), AttributeValue::L(tag_list));
        }

        values
    }
}

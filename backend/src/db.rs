use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;

use crate::error::ApiError;
use crate::models::Todo;

#[derive(Clone)]
pub struct DynamoClient {
    client: Client,
    table_name: String,
}

impl DynamoClient {
    pub async fn new(table_name: &str) -> Self {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);
        Self {
            client,
            table_name: table_name.to_string(),
        }
    }

    pub async fn list_todos(&self, family_id: &str) -> Result<Vec<Todo>, ApiError> {
        let pk = format!("FAMILY#{family_id}");

        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .expression_attribute_values(":pk", AttributeValue::S(pk))
            .expression_attribute_values(":sk_prefix", AttributeValue::S("TODO#".to_string()))
            .send()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let todos = result.items().iter().filter_map(item_to_todo).collect();

        Ok(todos)
    }

    pub async fn put_todo(&self, family_id: &str, todo: &Todo) -> Result<(), ApiError> {
        let pk = format!("FAMILY#{family_id}");
        let sk = format!("TODO#{}", todo.id);

        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(pk))
            .item("SK", AttributeValue::S(sk))
            .item("id", AttributeValue::S(todo.id.clone()))
            .item("title", AttributeValue::S(todo.title.clone()))
            .item("completed", AttributeValue::Bool(todo.completed))
            .item("created_by", AttributeValue::S(todo.created_by.clone()))
            .item("created_at", AttributeValue::S(todo.created_at.clone()))
            .item("updated_at", AttributeValue::S(todo.updated_at.clone()))
            .send()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(())
    }

    pub async fn update_todo(
        &self,
        family_id: &str,
        todo_id: &str,
        title: Option<&str>,
        completed: Option<bool>,
    ) -> Result<Todo, ApiError> {
        let pk = format!("FAMILY#{family_id}");
        let sk = format!("TODO#{todo_id}");
        let now = chrono::Utc::now().to_rfc3339();

        let mut update_parts = vec!["updated_at = :updated_at"];
        let mut builder = self
            .client
            .update_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk))
            .expression_attribute_values(":updated_at", AttributeValue::S(now))
            .return_values(aws_sdk_dynamodb::types::ReturnValue::AllNew);

        if let Some(t) = title {
            update_parts.push("title = :title");
            builder =
                builder.expression_attribute_values(":title", AttributeValue::S(t.to_string()));
        }

        if let Some(c) = completed {
            update_parts.push("completed = :completed");
            builder = builder.expression_attribute_values(":completed", AttributeValue::Bool(c));
        }

        let expression = format!("SET {}", update_parts.join(", "));
        builder = builder.update_expression(expression);

        let result = builder
            .send()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let item = result.attributes().ok_or(ApiError::NotFound)?;
        item_to_todo(item).ok_or(ApiError::Internal(
            "Failed to parse updated item".to_string(),
        ))
    }

    pub async fn delete_todo(&self, family_id: &str, todo_id: &str) -> Result<(), ApiError> {
        let pk = format!("FAMILY#{family_id}");
        let sk = format!("TODO#{todo_id}");

        self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk))
            .send()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(())
    }
}

fn item_to_todo(item: &std::collections::HashMap<String, AttributeValue>) -> Option<Todo> {
    Some(Todo {
        id: item.get("id")?.as_s().ok()?.clone(),
        title: item.get("title")?.as_s().ok()?.clone(),
        completed: *item.get("completed")?.as_bool().ok()?,
        created_by: item.get("created_by")?.as_s().ok()?.clone(),
        created_at: item.get("created_at")?.as_s().ok()?.clone(),
        updated_at: item.get("updated_at")?.as_s().ok()?.clone(),
    })
}

use anyhow::Result;
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use chrono::Utc;
use shared::{
    domain::{
        aggregates::{Todo, TodoUpdates},
        events::TodoEvent,
        identifiers::{TodoId, UserId, FamilyId, EventId},
        error::{DomainError, UpdateError},
    },
    infra::EventStore,
};
use tracing::{info, warn, error, instrument};

use crate::commands::{CreateTodoCommand, UpdateTodoCommand, CompleteTodoCommand, DeleteTodoCommand};
use crate::responses::ApiResponse;

pub struct CommandHandler {
    event_store: EventStore,
}

impl CommandHandler {
    pub fn new(table_name: String) -> Self {
        Self {
            event_store: EventStore::new(table_name),
        }
    }

    #[instrument(skip(self))]
    pub async fn handle_request(
        &self,
        request: ApiGatewayProxyRequest,
    ) -> Result<ApiGatewayProxyResponse> {
        match request.http_method.as_str() {
            "OPTIONS" => Ok(self.handle_preflight()),
            "POST" => self.handle_post(request).await,
            "PUT" => self.handle_put(request).await,
            "DELETE" => self.handle_delete(request).await,
            _ => Ok(ApiResponse::bad_request("Method not allowed")),
        }
    }

    fn handle_preflight(&self) -> ApiGatewayProxyResponse {
        ApiResponse::no_content()
    }

    async fn handle_post(&self, request: ApiGatewayProxyRequest) -> Result<ApiGatewayProxyResponse> {
        let path = &request.path;
        
        if path == "/todos" {
            self.create_todo(request).await
        } else if let Some(todo_id) = self.extract_todo_id_from_complete_path(path) {
            self.complete_todo(todo_id, request).await
        } else {
            Ok(ApiResponse::not_found("Endpoint not found"))
        }
    }

    async fn handle_put(&self, request: ApiGatewayProxyRequest) -> Result<ApiGatewayProxyResponse> {
        if let Some(todo_id) = self.extract_todo_id_from_path(&request.path) {
            self.update_todo(todo_id, request).await
        } else {
            Ok(ApiResponse::not_found("Todo not found"))
        }
    }

    async fn handle_delete(&self, request: ApiGatewayProxyRequest) -> Result<ApiGatewayProxyResponse> {
        if let Some(todo_id) = self.extract_todo_id_from_path(&request.path) {
            self.delete_todo(todo_id, request).await
        } else {
            Ok(ApiResponse::not_found("Todo not found"))
        }
    }

    #[instrument(skip(self, request))]
    async fn create_todo(&self, request: ApiGatewayProxyRequest) -> Result<ApiGatewayProxyResponse> {
        let body = match request.body.as_ref() {
            Some(body) => body,
            None => return Ok(ApiResponse::bad_request("Request body is required")),
        };

        let command: CreateTodoCommand = match serde_json::from_str(body) {
            Ok(cmd) => cmd,
            Err(_) => return Ok(ApiResponse::bad_request("Invalid JSON format")),
        };

        if let Err(validation_error) = self.validate_create_command(&command) {
            return Ok(ApiResponse::bad_request(&validation_error));
        }

        let family_id = self.extract_family_id(&request)?;
        let user_id = self.extract_user_id(&request)?;
        let todo_id = TodoId::new();
        let event_id = EventId::new();

        let event = TodoEvent::TodoCreatedV2 {
            event_id,
            todo_id: todo_id.clone(),
            title: command.title.trim().to_string(),
            description: command.description.map(|d| d.trim().to_string()).filter(|d| !d.is_empty()),
            tags: command.tags.unwrap_or_default(),
            created_by: user_id,
            timestamp: Utc::now(),
        };

        match self.event_store.append_event(&family_id, event.clone()).await {
            Ok(_) => {
                info!(
                    todo_id = todo_id.as_str(),
                    family_id = family_id.as_str(),
                    "Todo creation event saved successfully"
                );

                let mut todo = Todo::default();
                todo.apply(event);

                Ok(ApiResponse::created(todo))
            }
            Err(e) => {
                error!(error = %e, "Failed to save todo creation event");
                Ok(ApiResponse::internal_server_error("Failed to create todo"))
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn update_todo(
        &self,
        todo_id: TodoId,
        request: ApiGatewayProxyRequest,
    ) -> Result<ApiGatewayProxyResponse> {
        let body = match request.body.as_ref() {
            Some(body) => body,
            None => return Ok(ApiResponse::bad_request("Request body is required")),
        };

        let command: UpdateTodoCommand = match serde_json::from_str(body) {
            Ok(mut cmd) => {
                cmd.todo_id = todo_id.clone();
                cmd
            }
            Err(_) => return Ok(ApiResponse::bad_request("Invalid JSON format")),
        };

        if let Err(validation_error) = self.validate_update_command(&command) {
            return Ok(ApiResponse::bad_request(&validation_error));
        }

        let family_id = self.extract_family_id(&request)?;
        let user_id = self.extract_user_id(&request)?;
        let event_id = EventId::new();

        let event = TodoEvent::TodoUpdatedV1 {
            event_id,
            todo_id: todo_id.clone(),
            title: command.title.map(|t| t.trim().to_string()),
            description: command.description.map(|d| d.trim().to_string()),
            updated_by: user_id,
            timestamp: Utc::now(),
        };

        match self.event_store.append_event(&family_id, event.clone()).await {
            Ok(_) => {
                info!(
                    todo_id = todo_id.as_str(),
                    family_id = family_id.as_str(),
                    "Todo update event saved successfully"
                );

                match self.event_store.rebuild_with_snapshot(&family_id, &todo_id).await {
                    Ok(todo) => Ok(ApiResponse::ok(todo)),
                    Err(DomainError::TodoNotFound(_)) => Ok(ApiResponse::not_found("Todo not found")),
                    Err(e) => {
                        error!(error = %e, "Failed to rebuild todo from events");
                        Ok(ApiResponse::internal_server_error("Failed to update todo"))
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to save todo update event");
                Ok(ApiResponse::internal_server_error("Failed to update todo"))
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn complete_todo(
        &self,
        todo_id: TodoId,
        request: ApiGatewayProxyRequest,
    ) -> Result<ApiGatewayProxyResponse> {
        let family_id = self.extract_family_id(&request)?;
        let user_id = self.extract_user_id(&request)?;
        let event_id = EventId::new();

        let event = TodoEvent::TodoCompletedV1 {
            event_id,
            todo_id: todo_id.clone(),
            completed_by: user_id,
            timestamp: Utc::now(),
        };

        match self.event_store.append_event(&family_id, event.clone()).await {
            Ok(_) => {
                info!(
                    todo_id = todo_id.as_str(),
                    family_id = family_id.as_str(),
                    "Todo completion event saved successfully"
                );

                match self.event_store.rebuild_with_snapshot(&family_id, &todo_id).await {
                    Ok(todo) => Ok(ApiResponse::ok(todo)),
                    Err(DomainError::TodoNotFound(_)) => Ok(ApiResponse::not_found("Todo not found")),
                    Err(e) => {
                        error!(error = %e, "Failed to rebuild todo from events");
                        Ok(ApiResponse::internal_server_error("Failed to complete todo"))
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to save todo completion event");
                Ok(ApiResponse::internal_server_error("Failed to complete todo"))
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn delete_todo(
        &self,
        todo_id: TodoId,
        request: ApiGatewayProxyRequest,
    ) -> Result<ApiGatewayProxyResponse> {
        let family_id = self.extract_family_id(&request)?;
        let user_id = self.extract_user_id(&request)?;
        let event_id = EventId::new();

        let reason = request.body
            .as_ref()
            .and_then(|body| serde_json::from_str::<DeleteTodoCommand>(body).ok())
            .and_then(|cmd| cmd.reason);

        let event = TodoEvent::TodoDeletedV1 {
            event_id,
            todo_id: todo_id.clone(),
            deleted_by: user_id,
            reason,
            timestamp: Utc::now(),
        };

        match self.event_store.append_event(&family_id, event.clone()).await {
            Ok(_) => {
                info!(
                    todo_id = todo_id.as_str(),
                    family_id = family_id.as_str(),
                    "Todo deletion event saved successfully"
                );
                Ok(ApiResponse::no_content())
            }
            Err(e) => {
                error!(error = %e, "Failed to save todo deletion event");
                Ok(ApiResponse::internal_server_error("Failed to delete todo"))
            }
        }
    }

    fn extract_todo_id_from_path(&self, path: &str) -> Option<TodoId> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if parts.len() == 2 && parts[0] == "todos" {
            TodoId::from_string(parts[1].to_string()).ok()
        } else {
            None
        }
    }

    fn extract_todo_id_from_complete_path(&self, path: &str) -> Option<TodoId> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if parts.len() == 3 && parts[0] == "todos" && parts[2] == "complete" {
            TodoId::from_string(parts[1].to_string()).ok()
        } else {
            None
        }
    }

    fn extract_family_id(&self, request: &ApiGatewayProxyRequest) -> Result<FamilyId> {
        let family_id_str = request
            .headers
            .get("x-family-id")
            .or_else(|| request.query_string_parameters.get("family_id"))
            .ok_or_else(|| anyhow::anyhow!("Family ID is required"))?;

        FamilyId::from_string(family_id_str.clone())
            .map_err(|_| anyhow::anyhow!("Invalid family ID"))
    }

    fn extract_user_id(&self, request: &ApiGatewayProxyRequest) -> Result<UserId> {
        let user_id_str = request
            .request_context
            .authorizer
            .get("sub")
            .or_else(|| request.headers.get("x-user-id"))
            .ok_or_else(|| anyhow::anyhow!("User ID is required"))?;

        UserId::from_string(user_id_str.clone())
            .map_err(|_| anyhow::anyhow!("Invalid user ID"))
    }

    fn validate_create_command(&self, command: &CreateTodoCommand) -> Result<(), String> {
        if command.title.trim().is_empty() {
            return Err("Title cannot be empty".to_string());
        }
        if command.title.len() > 200 {
            return Err("Title too long (max 200 characters)".to_string());
        }
        if let Some(ref desc) = command.description {
            if desc.len() > 1000 {
                return Err("Description too long (max 1000 characters)".to_string());
            }
        }
        if let Some(ref tags) = command.tags {
            if tags.len() > 10 {
                return Err("Too many tags (max 10)".to_string());
            }
        }
        Ok(())
    }

    fn validate_update_command(&self, command: &UpdateTodoCommand) -> Result<(), String> {
        if let Some(ref title) = command.title {
            if title.trim().is_empty() {
                return Err("Title cannot be empty".to_string());
            }
            if title.len() > 200 {
                return Err("Title too long (max 200 characters)".to_string());
            }
        }
        if let Some(ref desc) = command.description {
            if desc.len() > 1000 {
                return Err("Description too long (max 1000 characters)".to_string());
            }
        }
        if let Some(ref tags) = command.tags {
            if tags.len() > 10 {
                return Err("Too many tags (max 10)".to_string());
            }
        }
        Ok(())
    }
}
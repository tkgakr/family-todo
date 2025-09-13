use anyhow::Result;
use aws_lambda_events::event::apigw::{ApiGatewayProxyRequest, ApiGatewayProxyResponse};
use shared::{
    domain::{
        aggregates::TodoStatus,
        error::DomainError,
        identifiers::{FamilyId, TodoId},
    },
    infra::{DynamoDbRepository, EventStore},
};
use tracing::{error, info, instrument};

use crate::queries::{
    FamilyMember, FamilyMembersResponse, GetTodosQuery, TodoHistoryResponse, TodoListResponse,
};
use crate::responses::ApiResponse;

pub struct QueryHandler {
    repository: DynamoDbRepository,
    event_store: EventStore,
}

impl QueryHandler {
    pub async fn new(table_name: String) -> Self {
        Self {
            repository: DynamoDbRepository::new(table_name.clone()).await,
            event_store: EventStore::new(table_name).await,
        }
    }

    #[instrument(skip(self))]
    pub async fn handle_request(
        &self,
        request: ApiGatewayProxyRequest,
    ) -> Result<ApiGatewayProxyResponse> {
        match request.http_method.as_str() {
            "GET" => self.handle_get(request).await,
            _ => Ok(ApiResponse::bad_request("Method not allowed")),
        }
    }

    async fn handle_get(&self, request: ApiGatewayProxyRequest) -> Result<ApiGatewayProxyResponse> {
        let path = request.path.as_deref().unwrap_or("");

        if path == "/todos" {
            self.get_todos(request).await
        } else if let Some(todo_id) = self.extract_todo_id_from_path(path) {
            self.get_todo(todo_id, request).await
        } else if let Some(todo_id) = self.extract_todo_id_from_history_path(path) {
            self.get_todo_history(todo_id, request).await
        } else if path == "/family/members" {
            self.get_family_members(request).await
        } else {
            Ok(ApiResponse::not_found("Endpoint not found"))
        }
    }

    #[instrument(skip(self, request))]
    async fn get_todos(&self, request: ApiGatewayProxyRequest) -> Result<ApiGatewayProxyResponse> {
        let family_id = match self.extract_family_id(&request) {
            Ok(id) => id,
            Err(e) => return Ok(ApiResponse::bad_request(&e.to_string())),
        };

        let query = self.parse_todos_query(&request);

        match query.status.as_ref().unwrap_or(&TodoStatus::Active) {
            TodoStatus::Active => {
                match self
                    .repository
                    .get_active_todos(&family_id, query.limit.map(|l| l as i32))
                    .await
                {
                    Ok(todos) => {
                        let response = TodoListResponse {
                            has_more: todos.len() == query.limit.unwrap_or(50) as usize,
                            total_count: None,
                            todos,
                        };
                        Ok(ApiResponse::ok(response))
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to get active todos");
                        Ok(ApiResponse::internal_server_error("Failed to get todos"))
                    }
                }
            }
            _ => {
                // For other statuses, we would need to implement additional repository methods
                // For now, return empty list
                let response = TodoListResponse {
                    todos: vec![],
                    has_more: false,
                    total_count: Some(0),
                };
                Ok(ApiResponse::ok(response))
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn get_todo(
        &self,
        todo_id: TodoId,
        request: ApiGatewayProxyRequest,
    ) -> Result<ApiGatewayProxyResponse> {
        let family_id = match self.extract_family_id(&request) {
            Ok(id) => id,
            Err(e) => return Ok(ApiResponse::bad_request(&e.to_string())),
        };

        match self.repository.get_todo(&family_id, &todo_id).await {
            Ok(todo) => {
                info!(
                    todo_id = todo_id.as_str(),
                    family_id = family_id.as_str(),
                    "Todo retrieved successfully"
                );
                Ok(ApiResponse::ok(todo))
            }
            Err(DomainError::TodoNotFound(_)) => Ok(ApiResponse::not_found("Todo not found")),
            Err(e) => {
                error!(error = %e, "Failed to get todo");
                Ok(ApiResponse::internal_server_error("Failed to get todo"))
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn get_todo_history(
        &self,
        todo_id: TodoId,
        request: ApiGatewayProxyRequest,
    ) -> Result<ApiGatewayProxyResponse> {
        let family_id = match self.extract_family_id(&request) {
            Ok(id) => id,
            Err(e) => return Ok(ApiResponse::bad_request(&e.to_string())),
        };

        let limit = 100u32;

        match self.event_store.get_all_events(&family_id, &todo_id).await {
            Ok(mut events) => {
                // Apply limit if specified
                if limit < events.len() as u32 {
                    events.truncate(limit as usize);
                }

                let response = TodoHistoryResponse {
                    todo_id: todo_id.clone(),
                    total_count: events.len() as u32,
                    events,
                };

                info!(
                    todo_id = todo_id.as_str(),
                    family_id = family_id.as_str(),
                    event_count = response.total_count,
                    "Todo history retrieved successfully"
                );

                Ok(ApiResponse::ok(response))
            }
            Err(e) => {
                error!(error = %e, "Failed to get todo history");
                Ok(ApiResponse::internal_server_error(
                    "Failed to get todo history",
                ))
            }
        }
    }

    #[instrument(skip(self, request))]
    async fn get_family_members(
        &self,
        request: ApiGatewayProxyRequest,
    ) -> Result<ApiGatewayProxyResponse> {
        let _family_id = match self.extract_family_id(&request) {
            Ok(id) => id,
            Err(e) => return Ok(ApiResponse::bad_request(&e.to_string())),
        };

        // For now, return mock data since we haven't implemented family member management
        let mock_members = vec![FamilyMember {
            user_id: "user1".to_string(),
            display_name: "家族メンバー1".to_string(),
            avatar_url: None,
            role: "admin".to_string(),
            joined_at: chrono::Utc::now(),
        }];

        let response = FamilyMembersResponse {
            total_count: mock_members.len() as u32,
            members: mock_members,
        };

        Ok(ApiResponse::ok(response))
    }

    fn extract_todo_id_from_path(&self, path: &str) -> Option<TodoId> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if parts.len() == 2 && parts[0] == "todos" {
            TodoId::from_string(parts[1].to_string()).ok()
        } else {
            None
        }
    }

    fn extract_todo_id_from_history_path(&self, path: &str) -> Option<TodoId> {
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if parts.len() == 3 && parts[0] == "todos" && parts[2] == "history" {
            TodoId::from_string(parts[1].to_string()).ok()
        } else {
            None
        }
    }

    fn extract_family_id(&self, request: &ApiGatewayProxyRequest) -> Result<FamilyId> {
        let family_id_str = request
            .headers
            .get("x-family-id")
            .or(None)
            .ok_or_else(|| anyhow::anyhow!("Family ID is required"))?;

        FamilyId::from_string(family_id_str.to_str().unwrap_or("").to_string())
            .map_err(|_| anyhow::anyhow!("Invalid family ID"))
    }

    fn parse_todos_query(&self, _request: &ApiGatewayProxyRequest) -> GetTodosQuery {
        let status: Option<TodoStatus> = None;

        let limit: Option<u32> = Some(20);

        let offset: Option<u32> = None;

        GetTodosQuery {
            status,
            limit,
            offset,
        }
    }
}

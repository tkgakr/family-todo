use serde::{Deserialize, Serialize};
use shared::domain::{
    aggregates::{Todo, TodoStatus},
    events::TodoEvent,
    identifiers::TodoId,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct GetTodoQuery {
    pub todo_id: TodoId,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetTodosQuery {
    pub status: Option<TodoStatus>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl Default for GetTodosQuery {
    fn default() -> Self {
        Self {
            status: Some(TodoStatus::Active),
            limit: Some(50),
            offset: Some(0),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetTodoHistoryQuery {
    pub todo_id: TodoId,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct TodoListResponse {
    pub todos: Vec<Todo>,
    pub total_count: Option<u32>,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
pub struct TodoHistoryResponse {
    pub todo_id: TodoId,
    pub events: Vec<TodoEvent>,
    pub total_count: u32,
}

#[derive(Debug, Serialize)]
pub struct FamilyMembersResponse {
    pub members: Vec<FamilyMember>,
    pub total_count: u32,
}

#[derive(Debug, Serialize)]
pub struct FamilyMember {
    pub user_id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

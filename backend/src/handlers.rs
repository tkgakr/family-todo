use lambda_http::{Body, Request, Response};

use crate::db::DynamoClient;
use crate::error::ApiError;
use crate::models::{CreateTodoRequest, Todo, UpdateTodoRequest};

fn json_response(status: u16, body: &impl serde::Serialize) -> Result<Response<Body>, ApiError> {
    let json = serde_json::to_string(body).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(json))
        .unwrap())
}

pub async fn list_todos(
    db: &DynamoClient,
    family_id: &str,
) -> Result<Response<Body>, ApiError> {
    let todos = db.list_todos(family_id).await?;
    json_response(200, &todos)
}

pub async fn create_todo(
    req: Request,
    db: &DynamoClient,
    family_id: &str,
    user_id: &str,
) -> Result<Response<Body>, ApiError> {
    let body = req.body();
    let body_str = match body {
        Body::Text(s) => s.clone(),
        Body::Binary(b) => String::from_utf8(b.to_vec())
            .map_err(|_| ApiError::BadRequest("Invalid UTF-8".to_string()))?,
        Body::Empty => return Err(ApiError::BadRequest("Empty body".to_string())),
    };

    let input: CreateTodoRequest = serde_json::from_str(&body_str)?;

    if input.title.trim().is_empty() {
        return Err(ApiError::BadRequest("Title cannot be empty".to_string()));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let todo = Todo {
        id: ulid::Ulid::new().to_string(),
        title: input.title,
        completed: false,
        created_by: user_id.to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    db.put_todo(family_id, &todo).await?;
    json_response(201, &todo)
}

pub async fn update_todo(
    req: Request,
    db: &DynamoClient,
    family_id: &str,
    todo_id: &str,
) -> Result<Response<Body>, ApiError> {
    let body = req.body();
    let body_str = match body {
        Body::Text(s) => s.clone(),
        Body::Binary(b) => String::from_utf8(b.to_vec())
            .map_err(|_| ApiError::BadRequest("Invalid UTF-8".to_string()))?,
        Body::Empty => return Err(ApiError::BadRequest("Empty body".to_string())),
    };

    let input: UpdateTodoRequest = serde_json::from_str(&body_str)?;

    if input.title.is_none() && input.completed.is_none() {
        return Err(ApiError::BadRequest(
            "At least one of 'title' or 'completed' is required".to_string(),
        ));
    }

    let todo = db
        .update_todo(family_id, todo_id, input.title.as_deref(), input.completed)
        .await?;

    json_response(200, &todo)
}

pub async fn delete_todo(
    db: &DynamoClient,
    family_id: &str,
    todo_id: &str,
) -> Result<Response<Body>, ApiError> {
    db.delete_todo(family_id, todo_id).await?;
    Ok(Response::builder()
        .status(204)
        .body(Body::Empty)
        .unwrap())
}

use serde::{Deserialize, Serialize};
use shared::domain::identifiers::{TodoId, UserId};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateTodoCommand {
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateTodoCommand {
    pub todo_id: TodoId,
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompleteTodoCommand {
    pub todo_id: TodoId,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteTodoCommand {
    pub todo_id: TodoId,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "command_type")]
pub enum TodoCommand {
    CreateTodo(CreateTodoCommand),
    UpdateTodo(UpdateTodoCommand),
    CompleteTodo(CompleteTodoCommand),
    DeleteTodo(DeleteTodoCommand),
}

impl TodoCommand {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            TodoCommand::CreateTodo(cmd) => {
                if cmd.title.trim().is_empty() {
                    return Err("Title cannot be empty".to_string());
                }
                if cmd.title.len() > 200 {
                    return Err("Title too long (max 200 characters)".to_string());
                }
                if let Some(ref desc) = cmd.description {
                    if desc.len() > 1000 {
                        return Err("Description too long (max 1000 characters)".to_string());
                    }
                }
                if let Some(ref tags) = cmd.tags {
                    if tags.len() > 10 {
                        return Err("Too many tags (max 10)".to_string());
                    }
                }
            }
            TodoCommand::UpdateTodo(cmd) => {
                if let Some(ref title) = cmd.title {
                    if title.trim().is_empty() {
                        return Err("Title cannot be empty".to_string());
                    }
                    if title.len() > 200 {
                        return Err("Title too long (max 200 characters)".to_string());
                    }
                }
                if let Some(ref desc) = cmd.description {
                    if desc.len() > 1000 {
                        return Err("Description too long (max 1000 characters)".to_string());
                    }
                }
                if let Some(ref tags) = cmd.tags {
                    if tags.len() > 10 {
                        return Err("Too many tags (max 10)".to_string());
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
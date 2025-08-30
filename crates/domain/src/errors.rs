use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid TodoId: {0}")]
    InvalidTodoId(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invalid event data: {0}")]
    InvalidEvent(String),
}

#[derive(Debug, Error)]
pub enum TodoError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Concurrent modification detected")]
    ConcurrentModification,

    #[error("Todo not found: {0}")]
    NotFound(String),

    #[error("DynamoDB error: {0}")]
    DynamoDb(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

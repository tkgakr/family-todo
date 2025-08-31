use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid TODO ID: {0}")]
    InvalidTodoId(String),

    #[error("Invalid User ID: {0}")]
    InvalidUserId(String),

    #[error("TODO not found: {0}")]
    TodoNotFound(String),

    #[error("Invalid TODO state: {0}")]
    InvalidTodoState(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

pub type DomainResult<T> = Result<T, DomainError>;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("Temporary failure: {0}")]
    TemporaryFailure(String),

    #[error("Throttling exception: {0}")]
    ThrottlingException(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Permanent failure: {0}")]
    PermanentFailure(String),
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("Concurrent modification detected")]
    ConcurrentModification,

    #[error("Todo not found")]
    NotFound,

    #[error("DynamoDB error: {0}")]
    DynamoDb(String),
}

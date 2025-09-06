use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum DomainError {
    #[error("Invalid TodoId: {0}")]
    InvalidTodoId(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invalid event data: {0}")]
    InvalidEvent(String),

    #[error("Invalid event version: {0}")]
    InvalidEventVersion(String),

    #[error("Event serialization error: {0}")]
    EventSerialization(String),

    #[error("Event deserialization error: {0}")]
    EventDeserialization(String),

    #[error("Unknown event type: {0}")]
    UnknownEventType(String),

    // 家族関連のエラー
    #[error("Invalid FamilyId: {0}")]
    InvalidFamilyId(String),

    #[error("Invalid UserId: {0}")]
    InvalidUserId(String),

    #[error("Invalid family role: {0}")]
    InvalidFamilyRole(String),

    #[error("Invalid email: {0}")]
    InvalidEmail(String),

    #[error("Invalid display name: {0}")]
    InvalidDisplayName(String),

    #[error("Invalid family name: {0}")]
    InvalidFamilyName(String),

    #[error("Invalid invitation token: {0}")]
    InvalidInvitationToken(String),

    #[error("Invalid expiration date: {0}")]
    InvalidExpirationDate(String),

    #[error("Member already exists: {0}")]
    MemberAlreadyExists(String),

    #[error("Member not found: {0}")]
    MemberNotFound(String),

    #[error("Cannot remove the last admin")]
    CannotRemoveLastAdmin,

    #[error("Invitation expired")]
    InvitationExpired,

    #[error("Invitation already used")]
    InvitationAlreadyUsed,

    #[error("Unauthorized operation")]
    Unauthorized,
}

#[derive(Debug, Clone, Error)]
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

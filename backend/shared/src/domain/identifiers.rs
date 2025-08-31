use crate::domain::error::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};
use std::fmt;
use ulid::Ulid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TodoId(String);

impl TodoId {
    pub fn new() -> Self {
        Self(Ulid::new().to_string())
    }

    pub fn from_string(s: String) -> DomainResult<Self> {
        Ulid::from_string(&s).map_err(|_| DomainError::InvalidTodoId(s.clone()))?;
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn timestamp_ms(&self) -> Option<u64> {
        Ulid::from_string(&self.0)
            .ok()
            .map(|ulid| ulid.timestamp_ms())
    }

    pub fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.timestamp_ms()
            .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64))
    }
}

impl fmt::Display for TodoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TodoId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(String);

impl UserId {
    pub fn new() -> Self {
        Self(Ulid::new().to_string())
    }

    pub fn from_string(s: String) -> DomainResult<Self> {
        if s.is_empty() {
            return Err(DomainError::InvalidUserId(
                "User ID cannot be empty".to_string(),
            ));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FamilyId(String);

impl FamilyId {
    pub fn new() -> Self {
        Self(Ulid::new().to_string())
    }

    pub fn from_string(s: String) -> DomainResult<Self> {
        if s.is_empty() {
            return Err(DomainError::InvalidUserId(
                "Family ID cannot be empty".to_string(),
            ));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FamilyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for FamilyId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(String);

impl EventId {
    pub fn new() -> Self {
        Self(Ulid::new().to_string())
    }

    pub fn from_string(s: String) -> DomainResult<Self> {
        Ulid::from_string(&s).map_err(|_| DomainError::InvalidTodoId(s.clone()))?;
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

use crate::events::TodoEvent;
use crate::errors::DomainError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// ULID-based identifier for Todo entities
/// 
/// ULIDs provide lexicographically sortable, globally unique identifiers
/// with embedded timestamp information for natural ordering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TodoId(String);

impl TodoId {
    /// Creates a new TodoId with a fresh ULID
    pub fn new() -> Self {
        Self(ulid::Ulid::new().to_string())
    }

    /// Creates a TodoId from a validated ULID string
    /// 
    /// # Arguments
    /// * `id` - A valid ULID string
    /// 
    /// # Returns
    /// * `Ok(TodoId)` if the string is a valid ULID
    /// * `Err(DomainError)` if the string is not a valid ULID
    pub fn from_string(id: String) -> Result<Self, DomainError> {
        // Validate that the string is a valid ULID
        ulid::Ulid::from_string(&id)
            .map_err(|_| DomainError::InvalidTodoId(id.clone()))?;
        Ok(Self(id))
    }

    /// Creates a TodoId from a ULID without validation (unsafe)
    /// 
    /// This should only be used when you're certain the string is valid,
    /// such as when deserializing from trusted sources.
    pub fn from_string_unchecked(id: String) -> Self {
        Self(id)
    }

    /// Extracts the timestamp from the ULID
    /// 
    /// # Returns
    /// * `Some(timestamp_ms)` if the ULID is valid
    /// * `None` if the ULID is invalid
    pub fn timestamp_ms(&self) -> Option<u64> {
        ulid::Ulid::from_string(&self.0)
            .ok()
            .map(|ulid| ulid.timestamp_ms())
    }

    /// Extracts the timestamp as a DateTime<Utc>
    /// 
    /// # Returns
    /// * `Some(DateTime<Utc>)` if the ULID is valid and timestamp can be converted
    /// * `None` if the ULID is invalid or timestamp conversion fails
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamp_ms().and_then(|ms| {
            DateTime::from_timestamp_millis(ms as i64)
        })
    }

    /// Returns the string representation of the TodoId
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validates that the TodoId contains a valid ULID
    pub fn is_valid(&self) -> bool {
        ulid::Ulid::from_string(&self.0).is_ok()
    }
}

impl Default for TodoId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TodoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TodoId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s.to_string())
    }
}

impl From<TodoId> for String {
    fn from(id: TodoId) -> Self {
        id.0
    }
}

impl AsRef<str> for TodoId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod todo_id_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_new_todo_id_creates_valid_ulid() {
        let id = TodoId::new();
        assert!(id.is_valid());
        assert_eq!(id.as_str().len(), 26); // ULID length is 26 characters
    }

    #[test]
    fn test_new_todo_ids_are_unique() {
        let mut ids = HashSet::new();
        for _ in 0..1000 {
            let id = TodoId::new();
            assert!(ids.insert(id.as_str().to_string()), "Generated duplicate TodoId");
        }
    }

    #[test]
    fn test_from_string_with_valid_ulid() {
        let ulid_str = ulid::Ulid::new().to_string();
        let result = TodoId::from_string(ulid_str.clone());
        
        assert!(result.is_ok());
        let todo_id = result.unwrap();
        assert_eq!(todo_id.as_str(), ulid_str);
        assert!(todo_id.is_valid());
    }

    #[test]
    fn test_from_string_with_invalid_ulid() {
        let invalid_ids = vec![
            "invalid",
            "123",
            "01ARZ3NDEKTSV4RRFFQ69G5FA", // Too short
            "01ARZ3NDEKTSV4RRFFQ69G5FAVV", // Too long
            "01ARZ3NDEKTSV4RRFFQ69G5F@V", // Invalid character
            "",
        ];

        for invalid_id in invalid_ids {
            let result = TodoId::from_string(invalid_id.to_string());
            assert!(result.is_err(), "Expected error for invalid ULID: {}", invalid_id);
        }
    }

    #[test]
    fn test_from_string_unchecked() {
        let ulid_str = ulid::Ulid::new().to_string();
        let todo_id = TodoId::from_string_unchecked(ulid_str.clone());
        assert_eq!(todo_id.as_str(), ulid_str);
    }

    #[test]
    fn test_timestamp_ms_extraction() {
        let ulid = ulid::Ulid::new();
        let expected_timestamp = ulid.timestamp_ms();
        let todo_id = TodoId::from_string_unchecked(ulid.to_string());
        
        let extracted_timestamp = todo_id.timestamp_ms();
        assert_eq!(extracted_timestamp, Some(expected_timestamp));
    }

    #[test]
    fn test_timestamp_extraction() {
        let ulid = ulid::Ulid::new();
        let todo_id = TodoId::from_string_unchecked(ulid.to_string());
        
        let timestamp = todo_id.timestamp();
        assert!(timestamp.is_some());
        
        let timestamp = timestamp.unwrap();
        let expected_ms = ulid.timestamp_ms();
        let actual_ms = timestamp.timestamp_millis() as u64;
        
        // Allow for small differences due to precision
        assert!((actual_ms as i64 - expected_ms as i64).abs() < 1000);
    }

    #[test]
    fn test_timestamp_with_invalid_ulid() {
        let invalid_todo_id = TodoId("invalid".to_string());
        assert_eq!(invalid_todo_id.timestamp_ms(), None);
        assert_eq!(invalid_todo_id.timestamp(), None);
    }

    #[test]
    fn test_is_valid() {
        let valid_id = TodoId::new();
        assert!(valid_id.is_valid());

        let invalid_id = TodoId("invalid".to_string());
        assert!(!invalid_id.is_valid());
    }

    #[test]
    fn test_display_trait() {
        let ulid_str = ulid::Ulid::new().to_string();
        let todo_id = TodoId::from_string_unchecked(ulid_str.clone());
        assert_eq!(format!("{}", todo_id), ulid_str);
    }

    #[test]
    fn test_from_str_trait() {
        let ulid_str = ulid::Ulid::new().to_string();
        let result: Result<TodoId, _> = ulid_str.parse();
        
        assert!(result.is_ok());
        let todo_id = result.unwrap();
        assert_eq!(todo_id.as_str(), ulid_str);
    }

    #[test]
    fn test_from_str_trait_with_invalid() {
        let result: Result<TodoId, _> = "invalid".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_into_string() {
        let ulid_str = ulid::Ulid::new().to_string();
        let todo_id = TodoId::from_string_unchecked(ulid_str.clone());
        let converted: String = todo_id.into();
        assert_eq!(converted, ulid_str);
    }

    #[test]
    fn test_as_ref() {
        let ulid_str = ulid::Ulid::new().to_string();
        let todo_id = TodoId::from_string_unchecked(ulid_str.clone());
        let as_ref: &str = todo_id.as_ref();
        assert_eq!(as_ref, ulid_str);
    }

    #[test]
    fn test_equality_and_hashing() {
        let ulid_str = ulid::Ulid::new().to_string();
        let id1 = TodoId::from_string_unchecked(ulid_str.clone());
        let id2 = TodoId::from_string_unchecked(ulid_str.clone());
        let id3 = TodoId::new();

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);

        // Test that equal TodoIds have the same hash
        let mut set = HashSet::new();
        set.insert(id1.clone());
        assert!(set.contains(&id2));
        assert!(!set.contains(&id3));
    }

    #[test]
    fn test_serde_serialization() {
        let todo_id = TodoId::new();
        let serialized = serde_json::to_string(&todo_id).unwrap();
        let deserialized: TodoId = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(todo_id, deserialized);
    }

    #[test]
    fn test_chronological_ordering() {
        // Create TodoIds with a small delay to ensure different timestamps
        let id1 = TodoId::new();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = TodoId::new();

        let timestamp1 = id1.timestamp_ms().unwrap();
        let timestamp2 = id2.timestamp_ms().unwrap();
        
        assert!(timestamp1 <= timestamp2, "TodoIds should be chronologically ordered");
    }

    #[test]
    fn test_default_trait() {
        let default_id = TodoId::default();
        assert!(default_id.is_valid());
        assert_eq!(default_id.as_str().len(), 26);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: TodoId,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub completed: bool,
    pub created_by: String, // UserId
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u64,
}

impl Todo {
    pub fn apply(&mut self, event: TodoEvent) {
        // Event application logic will be implemented in task 2.3
        match event {
            TodoEvent::TodoCreatedV2 { .. } => {
                // Implementation placeholder
            }
            TodoEvent::TodoUpdatedV1 { .. } => {
                // Implementation placeholder
            }
            TodoEvent::TodoCompletedV1 { .. } => {
                // Implementation placeholder
            }
            TodoEvent::TodoDeletedV1 { .. } => {
                // Implementation placeholder
            }
        }
    }
}

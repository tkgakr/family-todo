use serde::{Deserialize, Serialize};
use ulid::Ulid;

pub mod domain;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TodoId(String);

impl TodoId {
    pub fn new() -> Self {
        Self(Ulid::new().to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    pub fn timestamp(&self) -> Option<u64> {
        self.parse_ulid().map(|ulid| ulid.timestamp_ms())
    }
    
    fn parse_ulid(&self) -> Option<Ulid> {
        Ulid::from_string(&self.0).ok()
    }
}

impl Default for TodoId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_id_new_generates_26_char_string() {
        // Arrange: なし

        // Act: 新しいTodoIdを生成
        let todo_id = TodoId::new();
        let id_str = todo_id.as_str();

        // Assert: 26文字のBase32形式であることを確認
        assert_eq!(id_str.len(), 26);
        let valid_chars = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        for c in id_str.chars() {
            assert!(valid_chars.contains(c), "Invalid character: {c}");
        }
    }
}

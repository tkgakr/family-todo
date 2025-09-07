use shared::domain::identifiers::{FamilyId, TodoId, UserId};

pub fn generate_test_ids() -> (FamilyId, UserId, TodoId) {
    (
        FamilyId::new(),
        UserId::new(),
        TodoId::new(),
    )
}
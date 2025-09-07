use integration_tests::{generate_test_ids, TodoFixtures};
use shared::domain::aggregates::{Todo, TodoStatus};

#[tokio::test]
async fn test_todo_creation_and_modification() {
    let (_family_id, user_id, todo_id) = generate_test_ids();

    // Todoアグリゲートを作成
    let mut todo = TodoFixtures::sample_todo(user_id.clone(), todo_id.clone());
    
    assert_eq!(todo.id, todo_id);
    assert_eq!(todo.title, "Sample Todo");
    assert_eq!(todo.description, Some("Sample Description".to_string()));
    assert_eq!(todo.status, TodoStatus::Active);
    assert_eq!(todo.created_by, user_id);
    assert!(todo.is_active());
    assert!(!todo.is_completed());
    assert!(!todo.is_deleted());

    // Todoを更新するイベントを適用
    let update_event = TodoFixtures::update_todo_event(
        todo_id.clone(),
        user_id.clone(),
        Some("Updated Title"),
        Some("Updated Description"),
    );

    todo.apply(update_event);
    
    assert_eq!(todo.title, "Updated Title");
    assert_eq!(todo.description, Some("Updated Description".to_string()));
    assert!(todo.is_active());

    // Todoを完了するイベントを適用
    let complete_event = TodoFixtures::complete_todo_event(todo_id, user_id);
    
    todo.apply(complete_event);
    
    assert_eq!(todo.status, TodoStatus::Completed);
    assert!(todo.is_completed());
    assert!(!todo.is_active());
    assert!(todo.completed_at.is_some());
}

#[tokio::test]
async fn test_todo_business_rules() {
    let (_, user_id, todo_id) = generate_test_ids();

    // 空のタイトルでTodo作成を試行
    let result = Todo::new(
        todo_id.clone(),
        "".to_string(),
        None,
        Vec::new(),
        user_id.clone(),
    );
    assert!(result.is_err());

    // 長すぎるタイトルでTodo作成を試行
    let long_title = "a".repeat(201);
    let result = Todo::new(
        todo_id.clone(),
        long_title,
        None,
        Vec::new(),
        user_id.clone(),
    );
    assert!(result.is_err());

    // 長すぎる説明でTodo作成を試行
    let long_description = "a".repeat(1001);
    let result = Todo::new(
        todo_id.clone(),
        "Valid Title".to_string(),
        Some(long_description),
        Vec::new(),
        user_id.clone(),
    );
    assert!(result.is_err());

    // 正常なTodo作成
    let result = Todo::new(
        todo_id,
        "Valid Title".to_string(),
        Some("Valid Description".to_string()),
        vec!["tag1".to_string(), "tag2".to_string()],
        user_id,
    );
    assert!(result.is_ok());

    let todo = result.unwrap();
    assert_eq!(todo.title, "Valid Title");
    assert_eq!(todo.description, Some("Valid Description".to_string()));
    assert_eq!(todo.tags, vec!["tag1", "tag2"]);
}

#[tokio::test]
async fn test_todo_state_transitions() {
    let (_, user_id, todo_id) = generate_test_ids();

    let mut todo = Todo::new(
        todo_id.clone(),
        "Test Todo".to_string(),
        None,
        Vec::new(),
        user_id.clone(),
    ).unwrap();

    // 初期状態はActive
    assert!(todo.can_be_updated());
    assert!(todo.can_be_completed());
    assert!(todo.can_be_deleted());

    // Todoを完了
    let complete_event = TodoFixtures::complete_todo_event(todo_id.clone(), user_id.clone());
    todo.apply(complete_event);

    assert!(!todo.can_be_updated());
    assert!(!todo.can_be_completed());
    assert!(todo.can_be_deleted());

    // Todoを削除
    let delete_event = TodoFixtures::delete_todo_event(todo_id, user_id);
    todo.apply(delete_event);

    assert!(!todo.can_be_updated());
    assert!(!todo.can_be_completed());
    assert!(!todo.can_be_deleted());
}

#[tokio::test]
async fn test_event_sequencing() {
    let (_, user_id, todo_id) = generate_test_ids();

    let mut todo = Todo::default();
    let initial_version = todo.version;

    // イベントを順次適用
    let create_event = TodoFixtures::create_todo_event(
        user_id.clone(),
        todo_id.clone(),
        "Initial Todo",
        Some("Initial Description"),
    );

    todo.apply(create_event);
    assert_eq!(todo.version, initial_version + 1);
    assert_eq!(todo.title, "Initial Todo");

    let update_event = TodoFixtures::update_todo_event(
        todo_id.clone(),
        user_id.clone(),
        Some("Updated Todo"),
        None,
    );

    todo.apply(update_event);
    assert_eq!(todo.version, initial_version + 2);
    assert_eq!(todo.title, "Updated Todo");
    assert_eq!(todo.description, Some("Initial Description".to_string())); // 説明は変更されない

    let complete_event = TodoFixtures::complete_todo_event(todo_id, user_id);

    todo.apply(complete_event);
    assert_eq!(todo.version, initial_version + 3);
    assert!(todo.is_completed());
}
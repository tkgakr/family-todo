use anyhow::Result;
use chrono::Utc;
use integration_tests::{
    create_sample_family_id, create_sample_todo, create_sample_todo_id, DynamoDbTestClient,
};
use shared::{
    domain::{
        aggregates::{TodoSnapshot, TodoStatus, TodoUpdates},
        error::UpdateError,
        events::TodoEvent,
        identifiers::{FamilyId, TodoId, UserId, EventId},
    },
};

#[tokio::test]
async fn test_dynamodb_table_creation_and_initialization() -> Result<()> {
    let client = match DynamoDbTestClient::new().await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("DynamoDB Local not available, skipping test");
            return Ok(());
        }
    };

    // テーブルが正常に作成されているか確認
    assert!(client.verify_table_exists().await?);

    // 初期状態ではアイテム数が0であることを確認
    assert_eq!(client.count_items().await?, 0);

    Ok(())
}

#[tokio::test]
async fn test_event_save_and_retrieve() -> Result<()> {
    let client = match DynamoDbTestClient::new().await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("DynamoDB Local not available, skipping test");
            return Ok(());
        }
    };
    let repository = client.create_repository();

    let family_id = create_sample_family_id();
    let todo_id = create_sample_todo_id();

    let user_id = UserId::new();
    
    // TodoCreatedV2イベントを作成して保存
    let created_event = TodoEvent::TodoCreatedV2 {
        event_id: EventId::new(),
        todo_id: todo_id.clone(),
        title: "テストTodo".to_string(),
        description: Some("テスト用のTodo項目".to_string()),
        tags: vec!["test".to_string(), "integration".to_string()],
        created_by: user_id,
        timestamp: Utc::now(),
    };

    // イベント保存
    repository.save_event(&family_id, &created_event).await?;

    // イベント取得
    let retrieved_events = repository
        .get_events_for_todo(&family_id, &todo_id)
        .await?;

    // 保存したイベントが取得できることを確認
    assert_eq!(retrieved_events.len(), 1);
    if let TodoEvent::TodoCreatedV2 {
        title,
        description,
        tags,
        ..
    } = &retrieved_events[0]
    {
        assert_eq!(title, "テストTodo");
        assert_eq!(description.as_ref().unwrap(), "テスト用のTodo項目");
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"test".to_string()));
        assert!(tags.contains(&"integration".to_string()));
    } else {
        panic!("Expected TodoCreatedV2 event");
    }

    // アイテム数の確認
    assert_eq!(client.count_items().await?, 1);

    Ok(())
}

#[tokio::test]
async fn test_todo_projection_save_and_retrieve() -> Result<()> {
    let client = match DynamoDbTestClient::new().await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("DynamoDB Local not available, skipping test");
            return Ok(());
        }
    };
    let repository = client.create_repository();

    let family_id = create_sample_family_id();
    let mut todo = create_sample_todo();
    todo.title = "プロジェクション テスト Todo".to_string();
    todo.description = Some("プロジェクション保存・取得テスト".to_string());

    // Todo プロジェクション保存
    repository.save_todo_projection(&family_id, &todo).await?;

    // Todo プロジェクション取得
    let retrieved_todo = repository.get_todo(&family_id, &todo.id).await?;

    // データが正しく保存・取得されることを確認
    assert_eq!(retrieved_todo.title, "プロジェクション テスト Todo");
    assert_eq!(
        retrieved_todo.description.unwrap(),
        "プロジェクション保存・取得テスト"
    );
    assert_eq!(retrieved_todo.version, todo.version);
    assert_eq!(retrieved_todo.id, todo.id);

    Ok(())
}

#[tokio::test]
async fn test_active_todos_query() -> Result<()> {
    let client = match DynamoDbTestClient::new().await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("DynamoDB Local not available, skipping test");
            return Ok(());
        }
    };
    let repository = client.create_repository();

    let family_id = create_sample_family_id();

    // アクティブなTodoを3個作成・保存
    for i in 1..=3 {
        let mut todo = create_sample_todo();
        todo.id = TodoId::new();
        todo.title = format!("アクティブTodo {i}");
        todo.status = TodoStatus::Active;

        repository.save_todo_projection(&family_id, &todo).await?;
    }

    // 完了済みのTodoも1個作成・保存（GSI1には含まれない）
    let mut completed_todo = create_sample_todo();
    completed_todo.id = TodoId::new();
    completed_todo.title = "完了済みTodo".to_string();
    completed_todo.status = TodoStatus::Completed;

    repository
        .save_todo_projection(&family_id, &completed_todo)
        .await?;

    // アクティブなTodo一覧を取得
    let active_todos = repository.get_active_todos(&family_id, None).await?;

    // アクティブなTodoのみ取得されることを確認
    assert_eq!(active_todos.len(), 3);
    for todo in &active_todos {
        assert_eq!(todo.status, TodoStatus::Active);
        assert!(todo.title.starts_with("アクティブTodo"));
    }

    // limit指定での取得テスト
    let limited_todos = repository.get_active_todos(&family_id, Some(2)).await?;
    assert_eq!(limited_todos.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_optimistic_locking_update() -> Result<()> {
    let client = match DynamoDbTestClient::new().await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("DynamoDB Local not available, skipping test");
            return Ok(());
        }
    };
    let repository = client.create_repository();

    let family_id = create_sample_family_id();
    let todo = create_sample_todo();

    // 初期Todo保存
    repository.save_todo_projection(&family_id, &todo).await?;

    // 正常な更新（正しいバージョン）
    let updates = TodoUpdates {
        title: Some("更新されたタイトル".to_string()),
        description: Some("更新された説明".to_string()),
        tags: None,
    };

    let updated_todo = repository
        .update_todo_with_lock(&family_id, &todo, updates)
        .await?;

    assert_eq!(updated_todo.title, "更新されたタイトル");
    assert_eq!(updated_todo.description.unwrap(), "更新された説明");
    assert_eq!(updated_todo.version, todo.version + 1);

    // 楽観的ロック失敗テスト（古いバージョンでの更新試行）
    let old_version_updates = TodoUpdates {
        title: Some("失敗するはずの更新".to_string()),
        description: None,
        tags: None,
    };

    let lock_error = repository
        .update_todo_with_lock(&family_id, &todo, old_version_updates)
        .await;

    assert!(matches!(lock_error, Err(UpdateError::ConcurrentModification)));

    Ok(())
}

#[tokio::test]
async fn test_snapshot_operations() -> Result<()> {
    let client = match DynamoDbTestClient::new().await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("DynamoDB Local not available, skipping test");
            return Ok(());
        }
    };
    let repository = client.create_repository();

    let family_id = create_sample_family_id();
    let todo_id = create_sample_todo_id();
    let todo = create_sample_todo();

    // スナップショット作成・保存
    let snapshot = TodoSnapshot {
        todo_id: todo_id.clone(),
        state: todo.clone(),
        last_event_id: ulid::Ulid::new().to_string(),
        stream_version: 1,
        created_at: Utc::now(),
    };

    repository.save_snapshot(&family_id, &snapshot).await?;

    // スナップショット取得
    let retrieved_snapshot = repository
        .get_latest_snapshot(&family_id, &todo_id)
        .await?;

    assert!(retrieved_snapshot.is_some());
    let retrieved_snapshot = retrieved_snapshot.unwrap();
    assert_eq!(retrieved_snapshot.todo_id, todo_id);
    assert_eq!(retrieved_snapshot.state.title, todo.title);
    assert_eq!(retrieved_snapshot.last_event_id, snapshot.last_event_id);

    Ok(())
}

#[tokio::test]
async fn test_todo_rebuild_from_events() -> Result<()> {
    let client = match DynamoDbTestClient::new().await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("DynamoDB Local not available, skipping test");
            return Ok(());
        }
    };
    let repository = client.create_repository();

    let family_id = create_sample_family_id();
    let todo_id = create_sample_todo_id();

    let user_id = UserId::new();
    
    // 一連のイベントを作成・保存
    let events = vec![
        TodoEvent::TodoCreatedV2 {
            event_id: EventId::new(),
            todo_id: todo_id.clone(),
            title: "初期タイトル".to_string(),
            description: Some("初期説明".to_string()),
            tags: vec!["initial".to_string()],
            created_by: user_id.clone(),
            timestamp: Utc::now(),
        },
        TodoEvent::TodoUpdatedV1 {
            event_id: EventId::new(),
            todo_id: todo_id.clone(),
            title: Some("更新されたタイトル".to_string()),
            description: None,
            updated_by: user_id.clone(),
            timestamp: Utc::now(),
        },
        TodoEvent::TodoCompletedV1 {
            event_id: EventId::new(),
            todo_id: todo_id.clone(),
            completed_by: user_id,
            timestamp: Utc::now(),
        },
    ];

    // イベントを順次保存
    for event in &events {
        repository.save_event(&family_id, event).await?;
    }

    // イベントからTodoを再構築
    let rebuilt_todo = repository
        .rebuild_todo_from_snapshot(&family_id, &todo_id)
        .await?;

    // 最終状態が正しく再構築されることを確認
    assert_eq!(rebuilt_todo.title, "更新されたタイトル");
    assert_eq!(rebuilt_todo.description.unwrap(), "初期説明"); // 更新されていない
    assert_eq!(rebuilt_todo.status, TodoStatus::Completed);
    // 注意: versionはイベント処理では自動増分される可能性がある
    assert!(rebuilt_todo.tags.contains(&"initial".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_multiple_families_isolation() -> Result<()> {
    let client = match DynamoDbTestClient::new().await {
        Ok(client) => client,
        Err(_) => {
            eprintln!("DynamoDB Local not available, skipping test");
            return Ok(());
        }
    };
    let repository = client.create_repository();

    let family_id_1 = FamilyId::new();
    let family_id_2 = FamilyId::new();
    let _todo_id = create_sample_todo_id();

    // 家族1のTodo作成・保存
    let mut todo_1 = create_sample_todo();
    todo_1.title = "家族1のTodo".to_string();
    repository.save_todo_projection(&family_id_1, &todo_1).await?;

    // 家族2のTodo作成・保存
    let mut todo_2 = create_sample_todo();
    todo_2.title = "家族2のTodo".to_string();
    repository.save_todo_projection(&family_id_2, &todo_2).await?;

    // 家族1のTodoが家族2のクエリに含まれないことを確認
    let family_2_todos = repository.get_active_todos(&family_id_2, None).await?;
    assert_eq!(family_2_todos.len(), 1);
    assert_eq!(family_2_todos[0].title, "家族2のTodo");

    // 家族2のTodoが家族1のクエリに含まれないことを確認
    let family_1_todos = repository.get_active_todos(&family_id_1, None).await?;
    assert_eq!(family_1_todos.len(), 1);
    assert_eq!(family_1_todos[0].title, "家族1のTodo");

    Ok(())
}
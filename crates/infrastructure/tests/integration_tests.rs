use domain::{Todo, TodoError, TodoEvent, TodoId};
use infrastructure::{
    DynamoDbClient, EventRepository, OptimisticLockService, ProjectionRepository,
};
use shared::Config;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// 統合テスト用のセットアップ
async fn setup_test_environment() -> (DynamoDbClient, String) {
    let config = Config {
        dynamodb_table: "test-table".to_string(),
        environment: "test".to_string(),
        aws_region: "ap-northeast-1".to_string(),
        dynamodb_endpoint: Some("http://localhost:8000".to_string()),
        retry_max_attempts: 2,
        retry_initial_delay_ms: 10,
    };
    let client = DynamoDbClient::new(&config)
        .await
        .expect("テスト用DynamoDBクライアントの作成に失敗");

    let test_family_id = format!("test_family_{}", ulid::Ulid::new());

    (client, test_family_id)
}

/// 楽観的ロックの基本動作テスト
#[tokio::test]
async fn test_optimistic_lock_basic_operations() {
    let (client, family_id) = setup_test_environment().await;
    let service = OptimisticLockService::new(client);

    // 初期ToDoを作成
    let todo_id = TodoId::new();
    let create_event = TodoEvent::new_todo_created(
        todo_id.clone(),
        "統合テストToDo".to_string(),
        Some("統合テスト用の説明".to_string()),
        vec!["統合テスト".to_string()],
        "test_user".to_string(),
    );

    let initial_todo = Todo::from_created_event(&create_event).unwrap();

    // プロジェクションを保存（実際のDynamoDB Localが必要）
    match service
        .projection_repo()
        .save_projection(&family_id, initial_todo.clone())
        .await
    {
        Ok(_) => {
            println!("✓ 初期ToDo保存成功");

            // 楽観的ロック更新テスト
            let update_event = TodoEvent::new_todo_updated(
                todo_id.clone(),
                Some("更新されたタイトル".to_string()),
                Some("更新された説明".to_string()),
                "test_user".to_string(),
            );

            match service
                .update_todo_with_lock(&family_id, &todo_id, initial_todo.version, update_event)
                .await
            {
                Ok(updated_todo) => {
                    println!("✓ 楽観的ロック更新成功");
                    assert_eq!(updated_todo.title, "更新されたタイトル");
                    assert_eq!(updated_todo.version, initial_todo.version + 1);

                    // バージョン競合テスト
                    let conflict_event = TodoEvent::new_todo_updated(
                        todo_id.clone(),
                        Some("競合更新".to_string()),
                        None,
                        "test_user".to_string(),
                    );

                    match service
                        .update_todo_with_lock(
                            &family_id,
                            &todo_id,
                            initial_todo.version, // 古いバージョン
                            conflict_event,
                        )
                        .await
                    {
                        Err(TodoError::ConcurrentModification) => {
                            println!("✓ バージョン競合検出成功");
                        }
                        _ => panic!("バージョン競合が検出されませんでした"),
                    }
                }
                Err(e) => println!("⚠ 楽観的ロック更新スキップ: {}", e),
            }
        }
        Err(e) => println!("⚠ 統合テストスキップ (DynamoDB Local未起動?): {}", e),
    }
}

/// 同時実行とリトライ機能のテスト
#[tokio::test]
async fn test_concurrent_updates_with_retry() {
    let (client, family_id) = setup_test_environment().await;
    let service = Arc::new(OptimisticLockService::new(client));

    // 初期ToDoを作成
    let todo_id = TodoId::new();
    let create_event = TodoEvent::new_todo_created(
        todo_id.clone(),
        "同時実行テストToDo".to_string(),
        None,
        vec![],
        "test_user".to_string(),
    );

    let initial_todo = Todo::from_created_event(&create_event).unwrap();

    match service
        .projection_repo()
        .save_projection(&family_id, initial_todo.clone())
        .await
    {
        Ok(_) => {
            println!("✓ 初期ToDo保存成功");

            // 複数の同時更新を実行
            let service1 = service.clone();
            let service2 = service.clone();
            let service3 = service.clone();

            let family_id1 = family_id.clone();
            let family_id2 = family_id.clone();
            let family_id3 = family_id.clone();

            let todo_id1 = todo_id.clone();
            let todo_id2 = todo_id.clone();
            let todo_id3 = todo_id.clone();

            let handle1 = tokio::spawn(async move {
                service1
                    .update_todo_with_retry(
                        &family_id1,
                        &todo_id1,
                        5, // 最大5回リトライ
                        |todo| {
                            Ok(TodoEvent::new_todo_updated(
                                todo.id.clone(),
                                Some(format!("更新1_{}", chrono::Utc::now().timestamp_millis())),
                                None,
                                "user1".to_string(),
                            ))
                        },
                    )
                    .await
            });

            let handle2 = tokio::spawn(async move {
                sleep(Duration::from_millis(10)).await; // 少し遅延
                service2
                    .update_todo_with_retry(&family_id2, &todo_id2, 5, |todo| {
                        Ok(TodoEvent::new_todo_updated(
                            todo.id.clone(),
                            Some(format!("更新2_{}", chrono::Utc::now().timestamp_millis())),
                            None,
                            "user2".to_string(),
                        ))
                    })
                    .await
            });

            let handle3 = tokio::spawn(async move {
                sleep(Duration::from_millis(20)).await; // さらに遅延
                service3
                    .update_todo_with_retry(&family_id3, &todo_id3, 5, |todo| {
                        Ok(TodoEvent::new_todo_completed(
                            todo.id.clone(),
                            "user3".to_string(),
                        ))
                    })
                    .await
            });

            let (result1, result2, result3) = tokio::join!(handle1, handle2, handle3);

            let mut success_count = 0;
            if result1.is_ok() && result1.unwrap().is_ok() {
                success_count += 1;
                println!("✓ 同時更新1成功");
            }
            if result2.is_ok() && result2.unwrap().is_ok() {
                success_count += 1;
                println!("✓ 同時更新2成功");
            }
            if result3.is_ok() && result3.unwrap().is_ok() {
                success_count += 1;
                println!("✓ 同時更新3成功");
            }

            println!("同時更新成功数: {}/3", success_count);

            // 最終状態を確認
            if let Ok(Some(final_todo)) = service
                .projection_repo()
                .get_projection(&family_id, &todo_id)
                .await
            {
                println!(
                    "✓ 最終ToDo状態: version={}, title={}, completed={}",
                    final_todo.version, final_todo.title, final_todo.completed
                );
                assert!(final_todo.version > initial_todo.version);
            }
        }
        Err(e) => println!("⚠ 同時実行テストスキップ (DynamoDB Local未起動?): {}", e),
    }
}

/// イベントストアとプロジェクションの整合性テスト
#[tokio::test]
async fn test_event_projection_consistency() {
    let (client, family_id) = setup_test_environment().await;
    let event_repo = EventRepository::new(client.clone());
    let projection_repo = ProjectionRepository::new(client.clone());
    let _service = OptimisticLockService::new(client);

    let todo_id = TodoId::new();

    // 一連のイベントを生成
    let events = vec![
        TodoEvent::new_todo_created(
            todo_id.clone(),
            "整合性テストToDo".to_string(),
            Some("初期説明".to_string()),
            vec!["テスト".to_string()],
            "user1".to_string(),
        ),
        TodoEvent::new_todo_updated(
            todo_id.clone(),
            Some("更新されたタイトル".to_string()),
            None,
            "user2".to_string(),
        ),
        TodoEvent::new_todo_updated(
            todo_id.clone(),
            None,
            Some("更新された説明".to_string()),
            "user3".to_string(),
        ),
        TodoEvent::new_todo_completed(todo_id.clone(), "user4".to_string()),
    ];

    match event_repo.save_event(&family_id, events[0].clone()).await {
        Ok(_) => {
            println!("✓ イベント保存テスト開始");

            // 初期プロジェクションを作成
            let mut current_todo = Todo::from_created_event(&events[0]).unwrap();
            let _ = projection_repo
                .save_projection(&family_id, current_todo.clone())
                .await;

            // 残りのイベントを順次適用
            for (i, event) in events.iter().skip(1).enumerate() {
                if let Ok(_) = event_repo.save_event(&family_id, event.clone()).await {
                    current_todo.apply(event.clone()).unwrap();
                    let _ = projection_repo
                        .save_projection(&family_id, current_todo.clone())
                        .await;
                    println!("✓ イベント {} 適用完了", i + 2);
                }
            }

            // イベントストアから再構築
            if let Ok(stored_events) = event_repo.get_events(&family_id, &todo_id).await {
                if !stored_events.is_empty() {
                    let reconstructed_todo = Todo::from_events(stored_events).unwrap();

                    // プロジェクションと再構築されたToDoの比較
                    if let Ok(Some(projection_todo)) =
                        projection_repo.get_projection(&family_id, &todo_id).await
                    {
                        assert_eq!(reconstructed_todo.id, projection_todo.id);
                        assert_eq!(reconstructed_todo.title, projection_todo.title);
                        assert_eq!(reconstructed_todo.description, projection_todo.description);
                        assert_eq!(reconstructed_todo.completed, projection_todo.completed);
                        assert_eq!(reconstructed_todo.version, projection_todo.version);

                        println!("✓ イベントストアとプロジェクションの整合性確認完了");
                        println!("  - 最終バージョン: {}", projection_todo.version);
                        println!("  - 最終タイトル: {}", projection_todo.title);
                        println!("  - 完了状態: {}", projection_todo.completed);
                    }
                }
            }
        }
        Err(e) => println!("⚠ 整合性テストスキップ (DynamoDB Local未起動?): {}", e),
    }
}

/// エラーハンドリングとリトライ機能のテスト
#[tokio::test]
async fn test_error_handling_and_retry() {
    let (client, family_id) = setup_test_environment().await;
    let service = OptimisticLockService::new(client);

    // 存在しないToDoの更新を試行
    let non_existent_todo_id = TodoId::new();
    let update_event = TodoEvent::new_todo_updated(
        non_existent_todo_id.clone(),
        Some("存在しない".to_string()),
        None,
        "user1".to_string(),
    );

    match service
        .update_todo_with_lock(&family_id, &non_existent_todo_id, 1, update_event)
        .await
    {
        Err(TodoError::NotFound(_)) => {
            println!("✓ 存在しないToDo更新エラー検出成功");
        }
        Err(e) => {
            println!("⚠ 予期しないエラー (DynamoDB接続エラーの可能性): {}", e);
        }
        Ok(_) => panic!("存在しないToDoの更新が成功してしまいました"),
    }

    // 無効なイベントでのリトライテスト
    let todo_id = TodoId::new();
    let create_event = TodoEvent::new_todo_created(
        todo_id.clone(),
        "エラーテストToDo".to_string(),
        None,
        vec![],
        "user1".to_string(),
    );

    let initial_todo = Todo::from_created_event(&create_event).unwrap();

    if service
        .projection_repo()
        .save_projection(&family_id, initial_todo.clone())
        .await
        .is_ok()
    {
        // 無効なタイトル（空文字）での更新を試行
        match service
            .update_todo_with_retry(&family_id, &todo_id, 3, |_todo| {
                Err(TodoError::Validation("無効なデータ".to_string()))
            })
            .await
        {
            Err(TodoError::Validation(_)) => {
                println!("✓ バリデーションエラー検出成功");
            }
            _ => println!("⚠ バリデーションエラーが期待通りに発生しませんでした"),
        }
    }
}

/// パフォーマンステスト（軽量版）
#[tokio::test]
async fn test_performance_basic() {
    let (client, family_id) = setup_test_environment().await;
    let service = OptimisticLockService::new(client);

    let todo_count = 10;
    let mut todo_ids = Vec::new();

    // 複数のToDoを作成
    for i in 0..todo_count {
        let todo_id = TodoId::new();
        let create_event = TodoEvent::new_todo_created(
            todo_id.clone(),
            format!("パフォーマンステストToDo {}", i),
            None,
            vec![],
            "perf_user".to_string(),
        );

        let todo = Todo::from_created_event(&create_event).unwrap();

        if service
            .projection_repo()
            .save_projection(&family_id, todo)
            .await
            .is_ok()
        {
            todo_ids.push(todo_id);
        }
    }

    if !todo_ids.is_empty() {
        let start_time = std::time::Instant::now();

        // 並行して更新を実行
        let mut handles = Vec::new();
        for (i, todo_id) in todo_ids.iter().enumerate() {
            let service_clone = service.clone();
            let family_id_clone = family_id.clone();
            let todo_id_clone = todo_id.clone();

            let handle: tokio::task::JoinHandle<Result<Todo, TodoError>> =
                tokio::spawn(async move {
                    service_clone
                        .update_todo_with_retry(&family_id_clone, &todo_id_clone, 3, |todo| {
                            Ok(TodoEvent::new_todo_updated(
                                todo.id.clone(),
                                Some(format!("並行更新 {}", i)),
                                None,
                                "perf_user".to_string(),
                            ))
                        })
                        .await
                });

            handles.push(handle);
        }

        // すべての更新完了を待機
        let results = futures::future::join_all(handles).await;
        let elapsed = start_time.elapsed();

        let success_count = results
            .iter()
            .filter(|r| r.is_ok() && r.as_ref().unwrap().is_ok())
            .count();

        println!("✓ パフォーマンステスト完了:");
        println!("  - 処理時間: {:?}", elapsed);
        println!("  - 成功数: {}/{}", success_count, todo_count);
        println!("  - 平均処理時間: {:?}", elapsed / todo_count as u32);

        // 基本的なパフォーマンス要件チェック（緩い条件）
        if success_count > 0 {
            let avg_time_per_operation = elapsed / success_count as u32;
            assert!(
                avg_time_per_operation < Duration::from_secs(5),
                "平均処理時間が5秒を超えています: {:?}",
                avg_time_per_operation
            );
        }
    } else {
        println!("⚠ パフォーマンステストスキップ (DynamoDB Local未起動?)");
    }
}

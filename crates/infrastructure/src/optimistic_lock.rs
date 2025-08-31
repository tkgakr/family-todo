use crate::{DynamoDbClient, ProjectionRepository, EventRepository, retry_dynamodb_operation, RetryConfig};
use aws_sdk_dynamodb::types::AttributeValue;
use domain::{TodoError, TodoEvent, TodoId, Todo};
use tracing::{info, warn, debug};

/// 楽観的ロック制御を提供するサービス
pub struct OptimisticLockService {
    db: DynamoDbClient,
    event_repo: EventRepository,
    projection_repo: ProjectionRepository,
}

impl OptimisticLockService {
    pub fn new(db: DynamoDbClient) -> Self {
        let event_repo = EventRepository::new(db.clone());
        let projection_repo = ProjectionRepository::new(db.clone());
        
        Self {
            db,
            event_repo,
            projection_repo,
        }
    }

    /// 楽観的ロックを使用してToDoを更新
    /// バージョン番号による同時実行制御を行う
    pub async fn update_todo_with_lock(
        &self,
        family_id: &str,
        todo_id: &TodoId,
        expected_version: u64,
        event: TodoEvent,
    ) -> Result<Todo, TodoError> {
        info!("楽観的ロック更新開始: family_id={}, todo_id={}, expected_version={}", 
              family_id, todo_id, expected_version);

        // 現在のプロジェクションを取得してバージョンを確認
        let current_todo = self.projection_repo
            .get_projection(family_id, todo_id)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("ToDo not found: {}", todo_id)))?;

        // バージョンチェック
        if current_todo.version != expected_version {
            warn!("バージョン競合検出: expected={}, actual={}", expected_version, current_todo.version);
            return Err(TodoError::ConcurrentModification);
        }

        // イベントを保存
        self.event_repo.save_event(family_id, event.clone()).await?;

        // プロジェクションを更新
        let mut updated_todo = current_todo;
        updated_todo.apply(event)?;
        
        self.projection_repo.save_projection(family_id, updated_todo.clone()).await?;

        debug!("楽観的ロック更新完了: new_version={}", updated_todo.version);
        Ok(updated_todo)
    }

    /// リトライ付きの楽観的ロック更新
    /// 同時実行による競合が発生した場合、自動的にリトライする
    pub async fn update_todo_with_retry<F>(
        &self,
        family_id: &str,
        todo_id: &TodoId,
        max_retries: u32,
        event_generator: F,
    ) -> Result<Todo, TodoError>
    where
        F: Fn(&Todo) -> Result<TodoEvent, TodoError>,
    {
        info!("リトライ付き楽観的ロック更新開始: family_id={}, todo_id={}, max_retries={}", 
              family_id, todo_id, max_retries);

        let retry_config = RetryConfig {
            max_attempts: max_retries,
            initial_delay_ms: 50,
            backoff_multiplier: 1.5,
            max_delay_ms: 1000,
        };

        retry_dynamodb_operation(
            || async {
                // 最新のプロジェクションを取得
                let current_todo = self.projection_repo
                    .get_projection(family_id, todo_id)
                    .await?
                    .ok_or_else(|| TodoError::NotFound(format!("ToDo not found: {}", todo_id)))?;

                // イベントを生成
                let event = event_generator(&current_todo)?;

                // 楽観的ロック更新を実行
                self.update_todo_with_lock(family_id, todo_id, current_todo.version, event).await
            },
            Some(&retry_config),
        ).await
    }

    /// 条件付きでプロジェクションを更新（DynamoDB条件式使用）
    pub async fn conditional_update_projection(
        &self,
        family_id: &str,
        todo: &Todo,
        expected_version: u64,
    ) -> Result<(), TodoError> {
        info!("条件付きプロジェクション更新: family_id={}, todo_id={}, expected_version={}", 
              family_id, todo.id, expected_version);

        let pk = format!("FAMILY#{}", family_id);
        let sk = format!("TODO#CURRENT#{}", todo.id.as_str());

        let todo_json = serde_json::to_string(todo)
            .map_err(|e| TodoError::Internal(format!("ToDo シリアライゼーションエラー: {}", e)))?;

        retry_dynamodb_operation(
            || async {
                self.db.client()
                    .update_item()
                    .table_name(self.db.table_name())
                    .key("PK", AttributeValue::S(pk.clone()))
                    .key("SK", AttributeValue::S(sk.clone()))
                    .update_expression("SET #data = :data, #version = :new_version, UpdatedAt = :updated_at")
                    .condition_expression("#version = :expected_version")
                    .expression_attribute_names("#data", "Data")
                    .expression_attribute_names("#version", "Version")
                    .expression_attribute_values(":data", AttributeValue::S(todo_json.clone()))
                    .expression_attribute_values(":new_version", AttributeValue::N(todo.version.to_string()))
                    .expression_attribute_values(":expected_version", AttributeValue::N(expected_version.to_string()))
                    .expression_attribute_values(":updated_at", AttributeValue::S(chrono::Utc::now().to_rfc3339()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("条件付きプロジェクション更新完了");
                Ok(())
            },
            None,
        ).await
    }

    /// バッチでの楽観的ロック更新
    /// 複数のToDoを同時に更新する際の整合性を保つ
    pub async fn batch_update_with_lock(
        &self,
        family_id: &str,
        updates: Vec<(TodoId, u64, TodoEvent)>, // (todo_id, expected_version, event)
    ) -> Result<Vec<Todo>, TodoError> {
        info!("バッチ楽観的ロック更新開始: family_id={}, count={}", family_id, updates.len());

        let mut updated_todos = Vec::new();

        // トランザクション的な処理のため、すべての更新を順次実行
        for (todo_id, expected_version, event) in updates {
            let updated_todo = self.update_todo_with_lock(
                family_id,
                &todo_id,
                expected_version,
                event,
            ).await?;
            
            updated_todos.push(updated_todo);
        }

        debug!("バッチ楽観的ロック更新完了: {} 件", updated_todos.len());
        Ok(updated_todos)
    }

    /// ToDoの存在確認と最新バージョン取得
    pub async fn get_todo_version(
        &self,
        family_id: &str,
        todo_id: &TodoId,
    ) -> Result<Option<u64>, TodoError> {
        let pk = format!("FAMILY#{}", family_id);
        let sk = format!("TODO#CURRENT#{}", todo_id.as_str());

        retry_dynamodb_operation(
            || async {
                let response = self.db.client()
                    .get_item()
                    .table_name(self.db.table_name())
                    .key("PK", AttributeValue::S(pk.clone()))
                    .key("SK", AttributeValue::S(sk.clone()))
                    .projection_expression("Version")
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                if let Some(item) = response.item {
                    let version = item.get("Version")
                        .and_then(|v| v.as_n().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .ok_or_else(|| TodoError::Internal("バージョン取得エラー".to_string()))?;

                    Ok(Some(version))
                } else {
                    Ok(None)
                }
            },
            None,
        ).await
    }

    /// 楽観的ロックによるToDo削除
    pub async fn delete_todo_with_lock(
        &self,
        family_id: &str,
        todo_id: &TodoId,
        expected_version: u64,
        delete_event: TodoEvent,
    ) -> Result<(), TodoError> {
        info!("楽観的ロック削除開始: family_id={}, todo_id={}, expected_version={}", 
              family_id, todo_id, expected_version);

        // 削除イベントを保存
        self.event_repo.save_event(family_id, delete_event).await?;

        // プロジェクションを条件付きで削除
        let pk = format!("FAMILY#{}", family_id);
        let sk = format!("TODO#CURRENT#{}", todo_id.as_str());

        retry_dynamodb_operation(
            || async {
                self.db.client()
                    .delete_item()
                    .table_name(self.db.table_name())
                    .key("PK", AttributeValue::S(pk.clone()))
                    .key("SK", AttributeValue::S(sk.clone()))
                    .condition_expression("Version = :expected_version")
                    .expression_attribute_values(":expected_version", AttributeValue::N(expected_version.to_string()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("楽観的ロック削除完了");
                Ok(())
            },
            None,
        ).await
    }
}

/// 楽観的ロック操作のヘルパー関数群
pub mod helpers {
    use super::*;

    /// ToDoタイトル更新のイベント生成ヘルパー
    pub fn create_title_update_event(
        todo: &Todo,
        new_title: String,
        updated_by: String,
    ) -> Result<TodoEvent, TodoError> {
        if new_title.is_empty() || new_title.len() > 200 {
            return Err(TodoError::Validation("無効なタイトル".to_string()));
        }

        Ok(TodoEvent::new_todo_updated(
            todo.id.clone(),
            Some(new_title),
            None,
            updated_by,
        ))
    }

    /// ToDo完了のイベント生成ヘルパー
    pub fn create_completion_event(
        todo: &Todo,
        completed_by: String,
    ) -> Result<TodoEvent, TodoError> {
        if todo.completed {
            return Err(TodoError::Validation("ToDoは既に完了しています".to_string()));
        }

        Ok(TodoEvent::new_todo_completed(
            todo.id.clone(),
            completed_by,
        ))
    }

    /// ToDo削除のイベント生成ヘルパー
    pub fn create_deletion_event(
        todo: &Todo,
        deleted_by: String,
        reason: Option<String>,
    ) -> Result<TodoEvent, TodoError> {
        if todo.deleted {
            return Err(TodoError::Validation("ToDoは既に削除されています".to_string()));
        }

        Ok(TodoEvent::new_todo_deleted(
            todo.id.clone(),
            deleted_by,
            reason,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::Config;
    use domain::{TodoEvent, TodoId};
    use std::sync::Arc;


    async fn setup_test_service() -> OptimisticLockService {
        let config = Config {
            dynamodb_table: "test-table".to_string(),
            environment: "test".to_string(),
            aws_region: "ap-northeast-1".to_string(),
            dynamodb_endpoint: Some("http://localhost:8000".to_string()),
            retry_max_attempts: 2,
            retry_initial_delay_ms: 10,
        };
        let client = DynamoDbClient::new_for_test(&config).await.unwrap();
        OptimisticLockService::new(client)
    }

    #[tokio::test]
    async fn test_optimistic_lock_success() {
        let service = setup_test_service().await;
        let family_id = "test_family";
        let todo_id = TodoId::new();

        // 初期ToDoを作成
        let create_event = TodoEvent::new_todo_created(
            todo_id.clone(),
            "初期タイトル".to_string(),
            None,
            vec![],
            "user123".to_string(),
        );

        let initial_todo = Todo::from_created_event(&create_event).unwrap();
        
        // 実際のDynamoDB Localが動いていない場合はスキップ
        if service.projection_repo.save_projection(family_id, initial_todo.clone()).await.is_ok() {
            // 楽観的ロック更新
            let update_event = TodoEvent::new_todo_updated(
                todo_id.clone(),
                Some("更新されたタイトル".to_string()),
                None,
                "user123".to_string(),
            );

            let result = service.update_todo_with_lock(
                family_id,
                &todo_id,
                initial_todo.version,
                update_event,
            ).await;

            assert!(result.is_ok());
            let updated_todo = result.unwrap();
            assert_eq!(updated_todo.title, "更新されたタイトル");
            assert_eq!(updated_todo.version, initial_todo.version + 1);
        }
    }

    #[tokio::test]
    async fn test_optimistic_lock_version_conflict() {
        let service = setup_test_service().await;
        let family_id = "test_family";
        let todo_id = TodoId::new();

        // 初期ToDoを作成
        let create_event = TodoEvent::new_todo_created(
            todo_id.clone(),
            "初期タイトル".to_string(),
            None,
            vec![],
            "user123".to_string(),
        );

        let initial_todo = Todo::from_created_event(&create_event).unwrap();
        
        // 実際のDynamoDB Localが動いていない場合はスキップ
        if service.projection_repo.save_projection(family_id, initial_todo.clone()).await.is_ok() {
            // 間違ったバージョンで更新を試行
            let update_event = TodoEvent::new_todo_updated(
                todo_id.clone(),
                Some("更新されたタイトル".to_string()),
                None,
                "user123".to_string(),
            );

            let result = service.update_todo_with_lock(
                family_id,
                &todo_id,
                initial_todo.version + 1, // 間違ったバージョン
                update_event,
            ).await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), TodoError::ConcurrentModification));
        }
    }

    #[tokio::test]
    async fn test_retry_with_concurrent_updates() {
        let service = Arc::new(setup_test_service().await);
        let family_id = "test_family";
        let todo_id = TodoId::new();

        // 初期ToDoを作成
        let create_event = TodoEvent::new_todo_created(
            todo_id.clone(),
            "初期タイトル".to_string(),
            None,
            vec![],
            "user123".to_string(),
        );

        let initial_todo = Todo::from_created_event(&create_event).unwrap();
        
        // 実際のDynamoDB Localが動いていない場合はスキップ
        if service.projection_repo.save_projection(family_id, initial_todo.clone()).await.is_ok() {
            // 同時更新をシミュレート
            let service1 = service.clone();
            let service2 = service.clone();
            let todo_id1 = todo_id.clone();
            let todo_id2 = todo_id.clone();

            let handle1 = tokio::spawn(async move {
                service1.update_todo_with_retry(
                    family_id,
                    &todo_id1,
                    3,
                    |todo| helpers::create_title_update_event(todo, "更新1".to_string(), "user1".to_string()),
                ).await
            });

            let handle2 = tokio::spawn(async move {
                service2.update_todo_with_retry(
                    family_id,
                    &todo_id2,
                    3,
                    |todo| helpers::create_title_update_event(todo, "更新2".to_string(), "user2".to_string()),
                ).await
            });

            let (result1, result2) = tokio::join!(handle1, handle2);
            
            // 少なくとも一つは成功するはず
            assert!(result1.is_ok() || result2.is_ok());
        }
    }

    #[test]
    fn test_event_generation_helpers() {
        let todo_id = TodoId::new();
        let create_event = TodoEvent::new_todo_created(
            todo_id,
            "テストToDo".to_string(),
            None,
            vec![],
            "user123".to_string(),
        );
        let todo = Todo::from_created_event(&create_event).unwrap();

        // タイトル更新イベント生成
        let title_event = helpers::create_title_update_event(
            &todo,
            "新しいタイトル".to_string(),
            "user456".to_string(),
        );
        assert!(title_event.is_ok());

        // 空のタイトルでエラー
        let invalid_title_event = helpers::create_title_update_event(
            &todo,
            "".to_string(),
            "user456".to_string(),
        );
        assert!(invalid_title_event.is_err());

        // 完了イベント生成
        let completion_event = helpers::create_completion_event(&todo, "user789".to_string());
        assert!(completion_event.is_ok());

        // 削除イベント生成
        let deletion_event = helpers::create_deletion_event(
            &todo,
            "user999".to_string(),
            Some("不要になった".to_string()),
        );
        assert!(deletion_event.is_ok());
    }
}
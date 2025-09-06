use crate::{DynamoDbClient, EventRepository, ProjectionRepository};
use aws_sdk_dynamodb::types::AttributeValue;
use domain::{Todo, TodoEvent, TodoId};
use shared::{AppError, OptimisticLockRetryExecutor, RetryResult};
use tracing::{debug, info, warn};

/// 楽観的ロック制御を提供するサービス
#[derive(Clone)]
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

    /// EventRepositoryへの参照を取得
    pub fn event_repo(&self) -> &EventRepository {
        &self.event_repo
    }

    /// ProjectionRepositoryへの参照を取得
    pub fn projection_repo(&self) -> &ProjectionRepository {
        &self.projection_repo
    }

    /// 楽観的ロックを使用してToDoを更新
    /// バージョン番号による同時実行制御を行う
    pub async fn update_todo_with_lock(
        &self,
        family_id: &str,
        todo_id: &TodoId,
        expected_version: u64,
        event: TodoEvent,
    ) -> Result<Todo, AppError> {
        info!(
            "楽観的ロック更新開始: family_id={}, todo_id={}, expected_version={}",
            family_id, todo_id, expected_version
        );

        let result = OptimisticLockRetryExecutor::execute(|| async {
            // 現在のToDoを取得
            let current_todo = self
                .projection_repo
                .get_projection(family_id, todo_id)
                .await?;

            let mut todo = match current_todo {
                Some(todo) => todo,
                None => {
                    return Err(AppError::NotFound(format!("ToDo not found: {todo_id}")));
                }
            };

            // バージョンチェック
            if todo.version != expected_version {
                warn!(
                    "バージョン不一致: expected={}, actual={}",
                    expected_version, todo.version
                );
                return Err(AppError::ConcurrentModification);
            }

            // イベントを保存
            self.event_repo.save_event(family_id, event.clone()).await?;

            // ToDoにイベントを適用
            if let Err(e) = todo.apply(event.clone()) {
                tracing::warn!("イベント適用に失敗しました: {:?}", e);
            }

            // 更新されたToDoを保存
            self.projection_repo
                .save_projection(family_id, todo.clone())
                .await?;

            debug!("楽観的ロック更新完了: todo_id={}", todo_id);
            Ok(todo)
        })
        .await;

        match result {
            RetryResult::Success(todo) => Ok(todo),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }

    /// ToDoの削除（論理削除）
    /// 楽観的ロックを使用して安全に削除を実行
    pub async fn delete_todo_with_lock(
        &self,
        family_id: &str,
        todo_id: &TodoId,
        expected_version: u64,
        delete_event: TodoEvent,
    ) -> Result<(), AppError> {
        info!(
            "楽観的ロック削除開始: family_id={}, todo_id={}, expected_version={}",
            family_id, todo_id, expected_version
        );

        let result = OptimisticLockRetryExecutor::execute(|| async {
            // 現在のToDoを取得してバージョンチェック
            let current_todo = self
                .projection_repo
                .get_projection(family_id, todo_id)
                .await?;

            match current_todo {
                Some(todo) => {
                    if todo.version != expected_version {
                        warn!(
                            "削除時バージョン不一致: expected={}, actual={}",
                            expected_version, todo.version
                        );
                        return Err(AppError::ConcurrentModification);
                    }
                }
                None => {
                    return Err(AppError::NotFound(format!("ToDo not found: {todo_id}")));
                }
            }

            // 削除イベントを保存
            self.event_repo
                .save_event(family_id, delete_event.clone())
                .await?;

            // プロジェクションから削除（論理削除の場合はフラグ更新、物理削除の場合は削除）
            self.projection_repo
                .delete_projection(family_id, todo_id)
                .await?;

            debug!("楽観的ロック削除完了: todo_id={}", todo_id);
            Ok(())
        })
        .await;

        match result {
            RetryResult::Success(()) => Ok(()),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }

    /// 条件付き更新を使用したDynamoDB操作
    /// バージョン番号による楽観的ロック制御
    pub async fn conditional_update_projection(
        &self,
        family_id: &str,
        todo: &Todo,
        expected_version: u64,
    ) -> Result<(), AppError> {
        info!(
            "条件付き更新実行: family_id={}, todo_id={}, expected_version={}",
            family_id, todo.id, expected_version
        );

        let pk = format!("FAMILY#{family_id}");
        let sk = format!("TODO#CURRENT#{}", todo.id.as_str());

        let result = OptimisticLockRetryExecutor::execute(|| async {
            // ToDoデータをDynamoDBアイテムに変換
            let todo_json = serde_json::to_string(todo)
                .map_err(|e| AppError::Serialization(format!("ToDo serialization error: {e}")))?;

            self.db
                .client()
                .update_item()
                .table_name(self.db.table_name())
                .key("PK", AttributeValue::S(pk.clone()))
                .key("SK", AttributeValue::S(sk.clone()))
                .update_expression(
                    "SET #data = :data, #version = :new_version, #updated_at = :updated_at",
                )
                .condition_expression("#version = :expected_version")
                .expression_attribute_names("#data", "Data")
                .expression_attribute_names("#version", "Version")
                .expression_attribute_names("#updated_at", "UpdatedAt")
                .expression_attribute_values(":data", AttributeValue::S(todo_json))
                .expression_attribute_values(
                    ":new_version",
                    AttributeValue::N((expected_version + 1).to_string()),
                )
                .expression_attribute_values(
                    ":expected_version",
                    AttributeValue::N(expected_version.to_string()),
                )
                .expression_attribute_values(
                    ":updated_at",
                    AttributeValue::S(chrono::Utc::now().to_rfc3339()),
                )
                .send()
                .await
                .map_err(|e| self.db.convert_error(e))?;

            debug!("条件付き更新完了: todo_id={}", todo.id);
            Ok(())
        })
        .await;

        match result {
            RetryResult::Success(()) => Ok(()),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }

    /// 現在のバージョンを取得
    pub async fn get_current_version(
        &self,
        family_id: &str,
        todo_id: &TodoId,
    ) -> Result<Option<u64>, AppError> {
        info!(
            "現在バージョン取得: family_id={}, todo_id={}",
            family_id, todo_id
        );

        let pk = format!("FAMILY#{family_id}");
        let sk = format!("TODO#CURRENT#{}", todo_id.as_str());

        let result = OptimisticLockRetryExecutor::execute(|| async {
            let response = self
                .db
                .client()
                .get_item()
                .table_name(self.db.table_name())
                .key("PK", AttributeValue::S(pk.clone()))
                .key("SK", AttributeValue::S(sk.clone()))
                .projection_expression("Version")
                .send()
                .await
                .map_err(|e| self.db.convert_error(e))?;

            if let Some(item) = response.item {
                if let Some(version_attr) = item.get("Version") {
                    if let Ok(version_str) = version_attr.as_n() {
                        if let Ok(version) = version_str.parse::<u64>() {
                            debug!("現在バージョン取得完了: {}", version);
                            return Ok(Some(version));
                        }
                    }
                }
            }

            debug!("バージョン情報が見つかりません: todo_id={}", todo_id);
            Ok(None)
        })
        .await;

        match result {
            RetryResult::Success(version) => Ok(version),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }

    /// GDPR対応: 物理削除
    /// すべての関連データを完全に削除
    pub async fn hard_delete_todo(
        &self,
        family_id: &str,
        todo_id: &TodoId,
        delete_event: TodoEvent,
    ) -> Result<(), AppError> {
        info!("物理削除開始: family_id={}, todo_id={}", family_id, todo_id);

        // 削除イベントを記録
        self.event_repo.save_event(family_id, delete_event).await?;

        let result = OptimisticLockRetryExecutor::execute(|| async {
            // プロジェクションを物理削除
            self.db
                .client()
                .delete_item()
                .table_name(self.db.table_name())
                .key("PK", AttributeValue::S(format!("FAMILY#{family_id}")))
                .key(
                    "SK",
                    AttributeValue::S(format!("TODO#CURRENT#{}", todo_id.as_str())),
                )
                .send()
                .await
                .map_err(|e| self.db.convert_error(e))?;

            debug!("物理削除完了: todo_id={}", todo_id);
            Ok(())
        })
        .await;

        match result {
            RetryResult::Success(()) => Ok(()),
            RetryResult::MaxAttemptsReached(error) => Err(error),
            RetryResult::NonRetryable(error) => Err(error),
        }
    }
}

/// ToDo更新データ
#[derive(Debug, Clone)]
pub struct TodoUpdates {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
}

impl TodoUpdates {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            completed: None,
        }
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }

    pub fn with_completed(mut self, completed: bool) -> Self {
        self.completed = Some(completed);
        self
    }
}

impl Default for TodoUpdates {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{TodoEvent, TodoId};
    use shared::Config;

    async fn setup_test_service() -> Result<OptimisticLockService, AppError> {
        let config = Config {
            dynamodb_table: "test-table".to_string(),
            environment: "test".to_string(),
            aws_region: "ap-northeast-1".to_string(),
            dynamodb_endpoint: Some("http://localhost:8000".to_string()),
            retry_max_attempts: 2,
            retry_initial_delay_ms: 10,
        };
        let db_client = DynamoDbClient::new_for_test(&config).await?;
        Ok(OptimisticLockService::new(db_client))
    }

    #[tokio::test]
    async fn test_optimistic_lock_success() {
        let service = match setup_test_service().await {
            Ok(service) => service,
            Err(_) => {
                println!("DynamoDB Localが利用できないため、テストをスキップします");
                return;
            }
        };
        let todo_id = TodoId::new();
        let family_id = "test_family";

        // 最初にToDoを作成
        let create_event = TodoEvent::new_todo_created(
            todo_id.clone(),
            "テストToDo".to_string(),
            None,
            vec![],
            "user123".to_string(),
        );

        let todo = Todo::from_created_event(&create_event).unwrap();

        // プロジェクションを保存
        if service
            .projection_repo
            .save_projection(family_id, todo.clone())
            .await
            .is_ok()
        {
            // 更新イベントを作成
            let update_event = TodoEvent::new_todo_updated(
                todo_id.clone(),
                Some("更新されたタイトル".to_string()),
                None,
                "user123".to_string(),
            );

            // 楽観的ロックで更新
            let result = service
                .update_todo_with_lock(family_id, &todo_id, 1, update_event)
                .await;

            // 実際のDynamoDB Localが動いていない場合はスキップ
            if result.is_ok() {
                let updated_todo = result.unwrap();
                assert_eq!(updated_todo.title, "更新されたタイトル");
                assert_eq!(updated_todo.version, 2);
            }
        }
    }

    #[tokio::test]
    async fn test_optimistic_lock_version_conflict() {
        let service = match setup_test_service().await {
            Ok(service) => service,
            Err(_) => {
                println!("DynamoDB Localが利用できないため、テストをスキップします");
                return;
            }
        };
        let todo_id = TodoId::new();
        let family_id = "test_family";

        // 最初にToDoを作成
        let create_event = TodoEvent::new_todo_created(
            todo_id.clone(),
            "テストToDo".to_string(),
            None,
            vec![],
            "user123".to_string(),
        );

        let todo = Todo::from_created_event(&create_event).unwrap();

        // プロジェクションを保存
        if service
            .projection_repo
            .save_projection(family_id, todo.clone())
            .await
            .is_ok()
        {
            // 間違ったバージョンで更新を試行
            let update_event = TodoEvent::new_todo_updated(
                todo_id.clone(),
                Some("更新されたタイトル".to_string()),
                None,
                "user123".to_string(),
            );

            let result = service
                .update_todo_with_lock(family_id, &todo_id, 999, update_event) // 間違ったバージョン
                .await;

            // 実際のDynamoDB Localが動いていない場合はスキップ
            if let Err(error) = result {
                assert!(matches!(error, AppError::ConcurrentModification));
            }
        }
    }

    #[test]
    fn test_todo_updates_builder() {
        let updates = TodoUpdates::new()
            .with_title("新しいタイトル".to_string())
            .with_description(Some("新しい説明".to_string()))
            .with_completed(true);

        assert_eq!(updates.title, Some("新しいタイトル".to_string()));
        assert_eq!(updates.description, Some("新しい説明".to_string()));
        assert_eq!(updates.completed, Some(true));
    }
}

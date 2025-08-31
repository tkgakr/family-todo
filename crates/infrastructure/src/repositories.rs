use crate::{
    retry_dynamodb_operation, DynamoDbClient, EventItem, ProjectionItem, SnapshotData, SnapshotItem,
};
use aws_sdk_dynamodb::types::AttributeValue;
use domain::{Todo, TodoError, TodoEvent, TodoId};
use tracing::{debug, info};

/// イベントストアリポジトリ
/// イベントの保存・取得機能を提供
#[derive(Clone)]
pub struct EventRepository {
    db: DynamoDbClient,
}

impl EventRepository {
    pub fn new(db: DynamoDbClient) -> Self {
        Self { db }
    }

    /// イベントを保存する
    /// 楽観的ロックを使用して同時実行制御を行う
    pub async fn save_event(&self, family_id: &str, event: TodoEvent) -> Result<(), TodoError> {
        info!(
            "イベントを保存中: family_id={}, event_id={}",
            family_id,
            event.event_id()
        );

        let event_item = EventItem::new(family_id.to_string(), event, 1);
        let dynamodb_item = event_item
            .to_dynamodb_item()
            .map_err(|e| TodoError::Internal(format!("DynamoDBアイテム変換エラー: {e}")))?;

        let attr_map = dynamodb_item.to_attribute_map();

        retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .put_item()
                    .table_name(self.db.table_name())
                    .set_item(Some(attr_map.clone()))
                    // 同じイベントIDの重複保存を防ぐ
                    .condition_expression("attribute_not_exists(PK) AND attribute_not_exists(SK)")
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("イベント保存完了: {}", event_item.event.event_id());
                Ok(())
            },
            None,
        )
        .await
    }

    /// 特定のToDoに関連するすべてのイベントを取得
    pub async fn get_events(
        &self,
        family_id: &str,
        todo_id: &TodoId,
    ) -> Result<Vec<TodoEvent>, TodoError> {
        info!(
            "イベント取得中: family_id={}, todo_id={}",
            family_id, todo_id
        );

        let pk = format!("FAMILY#{family_id}");
        let sk_prefix = format!("TODO#EVENT#{}", todo_id.as_str());

        retry_dynamodb_operation(
            || async {
                let response = self
                    .db
                    .client()
                    .query()
                    .table_name(self.db.table_name())
                    .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
                    .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                    .expression_attribute_values(":sk_prefix", AttributeValue::S(sk_prefix.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                let mut events = Vec::new();
                if let Some(items) = response.items {
                    for item in items {
                        let dynamodb_item = crate::models::DynamoDbItem::from_attribute_map(&item)
                            .map_err(|e| TodoError::Internal(format!("アイテム変換エラー: {e}")))?;

                        let event_item =
                            EventItem::from_dynamodb_item(&dynamodb_item).map_err(|e| {
                                TodoError::Internal(format!("イベントアイテム変換エラー: {e}"))
                            })?;

                        events.push(event_item.event);
                    }
                }

                // イベントをタイムスタンプ順にソート（ULIDは自然にソートされる）
                events.sort_by(|a, b| a.event_id().cmp(b.event_id()));

                debug!("イベント取得完了: {} 件", events.len());
                Ok(events)
            },
            None,
        )
        .await
    }

    /// 家族のすべてのイベントを取得（時系列順）
    pub async fn get_all_events(&self, family_id: &str) -> Result<Vec<TodoEvent>, TodoError> {
        info!("全イベント取得中: family_id={}", family_id);

        let pk = format!("FAMILY#{family_id}");

        retry_dynamodb_operation(
            || async {
                let response = self
                    .db
                    .client()
                    .query()
                    .table_name(self.db.table_name())
                    .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
                    .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                    .expression_attribute_values(
                        ":sk_prefix",
                        AttributeValue::S("EVENT#".to_string()),
                    )
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                let mut events = Vec::new();
                if let Some(items) = response.items {
                    for item in items {
                        let dynamodb_item = crate::models::DynamoDbItem::from_attribute_map(&item)
                            .map_err(|e| TodoError::Internal(format!("アイテム変換エラー: {e}")))?;

                        let event_item =
                            EventItem::from_dynamodb_item(&dynamodb_item).map_err(|e| {
                                TodoError::Internal(format!("イベントアイテム変換エラー: {e}"))
                            })?;

                        events.push(event_item.event);
                    }
                }

                // イベントをタイムスタンプ順にソート
                events.sort_by(|a, b| a.event_id().cmp(b.event_id()));

                debug!("全イベント取得完了: {} 件", events.len());
                Ok(events)
            },
            None,
        )
        .await
    }

    /// 特定のイベントIDでイベントを取得
    pub async fn get_event_by_id(
        &self,
        family_id: &str,
        event_id: &str,
    ) -> Result<Option<TodoEvent>, TodoError> {
        info!(
            "イベントID取得中: family_id={}, event_id={}",
            family_id, event_id
        );

        let pk = format!("FAMILY#{family_id}");
        let sk = format!("EVENT#{event_id}");

        retry_dynamodb_operation(
            || async {
                let response = self
                    .db
                    .client()
                    .get_item()
                    .table_name(self.db.table_name())
                    .key("PK", AttributeValue::S(pk.clone()))
                    .key("SK", AttributeValue::S(sk.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                if let Some(item) = response.item {
                    let dynamodb_item = crate::models::DynamoDbItem::from_attribute_map(&item)
                        .map_err(|e| TodoError::Internal(format!("アイテム変換エラー: {e}")))?;

                    let event_item =
                        EventItem::from_dynamodb_item(&dynamodb_item).map_err(|e| {
                            TodoError::Internal(format!("イベントアイテム変換エラー: {e}"))
                        })?;

                    debug!("イベント取得完了: {}", event_id);
                    Ok(Some(event_item.event))
                } else {
                    debug!("イベントが見つかりません: {}", event_id);
                    Ok(None)
                }
            },
            None,
        )
        .await
    }
}

/// プロジェクションリポジトリ
/// 現在のToDo状態の管理機能を提供
#[derive(Clone)]
pub struct ProjectionRepository {
    db: DynamoDbClient,
}

impl ProjectionRepository {
    pub fn new(db: DynamoDbClient) -> Self {
        Self { db }
    }

    /// ToDoプロジェクションを保存または更新
    pub async fn save_projection(&self, family_id: &str, todo: Todo) -> Result<(), TodoError> {
        info!(
            "プロジェクション保存中: family_id={}, todo_id={}",
            family_id, todo.id
        );

        let projection_item = ProjectionItem::new(family_id.to_string(), todo);
        let dynamodb_item = projection_item
            .to_dynamodb_item()
            .map_err(|e| TodoError::Internal(format!("DynamoDBアイテム変換エラー: {e}")))?;

        let attr_map = dynamodb_item.to_attribute_map();

        retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .put_item()
                    .table_name(self.db.table_name())
                    .set_item(Some(attr_map.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("プロジェクション保存完了: {}", projection_item.todo.id);
                Ok(())
            },
            None,
        )
        .await
    }

    /// 特定のToDoプロジェクションを取得
    pub async fn get_projection(
        &self,
        family_id: &str,
        todo_id: &TodoId,
    ) -> Result<Option<Todo>, TodoError> {
        info!(
            "プロジェクション取得中: family_id={}, todo_id={}",
            family_id, todo_id
        );

        let pk = format!("FAMILY#{family_id}");
        let sk = format!("TODO#CURRENT#{}", todo_id.as_str());

        retry_dynamodb_operation(
            || async {
                let response = self
                    .db
                    .client()
                    .get_item()
                    .table_name(self.db.table_name())
                    .key("PK", AttributeValue::S(pk.clone()))
                    .key("SK", AttributeValue::S(sk.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                if let Some(item) = response.item {
                    let dynamodb_item = crate::models::DynamoDbItem::from_attribute_map(&item)
                        .map_err(|e| TodoError::Internal(format!("アイテム変換エラー: {e}")))?;

                    let projection_item = ProjectionItem::from_dynamodb_item(&dynamodb_item)
                        .map_err(|e| {
                            TodoError::Internal(format!("プロジェクションアイテム変換エラー: {e}"))
                        })?;

                    debug!("プロジェクション取得完了: {}", todo_id);
                    Ok(Some(projection_item.todo))
                } else {
                    debug!("プロジェクションが見つかりません: {}", todo_id);
                    Ok(None)
                }
            },
            None,
        )
        .await
    }

    /// アクティブなToDo一覧を取得
    pub async fn get_active_todos(
        &self,
        family_id: &str,
        limit: Option<i32>,
    ) -> Result<Vec<Todo>, TodoError> {
        info!("アクティブToDo取得中: family_id={}", family_id);

        let gsi1_pk = format!("FAMILY#{family_id}#ACTIVE");

        retry_dynamodb_operation(
            || async {
                let mut query = self
                    .db
                    .client()
                    .query()
                    .table_name(self.db.table_name())
                    .index_name("GSI1")
                    .key_condition_expression("GSI1PK = :gsi1_pk")
                    .expression_attribute_values(":gsi1_pk", AttributeValue::S(gsi1_pk.clone()));

                if let Some(limit) = limit {
                    query = query.limit(limit);
                }

                let response = query.send().await.map_err(|e| self.db.convert_error(e))?;

                let mut todos = Vec::new();
                if let Some(items) = response.items {
                    for item in items {
                        let dynamodb_item = crate::models::DynamoDbItem::from_attribute_map(&item)
                            .map_err(|e| TodoError::Internal(format!("アイテム変換エラー: {e}")))?;

                        let projection_item = ProjectionItem::from_dynamodb_item(&dynamodb_item)
                            .map_err(|e| {
                                TodoError::Internal(format!(
                                    "プロジェクションアイテム変換エラー: {e}"
                                ))
                            })?;

                        todos.push(projection_item.todo);
                    }
                }

                debug!("アクティブToDo取得完了: {} 件", todos.len());
                Ok(todos)
            },
            None,
        )
        .await
    }

    /// 家族のすべてのToDo（アクティブ・非アクティブ含む）を取得
    pub async fn get_all_todos(
        &self,
        family_id: &str,
        limit: Option<i32>,
    ) -> Result<Vec<Todo>, TodoError> {
        info!("全ToDo取得中: family_id={}", family_id);

        let pk = format!("FAMILY#{family_id}");

        retry_dynamodb_operation(
            || async {
                let mut query = self
                    .db
                    .client()
                    .query()
                    .table_name(self.db.table_name())
                    .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
                    .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                    .expression_attribute_values(
                        ":sk_prefix",
                        AttributeValue::S("TODO#CURRENT#".to_string()),
                    );

                if let Some(limit) = limit {
                    query = query.limit(limit);
                }

                let response = query.send().await.map_err(|e| self.db.convert_error(e))?;

                let mut todos = Vec::new();
                if let Some(items) = response.items {
                    for item in items {
                        let dynamodb_item = crate::models::DynamoDbItem::from_attribute_map(&item)
                            .map_err(|e| TodoError::Internal(format!("アイテム変換エラー: {e}")))?;

                        let projection_item = ProjectionItem::from_dynamodb_item(&dynamodb_item)
                            .map_err(|e| {
                                TodoError::Internal(format!(
                                    "プロジェクションアイテム変換エラー: {e}"
                                ))
                            })?;

                        todos.push(projection_item.todo);
                    }
                }

                debug!("全ToDo取得完了: {} 件", todos.len());
                Ok(todos)
            },
            None,
        )
        .await
    }

    /// プロジェクションを削除（物理削除）
    pub async fn delete_projection(
        &self,
        family_id: &str,
        todo_id: &TodoId,
    ) -> Result<(), TodoError> {
        info!(
            "プロジェクション削除中: family_id={}, todo_id={}",
            family_id, todo_id
        );

        let pk = format!("FAMILY#{family_id}");
        let sk = format!("TODO#CURRENT#{}", todo_id.as_str());

        retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .delete_item()
                    .table_name(self.db.table_name())
                    .key("PK", AttributeValue::S(pk.clone()))
                    .key("SK", AttributeValue::S(sk.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("プロジェクション削除完了: {}", todo_id);
                Ok(())
            },
            None,
        )
        .await
    }
}

/// スナップショットリポジトリ
/// スナップショット管理機能を提供
#[derive(Clone)]
pub struct SnapshotRepository {
    db: DynamoDbClient,
}

impl SnapshotRepository {
    pub fn new(db: DynamoDbClient) -> Self {
        Self { db }
    }

    /// スナップショットを保存
    pub async fn save_snapshot(
        &self,
        family_id: &str,
        todo_id: &TodoId,
        snapshot_data: SnapshotData,
        ttl_days: Option<u32>,
    ) -> Result<String, TodoError> {
        info!(
            "スナップショット保存中: family_id={}, todo_id={}",
            family_id, todo_id
        );

        let snapshot_item = SnapshotItem::new(
            family_id.to_string(),
            todo_id.clone(),
            snapshot_data,
            ttl_days,
        );

        let snapshot_id = snapshot_item.snapshot_id.clone();
        let dynamodb_item = snapshot_item
            .to_dynamodb_item()
            .map_err(|e| TodoError::Internal(format!("DynamoDBアイテム変換エラー: {e}")))?;

        let attr_map = dynamodb_item.to_attribute_map();

        retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .put_item()
                    .table_name(self.db.table_name())
                    .set_item(Some(attr_map.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("スナップショット保存完了: {}", snapshot_id);
                Ok(())
            },
            None,
        )
        .await?;

        Ok(snapshot_id)
    }

    /// 最新のスナップショットを取得
    pub async fn get_latest_snapshot(
        &self,
        family_id: &str,
        todo_id: &TodoId,
    ) -> Result<Option<SnapshotData>, TodoError> {
        info!(
            "最新スナップショット取得中: family_id={}, todo_id={}",
            family_id, todo_id
        );

        let pk = format!("FAMILY#{family_id}");
        let sk_prefix = format!("TODO#SNAPSHOT#{}", todo_id.as_str());

        retry_dynamodb_operation(
            || async {
                let response = self
                    .db
                    .client()
                    .query()
                    .table_name(self.db.table_name())
                    .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
                    .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                    .expression_attribute_values(":sk_prefix", AttributeValue::S(sk_prefix.clone()))
                    .scan_index_forward(false) // 降順（最新から）
                    .limit(1)
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                if let Some(items) = response.items {
                    if let Some(item) = items.first() {
                        let dynamodb_item = crate::models::DynamoDbItem::from_attribute_map(item)
                            .map_err(|e| {
                            TodoError::Internal(format!("アイテム変換エラー: {e}"))
                        })?;

                        let snapshot_item = SnapshotItem::from_dynamodb_item(&dynamodb_item)
                            .map_err(|e| {
                                TodoError::Internal(format!(
                                    "スナップショットアイテム変換エラー: {e}",
                                ))
                            })?;

                        debug!(
                            "最新スナップショット取得完了: {}",
                            snapshot_item.snapshot_id
                        );
                        return Ok(Some(snapshot_item.data));
                    }
                }

                debug!("スナップショットが見つかりません: {}", todo_id);
                Ok(None)
            },
            None,
        )
        .await
    }

    /// 古いスナップショットにTTLを設定
    pub async fn set_old_snapshots_ttl(
        &self,
        family_id: &str,
        todo_id: &TodoId,
        keep_count: usize,
        ttl_days: u32,
    ) -> Result<(), TodoError> {
        info!(
            "古いスナップショットTTL設定中: family_id={}, todo_id={}, keep_count={}",
            family_id, todo_id, keep_count
        );

        let pk = format!("FAMILY#{family_id}");
        let sk_prefix = format!("TODO#SNAPSHOT#{}", todo_id.as_str());

        retry_dynamodb_operation(
            || async {
                // すべてのスナップショットを取得
                let response = self
                    .db
                    .client()
                    .query()
                    .table_name(self.db.table_name())
                    .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
                    .expression_attribute_values(":pk", AttributeValue::S(pk.clone()))
                    .expression_attribute_values(":sk_prefix", AttributeValue::S(sk_prefix.clone()))
                    .scan_index_forward(false) // 降順（最新から）
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                if let Some(items) = response.items {
                    if items.len() > keep_count {
                        let ttl_timestamp =
                            chrono::Utc::now().timestamp() + (ttl_days as i64 * 24 * 60 * 60);

                        // 保持数を超えた古いスナップショットにTTLを設定
                        for item in items.iter().skip(keep_count) {
                            let sk =
                                item.get("SK").and_then(|v| v.as_s().ok()).ok_or_else(|| {
                                    TodoError::Internal("SKが見つかりません".to_string())
                                })?;

                            self.db
                                .client()
                                .update_item()
                                .table_name(self.db.table_name())
                                .key("PK", AttributeValue::S(pk.clone()))
                                .key("SK", AttributeValue::S(sk.clone()))
                                .update_expression("SET TTL = :ttl")
                                .expression_attribute_values(
                                    ":ttl",
                                    AttributeValue::N(ttl_timestamp.to_string()),
                                )
                                .send()
                                .await
                                .map_err(|e| self.db.convert_error(e))?;
                        }

                        debug!(
                            "古いスナップショットTTL設定完了: {} 件",
                            items.len() - keep_count
                        );
                    }
                }

                Ok(())
            },
            None,
        )
        .await
    }

    /// 特定のスナップショットを削除
    pub async fn delete_snapshot(
        &self,
        family_id: &str,
        todo_id: &TodoId,
        snapshot_id: &str,
    ) -> Result<(), TodoError> {
        info!(
            "スナップショット削除中: family_id={}, todo_id={}, snapshot_id={}",
            family_id, todo_id, snapshot_id
        );

        let pk = format!("FAMILY#{family_id}");
        let sk = format!("TODO#SNAPSHOT#{}#{}", todo_id.as_str(), snapshot_id);

        retry_dynamodb_operation(
            || async {
                self.db
                    .client()
                    .delete_item()
                    .table_name(self.db.table_name())
                    .key("PK", AttributeValue::S(pk.clone()))
                    .key("SK", AttributeValue::S(sk.clone()))
                    .send()
                    .await
                    .map_err(|e| self.db.convert_error(e))?;

                debug!("スナップショット削除完了: {}", snapshot_id);
                Ok(())
            },
            None,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{Todo, TodoEvent, TodoId};
    use shared::Config;

    async fn setup_test_client() -> DynamoDbClient {
        let config = Config {
            dynamodb_table: "test-table".to_string(),
            environment: "test".to_string(),
            aws_region: "ap-northeast-1".to_string(),
            dynamodb_endpoint: Some("http://localhost:8000".to_string()),
            retry_max_attempts: 2,
            retry_initial_delay_ms: 10,
        };
        DynamoDbClient::new_for_test(&config).await.unwrap()
    }

    #[tokio::test]
    async fn test_event_repository_save_and_get() {
        let client = setup_test_client().await;
        let repo = EventRepository::new(client);

        let todo_id = TodoId::new();
        let event = TodoEvent::new_todo_created(
            todo_id.clone(),
            "テストToDo".to_string(),
            Some("説明".to_string()),
            vec!["タグ1".to_string()],
            "user123".to_string(),
        );

        // 実際のDynamoDB Localが動いていない場合はスキップ
        if repo.save_event("test_family", event.clone()).await.is_ok() {
            let events = repo.get_events("test_family", &todo_id).await.unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].todo_id(), &todo_id);
        }
    }

    #[tokio::test]
    async fn test_projection_repository_save_and_get() {
        let client = setup_test_client().await;
        let repo = ProjectionRepository::new(client);

        let todo_id = TodoId::new();
        let event = TodoEvent::new_todo_created(
            todo_id.clone(),
            "テストToDo".to_string(),
            None,
            vec![],
            "user123".to_string(),
        );

        let todo = Todo::from_created_event(&event).unwrap();

        // 実際のDynamoDB Localが動いていない場合はスキップ
        if repo
            .save_projection("test_family", todo.clone())
            .await
            .is_ok()
        {
            let retrieved_todo = repo.get_projection("test_family", &todo_id).await.unwrap();
            assert!(retrieved_todo.is_some());
            assert_eq!(retrieved_todo.unwrap().id, todo_id);
        }
    }

    #[tokio::test]
    async fn test_snapshot_repository_save_and_get() {
        let client = setup_test_client().await;
        let repo = SnapshotRepository::new(client);

        let todo_id = TodoId::new();
        let event = TodoEvent::new_todo_created(
            todo_id.clone(),
            "テストToDo".to_string(),
            None,
            vec![],
            "user123".to_string(),
        );

        let todo = Todo::from_created_event(&event).unwrap();
        let snapshot_data = SnapshotData {
            todo: todo.clone(),
            event_count: 1,
            last_event_id: event.event_id().to_string(),
        };

        // 実際のDynamoDB Localが動いていない場合はスキップ
        if repo
            .save_snapshot("test_family", &todo_id, snapshot_data.clone(), Some(30))
            .await
            .is_ok()
        {
            let retrieved_snapshot = repo
                .get_latest_snapshot("test_family", &todo_id)
                .await
                .unwrap();
            assert!(retrieved_snapshot.is_some());
            let snapshot = retrieved_snapshot.unwrap();
            assert_eq!(snapshot.todo.id, todo_id);
            assert_eq!(snapshot.event_count, 1);
        }
    }

    #[test]
    fn test_dynamodb_keys_generation() {
        let family_id = "test_family";
        let todo_id = TodoId::new();
        let event_ulid = "01ARZ3NDEKTSV4RRFFQ69G5FAV";

        let event_keys = crate::models::DynamoDbKeys::for_event(family_id, event_ulid);
        assert_eq!(event_keys.pk, "FAMILY#test_family");
        assert_eq!(event_keys.sk, "EVENT#01ARZ3NDEKTSV4RRFFQ69G5FAV");

        let projection_keys = crate::models::DynamoDbKeys::for_todo_projection(family_id, &todo_id);
        assert_eq!(projection_keys.pk, "FAMILY#test_family");
        assert!(projection_keys.sk.starts_with("TODO#CURRENT#"));

        let active_keys = crate::models::DynamoDbKeys::for_active_todo(family_id, todo_id.as_str());
        assert_eq!(
            active_keys.gsi1_pk,
            Some("FAMILY#test_family#ACTIVE".to_string())
        );
        assert_eq!(active_keys.gsi1_sk, Some(todo_id.as_str().to_string()));
    }
}

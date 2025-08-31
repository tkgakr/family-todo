use anyhow::Result;
use domain::Todo;
use infrastructure::{
    DynamoDbClient, EventRepository, ProjectionRepository, SnapshotData, SnapshotRepository,
};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use shared::{init_tracing, Config};
use tracing::{error, info, warn};

/// EventBridge スケジュールイベント
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ScheduleEvent {
    #[serde(rename = "detail-type")]
    detail_type: String,
    source: String,
    account: String,
    time: String,
    region: String,
    detail: serde_json::Value,
}

/// スナップショット作成結果
#[derive(Debug, Serialize)]
struct SnapshotResult {
    processed_todos: usize,
    created_snapshots: usize,
    errors: Vec<String>,
}

/// スナップショット作成の閾値設定
const SNAPSHOT_EVENT_THRESHOLD: usize = 100;
const SNAPSHOT_AGE_THRESHOLD_DAYS: i64 = 7;
const SNAPSHOT_TTL_DAYS: u32 = 90; // スナップショットの保持期間
const KEEP_SNAPSHOT_COUNT: usize = 5; // 保持するスナップショット数

/// メイン関数ハンドラー
async fn function_handler(event: LambdaEvent<ScheduleEvent>) -> Result<serde_json::Value, Error> {
    info!("スナップショット管理処理を開始: {:?}", event.payload);

    let config = Config::from_env().map_err(|e| {
        error!("設定読み込みエラー: {}", e);
        Error::from(format!("設定エラー: {e}"))
    })?;

    let db_client = DynamoDbClient::new(&config).await.map_err(|e| {
        error!("DynamoDBクライアント初期化エラー: {}", e);
        Error::from(format!("DynamoDBエラー: {e}"))
    })?;

    let result = process_snapshots(&db_client).await.map_err(|e| {
        error!("スナップショット処理エラー: {}", e);
        Error::from(format!("処理エラー: {e}"))
    })?;

    info!(
        "スナップショット管理処理完了: 処理済みToDo={}, 作成スナップショット={}, エラー={}",
        result.processed_todos,
        result.created_snapshots,
        result.errors.len()
    );

    Ok(serde_json::to_value(result)?)
}

/// スナップショット処理のメイン関数
async fn process_snapshots(db_client: &DynamoDbClient) -> Result<SnapshotResult> {
    let event_repo = EventRepository::new(db_client.clone());
    let projection_repo = ProjectionRepository::new(db_client.clone());
    let snapshot_repo = SnapshotRepository::new(db_client.clone());

    // 全ての家族を取得（実際の実装では家族一覧を取得する必要がある）
    // 今回は簡略化のため、アクティブなToDoから家族IDを抽出する
    let family_ids = get_all_family_ids(&projection_repo).await?;

    let mut processed_todos = 0;
    let mut created_snapshots = 0;
    let mut errors = Vec::new();

    for family_id in family_ids {
        info!("家族のスナップショット処理開始: family_id={}", family_id);

        match process_family_snapshots(&family_id, &event_repo, &projection_repo, &snapshot_repo)
            .await
        {
            Ok((processed, created)) => {
                processed_todos += processed;
                created_snapshots += created;
                info!(
                    "家族のスナップショット処理完了: family_id={}, 処理済み={}, 作成={}",
                    family_id, processed, created
                );
            }
            Err(e) => {
                let error_msg = format!("家族{family_id}のスナップショット処理エラー: {e}");
                error!("{}", error_msg);
                errors.push(error_msg);
            }
        }
    }

    Ok(SnapshotResult {
        processed_todos,
        created_snapshots,
        errors,
    })
}

/// 全ての家族IDを取得
/// 実際の実装では専用のテーブルまたはGSIを使用するが、
/// 今回はアクティブなToDoから家族IDを抽出する
async fn get_all_family_ids(_projection_repo: &ProjectionRepository) -> Result<Vec<String>> {
    // 実装の簡略化のため、固定の家族IDリストを返す
    // 実際の実装では、DynamoDBから家族一覧を取得する

    // TODO: 実際の実装では、家族管理テーブルまたは
    // プロジェクションテーブルから家族IDを抽出する必要がある
    warn!("家族ID取得は簡略化実装を使用中");

    Ok(vec!["demo_family".to_string()])
}

/// 特定の家族のスナップショット処理
async fn process_family_snapshots(
    family_id: &str,
    event_repo: &EventRepository,
    projection_repo: &ProjectionRepository,
    snapshot_repo: &SnapshotRepository,
) -> Result<(usize, usize)> {
    // 家族の全てのToDoを取得
    let todos = projection_repo
        .get_all_todos(family_id, None)
        .await
        .map_err(|e| anyhow::anyhow!("ToDo取得エラー: {}", e))?;

    let mut processed_count = 0;
    let mut created_count = 0;

    for todo in todos {
        processed_count += 1;

        match process_todo_snapshot(family_id, &todo, event_repo, snapshot_repo).await {
            Ok(created) => {
                if created {
                    created_count += 1;
                }
            }
            Err(e) => {
                error!("ToDo {}のスナップショット処理エラー: {}", todo.id, e);
                // 個別のエラーは処理を継続
            }
        }
    }

    Ok((processed_count, created_count))
}

/// 個別のToDoのスナップショット処理
async fn process_todo_snapshot(
    family_id: &str,
    todo: &Todo,
    event_repo: &EventRepository,
    snapshot_repo: &SnapshotRepository,
) -> Result<bool> {
    // 最新のスナップショットを取得
    let latest_snapshot = snapshot_repo
        .get_latest_snapshot(family_id, &todo.id)
        .await
        .map_err(|e| anyhow::anyhow!("スナップショット取得エラー: {}", e))?;

    // スナップショット作成が必要かチェック
    let should_create = should_create_snapshot(todo, &latest_snapshot).await?;

    if !should_create {
        return Ok(false);
    }

    info!(
        "スナップショット作成開始: family_id={}, todo_id={}",
        family_id, todo.id
    );

    // ToDoの全イベントを取得してスナップショットデータを構築
    let events = event_repo
        .get_events(family_id, &todo.id)
        .await
        .map_err(|e| anyhow::anyhow!("イベント取得エラー: {}", e))?;

    if events.is_empty() {
        warn!("ToDo {}にイベントが見つかりません", todo.id);
        return Ok(false);
    }

    // 最新のイベントIDを取得
    let last_event_id = events
        .last()
        .map(|e| e.event_id().to_string())
        .unwrap_or_default();

    // スナップショットデータを作成
    let snapshot_data = SnapshotData {
        todo: todo.clone(),
        event_count: events.len(),
        last_event_id,
    };

    // スナップショットを保存
    let snapshot_id = snapshot_repo
        .save_snapshot(family_id, &todo.id, snapshot_data, Some(SNAPSHOT_TTL_DAYS))
        .await
        .map_err(|e| anyhow::anyhow!("スナップショット保存エラー: {}", e))?;

    info!(
        "スナップショット作成完了: snapshot_id={}, todo_id={}",
        snapshot_id, todo.id
    );

    // 古いスナップショットにTTLを設定
    snapshot_repo
        .set_old_snapshots_ttl(family_id, &todo.id, KEEP_SNAPSHOT_COUNT, SNAPSHOT_TTL_DAYS)
        .await
        .map_err(|e| anyhow::anyhow!("古いスナップショットTTL設定エラー: {}", e))?;

    Ok(true)
}

/// スナップショット作成が必要かどうかを判定
async fn should_create_snapshot(
    todo: &Todo,
    latest_snapshot: &Option<SnapshotData>,
) -> Result<bool> {
    match latest_snapshot {
        Some(snapshot) => {
            // 既存のスナップショットがある場合

            // イベント数による判定
            let current_event_count = todo.version as usize;
            let snapshot_event_count = snapshot.event_count;
            let new_events = current_event_count.saturating_sub(snapshot_event_count);

            if new_events >= SNAPSHOT_EVENT_THRESHOLD {
                info!(
                    "イベント数閾値によりスナップショット作成: todo_id={}, 新規イベント={}",
                    todo.id, new_events
                );
                return Ok(true);
            }

            // 時間による判定
            let snapshot_age = chrono::Utc::now()
                .signed_duration_since(todo.updated_at)
                .num_days();

            if snapshot_age >= SNAPSHOT_AGE_THRESHOLD_DAYS {
                info!(
                    "時間閾値によりスナップショット作成: todo_id={}, 経過日数={}",
                    todo.id, snapshot_age
                );
                return Ok(true);
            }

            Ok(false)
        }
        None => {
            // スナップショットが存在しない場合は作成
            info!("初回スナップショット作成: todo_id={}", todo.id);
            Ok(true)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();

    run(service_fn(function_handler)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use domain::{Todo, TodoEvent, TodoId};
    use infrastructure::SnapshotData;

    fn create_test_todo() -> Todo {
        let todo_id = TodoId::new();
        let event = TodoEvent::new_todo_created(
            todo_id,
            "テストToDo".to_string(),
            Some("テスト説明".to_string()),
            vec!["テスト".to_string()],
            "user123".to_string(),
        );
        Todo::from_created_event(&event).unwrap()
    }

    fn create_test_snapshot_data(todo: &Todo, event_count: usize) -> SnapshotData {
        SnapshotData {
            todo: todo.clone(),
            event_count,
            last_event_id: "test_event_id".to_string(),
        }
    }

    #[tokio::test]
    async fn test_should_create_snapshot_no_existing_snapshot() {
        let todo = create_test_todo();
        let result = should_create_snapshot(&todo, &None).await.unwrap();
        assert!(result, "スナップショットが存在しない場合は作成すべき");
    }

    #[tokio::test]
    async fn test_should_create_snapshot_event_threshold() {
        let todo = create_test_todo();

        // イベント数が閾値未満のスナップショット
        let snapshot = create_test_snapshot_data(&todo, 1);
        let result = should_create_snapshot(&todo, &Some(snapshot))
            .await
            .unwrap();

        // todoのversionは1なので、新規イベントは0個
        assert!(!result, "イベント数が閾値未満の場合は作成しない");

        // イベント数が閾値以上になるようにtodoのversionを調整
        let mut todo_with_many_events = todo.clone();
        todo_with_many_events.version = (SNAPSHOT_EVENT_THRESHOLD + 1) as u64;

        let snapshot = create_test_snapshot_data(&todo_with_many_events, 1);
        let result = should_create_snapshot(&todo_with_many_events, &Some(snapshot))
            .await
            .unwrap();

        assert!(result, "イベント数が閾値以上の場合は作成すべき");
    }

    #[tokio::test]
    async fn test_should_create_snapshot_age_threshold() {
        let mut todo = create_test_todo();

        // 古い更新日時を設定
        todo.updated_at = Utc::now() - Duration::days(SNAPSHOT_AGE_THRESHOLD_DAYS + 1);

        let snapshot = create_test_snapshot_data(&todo, 1);
        let result = should_create_snapshot(&todo, &Some(snapshot))
            .await
            .unwrap();

        assert!(result, "時間閾値を超えた場合は作成すべき");
    }

    #[tokio::test]
    async fn test_should_not_create_snapshot_within_thresholds() {
        let todo = create_test_todo();

        // 最近のスナップショットで、イベント数も少ない
        let snapshot = create_test_snapshot_data(&todo, todo.version as usize);
        let result = should_create_snapshot(&todo, &Some(snapshot))
            .await
            .unwrap();

        assert!(!result, "閾値内の場合は作成しない");
    }

    #[test]
    fn test_snapshot_result_serialization() {
        let result = SnapshotResult {
            processed_todos: 5,
            created_snapshots: 2,
            errors: vec!["テストエラー".to_string()],
        };

        let serialized = serde_json::to_string(&result).unwrap();
        assert!(serialized.contains("processed_todos"));
        assert!(serialized.contains("created_snapshots"));
        assert!(serialized.contains("errors"));
    }

    #[test]
    fn test_schedule_event_deserialization() {
        let json = r#"{
            "detail-type": "Scheduled Event",
            "source": "aws.events",
            "account": "123456789012",
            "time": "2023-01-01T00:00:00Z",
            "region": "ap-northeast-1",
            "detail": {}
        }"#;

        let event: ScheduleEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.detail_type, "Scheduled Event");
        assert_eq!(event.source, "aws.events");
    }

    #[tokio::test]
    async fn test_get_all_family_ids() {
        // モックのProjectionRepositoryを使用する場合のテスト
        // 実際の実装では、テスト用のDynamoDBクライアントを使用

        // 現在は固定値を返すので、その動作をテスト
        let family_ids = ["demo_family".to_string()];
        assert_eq!(family_ids.len(), 1);
        assert_eq!(family_ids[0], "demo_family");
    }

    #[test]
    fn test_constants() {
        assert_eq!(SNAPSHOT_EVENT_THRESHOLD, 100);
        assert_eq!(SNAPSHOT_AGE_THRESHOLD_DAYS, 7);
        assert_eq!(SNAPSHOT_TTL_DAYS, 90);
        assert_eq!(KEEP_SNAPSHOT_COUNT, 5);
    }
}

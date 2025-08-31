//! HTTP API（axum）スケルトン
//! 
//! 小さな一歩として `/health` エンドポイントのみを提供します。
//! 今後、`/todos` を段階的に追加していきます。

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
    extract::{Path, State},
};
use serde::Serialize;
use std::{collections::HashMap, sync::{Arc, Mutex}};
use thiserror::Error;
use todo_domain::{Command, Event, Todo};
use ulid::Ulid;

/// ルータを構築して返します。
/// 将来的に State を注入する設計に拡張可能です。
pub fn app() -> Router {
    app_with_state(AppState::default())
}

/// 単一 Todo 取得
async fn get_todo(Path(id): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    let family_id = "f-local"; // TODO: JWT から取得
    match state.store.load(&id, family_id) {
        Ok(events) if !events.is_empty() => {
            let todo = Todo::from_events(events);
            if todo.deleted {
                return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"not_found"}))).into_response();
            }
            let view = TodoSummary { id, title: todo.title, completed: todo.completed };
            (StatusCode::OK, Json(view)).into_response()
        }
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"not_found"}))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store"}))).into_response(),
    }
}

/// 外部から状態を注入できる版
pub fn app_with_state(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/todos", get(list_todos).post(create_todo))
        .route("/todos/:id", get(get_todo).patch(patch_todo))
        .with_state(state)
}

/// アプリケーションの共有状態
#[derive(Clone)]
pub struct AppState {
    store: Arc<dyn EventStore>,
}

impl Default for AppState {
    fn default() -> Self {
        Self { store: Arc::new(InMemoryEventStore::default()) }
    }
}

/// イベントストアの最小抽象
pub trait EventStore: Send + Sync {
    /// 集約にイベントを追記
    fn append(&self, aggregate_id: &str, family_id: &str, events: &[Event]) -> Result<(), StoreError>;
    /// 集約のイベント履歴を取得
    fn load(&self, aggregate_id: &str, family_id: &str) -> Result<Vec<Event>, StoreError>;
    /// 未完了一覧を取得（簡易版、ページングは後続）
    fn list_active(&self, family_id: &str) -> Result<Vec<TodoSummary>, StoreError>;
}

/// ストア層のエラー
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("unexpected store error")] 
    Unexpected,
}

/// 簡易な InMemory 実装（開発/テスト用）
#[derive(Default)]
struct InMemoryEventStore {
    // 追記呼び出しの履歴（テスト観測用）
    appends: Mutex<Vec<AppendCall>>, 
    // イベント履歴
    events_by_agg: Mutex<HashMap<(String, String), Vec<Event>>>,
    // プロジェクション（未完了一覧用）
    projection: Mutex<HashMap<(String, String), ProjectionState>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppendCall {
    aggregate_id: String,
    family_id: String,
    events: Vec<Event>,
}

impl EventStore for InMemoryEventStore {
    fn append(&self, aggregate_id: &str, family_id: &str, events: &[Event]) -> Result<(), StoreError> {
        let call = AppendCall {
            aggregate_id: aggregate_id.to_string(),
            family_id: family_id.to_string(),
            events: events.to_vec(),
        };
        self.appends.lock().unwrap().push(call);

        // イベント履歴に追記
        let key = (family_id.to_string(), aggregate_id.to_string());
        let mut map = self.events_by_agg.lock().unwrap();
        let entry = map.entry(key.clone()).or_default();
        entry.extend_from_slice(events);

        // プロジェクションを更新
        let mut proj = self.projection.lock().unwrap();
        let st = proj.entry(key).or_insert_with(|| ProjectionState { title: String::new(), completed: false, deleted: false });
        for ev in events {
            match ev {
                Event::Created { title } => { st.title = title.clone(); st.completed = false; st.deleted = false; }
                Event::TitleChanged { title } => { st.title = title.clone(); }
                Event::Completed => { st.completed = true; }
                Event::Reopened => { st.completed = false; }
                Event::Deleted => { st.deleted = true; }
            }
        }
        Ok(())
    }

    fn load(&self, aggregate_id: &str, family_id: &str) -> Result<Vec<Event>, StoreError> {
        let key = (family_id.to_string(), aggregate_id.to_string());
        let map = self.events_by_agg.lock().unwrap();
        Ok(map.get(&key).cloned().unwrap_or_default())
    }

    fn list_active(&self, family_id: &str) -> Result<Vec<TodoSummary>, StoreError> {
        let proj = self.projection.lock().unwrap();
        let mut out = Vec::new();
        for ((fam, id), st) in proj.iter() {
            if fam == family_id && !st.deleted && !st.completed {
                out.push(TodoSummary { id: id.clone(), title: st.title.clone(), completed: st.completed });
            }
        }
        Ok(out)
    }
}

#[derive(Debug, Clone)]
struct ProjectionState { title: String, completed: bool, deleted: bool }

/// POST /todos リクエスト
#[derive(Debug, serde::Deserialize)]
struct CreateTodoRequest {
    /// タイトル（単純化のためバリデーションは後続）
    title: String,
}

/// POST /todos レスポンス
#[derive(Debug, Serialize, PartialEq, Eq)]
struct CreateTodoResponse {
    id: String,
    title: String,
}

/// GET /todos レスポンスの要素
#[derive(Debug, Serialize, serde::Deserialize, PartialEq, Eq, Clone)]
pub struct TodoSummary { id: String, title: String, completed: bool }

/// ヘルスチェック用ハンドラ
async fn health() -> impl IntoResponse {
    let body = HealthBody { status: "ok" };
    (StatusCode::OK, Json(body))
}

/// Todo 作成ハンドラ（最小）
/// - ID は ULID を採用
/// - familyId は暫定的に固定（JWT 連携は後続）
async fn create_todo(
    State(state): State<AppState>,
    Json(req): Json<CreateTodoRequest>,
) -> impl IntoResponse {
    let id = Ulid::new().to_string();
    let family_id = "f-local"; // TODO: JWT から取得

    // ドメインの決定則に従ってイベントを生成
    let aggregate = Todo::default();
    let events = match aggregate.decide(Command::Create { title: req.title.clone() }) {
        Ok(e) => e,
        Err(_e) => {
            // バリデーションなどの詳細は後続
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"invalid"}))).into_response();
        }
    };

    // ストアに追記（Dynamo 置換予定）
    if let Err(_se) = state.store.append(&id, family_id, &events) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store"}))).into_response();
    }

    let res = CreateTodoResponse { id: id.clone(), title: req.title };
    (StatusCode::CREATED, Json(res)).into_response()
}

/// 未完了一覧ハンドラ（簡易）
async fn list_todos(State(state): State<AppState>) -> impl IntoResponse {
    let family_id = "f-local"; // TODO: JWT から取得
    match state.store.list_active(family_id) {
        Ok(list) => (StatusCode::OK, Json(list)).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store"}))).into_response(),
    }
}

/// PATCH /todos/{id} リクエスト（内部タグ方式）
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum PatchTodoRequest {
    ChangeTitle { title: String },
    Complete,
    Reopen,
}

/// 単一 Todo 更新（タイトル変更/完了/再開）
async fn patch_todo(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<PatchTodoRequest>,
) -> impl IntoResponse {
    let family_id = "f-local"; // TODO: JWT から取得
    let mut events = match state.store.load(&id, family_id) {
        Ok(e) => e,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store"}))).into_response(),
    };
    if events.is_empty() {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"not_found"}))).into_response();
    }

    let current = Todo::from_events(events.clone());
    let cmd = match req {
        PatchTodoRequest::ChangeTitle { title } => Command::ChangeTitle { title },
        PatchTodoRequest::Complete => Command::Complete,
        PatchTodoRequest::Reopen => Command::Reopen,
    };
    let decided = match current.decide(cmd) {
        Ok(e) => e,
        Err(todo_domain::DomainError::Deleted) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"not_found"}))).into_response(),
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"invalid"}))).into_response(),
    };

    if let Err(_) = state.store.append(&id, family_id, &decided) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store"}))).into_response();
    }

    events.extend(decided);
    let updated = Todo::from_events(events);
    let view = TodoSummary { id, title: updated.title, completed: updated.completed };
    (StatusCode::OK, Json(view)).into_response()
}

#[derive(Debug, Serialize)]
struct HealthBody {
    /// サービスの簡易ステータス
    status: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::{self, Body}, http::Request};
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn get_health_returns_ok() {
        let app = app();

        let request = Request::builder()
            .method("GET")
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let bytes = body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn post_todos_creates_event_and_returns_201() {
        // InMemory ストアを注入
        let store = Arc::new(InMemoryEventStore::default());
        let state = AppState { store: store.clone() };
        let app = app_with_state(state);

        let body = serde_json::json!({"title":"Buy milk"});
        let request = Request::builder()
            .method("POST")
            .uri("/todos")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let id = json["id"].as_str().unwrap().to_string();
        assert_eq!(json["title"], "Buy milk");

        // ストアに Created イベントが 1 回追記されたことを検証
        let appends = store.appends.lock().unwrap().clone();
        assert_eq!(appends.len(), 1);
        assert_eq!(appends[0].aggregate_id, id);
        assert_eq!(appends[0].events, vec![Event::Created { title: "Buy milk".into() }]);
    }

    #[tokio::test]
    async fn get_todos_returns_active_items() {
        let store = Arc::new(InMemoryEventStore::default());
        let state = AppState { store: store.clone() };
        let app = app_with_state(state);

        // 2件作成
        for title in ["A", "B"] {
            let body = serde_json::json!({"title": title});
            let req = Request::builder().method("POST").uri("/todos").header("content-type", "application/json").body(Body::from(body.to_string())).unwrap();
            let resp = app.clone().oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::CREATED);
        }

        // GET /todos
        let request = Request::builder().method("GET").uri("/todos").body(Body::empty()).unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Vec<TodoSummary> = serde_json::from_slice(&bytes).unwrap();
        let titles: Vec<String> = json.into_iter().map(|x| x.title).collect();
        assert!(titles.contains(&"A".to_string()) && titles.contains(&"B".to_string()));
    }

    #[tokio::test]
    async fn get_todo_returns_item() {
        let store = Arc::new(InMemoryEventStore::default());
        let state = AppState { store: store.clone() };
        let app = app_with_state(state);

        // 作成
        let body = serde_json::json!({"title": "Task"});
        let req = Request::builder().method("POST").uri("/todos").header("content-type", "application/json").body(Body::from(body.to_string())).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let bytes = body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let id = json["id"].as_str().unwrap().to_string();

        // 取得
        let req_get = Request::builder().method("GET").uri(format!("/todos/{}", id)).body(Body::empty()).unwrap();
        let res_get = app.oneshot(req_get).await.unwrap();
        assert_eq!(res_get.status(), StatusCode::OK);
        let bytes = body::to_bytes(res_get.into_body(), usize::MAX).await.unwrap();
        let view: TodoSummary = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(view.id, id);
        assert_eq!(view.title, "Task");
        assert!(!view.completed);
    }
}

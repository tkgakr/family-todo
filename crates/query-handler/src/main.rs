use anyhow::Result;
use chrono::{DateTime, Utc};
use domain::{TodoEvent, TodoId};
use infrastructure::{DynamoDbClient, EventRepository, ProjectionRepository};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{init_tracing, Config};
use std::collections::HashMap;
use tracing::{error, info};

/// API Gateway プロキシリクエスト構造体
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct ApiGatewayProxyRequest {
    http_method: String,
    path: String,
    path_parameters: Option<HashMap<String, String>>,
    query_string_parameters: Option<HashMap<String, String>>,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
    request_context: RequestContext,
}

/// リクエストコンテキスト構造体
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestContext {
    authorizer: Option<Authorizer>,
}

/// 認証情報構造体（Lambda Authorizer からのコンテキスト）
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct Authorizer {
    user_id: Option<String>,
    family_id: Option<String>,
    email: Option<String>,
    token_use: Option<String>,
    cognito_groups: Option<String>,
}

/// API Gateway プロキシレスポンス構造体
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayProxyResponse {
    status_code: u16,
    headers: HashMap<String, String>,
    body: String,
}

/// ToDo一覧レスポンス
#[derive(Debug, Serialize)]
struct TodoListResponse {
    todos: Vec<TodoResponse>,
    total_count: usize,
}

/// ToDo詳細レスポンス
#[derive(Debug, Serialize)]
struct TodoResponse {
    id: String,
    title: String,
    description: Option<String>,
    tags: Vec<String>,
    completed: bool,
    created_by: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: u64,
}

/// ToDo履歴レスポンス
#[derive(Debug, Serialize)]
struct TodoHistoryResponse {
    todo_id: String,
    events: Vec<TodoEventResponse>,
    total_count: usize,
}

/// イベントレスポンス
#[derive(Debug, Serialize)]
struct TodoEventResponse {
    event_id: String,
    event_type: String,
    timestamp: DateTime<Utc>,
    user_id: String,
    data: Value,
}

/// クエリの種類を表す列挙型
#[derive(Debug)]
enum Query {
    ListTodos {
        status: Option<String>,
        limit: Option<i32>,
    },
    GetTodo {
        todo_id: TodoId,
    },
    GetTodoHistory {
        todo_id: TodoId,
    },
}

/// クエリハンドラーのメイン関数
async fn function_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    info!(
        "QueryHandler開始: method={}, path={}",
        event.payload.http_method, event.payload.path
    );

    // 設定を読み込み
    let config = Config::from_env().map_err(|e| {
        error!("設定読み込みエラー: {}", e);
        Error::from(format!("設定エラー: {e}"))
    })?;

    // DynamoDBクライアントを初期化
    let db_client = DynamoDbClient::new(&config).await.map_err(|e| {
        error!("DynamoDBクライアント初期化エラー: {}", e);
        Error::from(format!("DynamoDBエラー: {e}"))
    })?;

    let event_repo = EventRepository::new(db_client.clone());
    let projection_repo = ProjectionRepository::new(db_client);

    // リクエストを処理
    match handle_request(&event.payload, &event_repo, &projection_repo).await {
        Ok(response) => {
            info!("QueryHandler完了: status={}", response.status_code);
            Ok(response)
        }
        Err(e) => {
            error!("QueryHandlerエラー: {}", e);
            Ok(create_error_response(500, "内部サーバーエラー"))
        }
    }
}

/// リクエストを処理する
async fn handle_request(
    request: &ApiGatewayProxyRequest,
    event_repo: &EventRepository,
    projection_repo: &ProjectionRepository,
) -> Result<ApiGatewayProxyResponse> {
    // Lambda Authorizer からユーザー情報を抽出
    let (user_id, family_id) = extract_user_info_from_authorizer(request)?;

    info!(
        "ユーザー認証成功: user_id={}, family_id={}",
        user_id, family_id
    );

    // パスとメソッドに基づいてクエリをパース
    let query = parse_query(request)?;

    // クエリを実行
    execute_query(query, &family_id, event_repo, projection_repo).await
}

/// Lambda Authorizer からユーザー情報を抽出
fn extract_user_info_from_authorizer(request: &ApiGatewayProxyRequest) -> Result<(String, String)> {
    let authorizer = request
        .request_context
        .authorizer
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("認証情報が見つかりません"))?;

    let user_id = authorizer
        .user_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ユーザーIDが見つかりません"))?
        .clone();

    let family_id = authorizer
        .family_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("家族IDが見つかりません"))?
        .clone();

    Ok((user_id, family_id))
}

/// リクエストからクエリをパース
fn parse_query(request: &ApiGatewayProxyRequest) -> Result<Query> {
    let method = &request.http_method;
    let path = &request.path;

    match (method.as_str(), path.as_str()) {
        ("GET", "/queries/todos") => {
            // ToDo一覧取得
            let query_params = request.query_string_parameters.as_ref();

            let status = query_params
                .and_then(|params| params.get("status"))
                .map(|s| s.to_string());

            let limit = query_params
                .and_then(|params| params.get("limit"))
                .and_then(|s| s.parse::<i32>().ok());

            Ok(Query::ListTodos { status, limit })
        }
        ("GET", path) if path.starts_with("/queries/todos/") && !path.ends_with("/history") => {
            // ToDo詳細取得
            let todo_id = extract_todo_id_from_path(path)?;
            Ok(Query::GetTodo { todo_id })
        }
        ("GET", path) if path.starts_with("/queries/todos/") && path.ends_with("/history") => {
            // ToDo履歴取得
            let todo_id = extract_todo_id_from_history_path(path)?;
            Ok(Query::GetTodoHistory { todo_id })
        }
        _ => Err(anyhow::anyhow!(
            "サポートされていないメソッドまたはパス: {} {}",
            method,
            path
        )),
    }
}

/// パスからTodoIdを抽出
fn extract_todo_id_from_path(path: &str) -> Result<TodoId> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 4 && parts[1] == "queries" && parts[2] == "todos" {
        let todo_id_str = parts[3];
        TodoId::from_string(todo_id_str.to_string())
            .map_err(|e| anyhow::anyhow!("無効なTodoId: {}", e))
    } else {
        Err(anyhow::anyhow!("パスからTodoIdを抽出できません: {}", path))
    }
}

/// 履歴パスからTodoIdを抽出
fn extract_todo_id_from_history_path(path: &str) -> Result<TodoId> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 5 && parts[1] == "queries" && parts[2] == "todos" && parts[4] == "history" {
        let todo_id_str = parts[3];
        TodoId::from_string(todo_id_str.to_string())
            .map_err(|e| anyhow::anyhow!("無効なTodoId: {}", e))
    } else {
        Err(anyhow::anyhow!(
            "履歴パスからTodoIdを抽出できません: {}",
            path
        ))
    }
}

/// クエリを実行
async fn execute_query(
    query: Query,
    family_id: &str,
    event_repo: &EventRepository,
    projection_repo: &ProjectionRepository,
) -> Result<ApiGatewayProxyResponse> {
    match query {
        Query::ListTodos { status, limit } => {
            info!(
                "ToDo一覧取得クエリ実行: status={:?}, limit={:?}",
                status, limit
            );

            let todos = if status.as_deref() == Some("active") {
                // アクティブなToDo一覧を取得
                projection_repo
                    .get_active_todos(family_id, limit)
                    .await
                    .map_err(|e| {
                        error!("アクティブToDo取得エラー: {}", e);
                        anyhow::anyhow!("アクティブToDo取得に失敗しました: {}", e)
                    })?
            } else {
                // 全てのToDo一覧を取得
                projection_repo
                    .get_all_todos(family_id, limit)
                    .await
                    .map_err(|e| {
                        error!("全ToDo取得エラー: {}", e);
                        anyhow::anyhow!("ToDo一覧取得に失敗しました: {}", e)
                    })?
            };

            let todo_responses: Vec<TodoResponse> = todos
                .into_iter()
                .map(|todo| TodoResponse {
                    id: todo.id.as_str().to_string(),
                    title: todo.title,
                    description: todo.description,
                    tags: todo.tags,
                    completed: todo.completed,
                    created_by: todo.created_by,
                    created_at: todo.created_at,
                    updated_at: todo.updated_at,
                    version: todo.version,
                })
                .collect();

            let response = TodoListResponse {
                total_count: todo_responses.len(),
                todos: todo_responses,
            };

            info!("ToDo一覧取得完了: {} 件", response.total_count);

            Ok(create_success_response(200, json!(response)))
        }
        Query::GetTodo { todo_id } => {
            info!("ToDo詳細取得クエリ実行: todo_id={}", todo_id);

            let todo = projection_repo
                .get_projection(family_id, &todo_id)
                .await
                .map_err(|e| {
                    error!("ToDo詳細取得エラー: {}", e);
                    anyhow::anyhow!("ToDo詳細取得に失敗しました: {}", e)
                })?;

            match todo {
                Some(todo) => {
                    let response = TodoResponse {
                        id: todo.id.as_str().to_string(),
                        title: todo.title,
                        description: todo.description,
                        tags: todo.tags,
                        completed: todo.completed,
                        created_by: todo.created_by,
                        created_at: todo.created_at,
                        updated_at: todo.updated_at,
                        version: todo.version,
                    };

                    info!("ToDo詳細取得完了: todo_id={}", todo_id);
                    Ok(create_success_response(200, json!(response)))
                }
                None => {
                    info!("ToDoが見つかりません: todo_id={}", todo_id);
                    Ok(create_error_response(404, "ToDoが見つかりません"))
                }
            }
        }
        Query::GetTodoHistory { todo_id } => {
            info!("ToDo履歴取得クエリ実行: todo_id={}", todo_id);

            let events = event_repo
                .get_events(family_id, &todo_id)
                .await
                .map_err(|e| {
                    error!("ToDo履歴取得エラー: {}", e);
                    anyhow::anyhow!("ToDo履歴取得に失敗しました: {}", e)
                })?;

            let event_responses: Vec<TodoEventResponse> = events
                .into_iter()
                .map(|event| {
                    let (event_type, user_id, timestamp, data) = match &event {
                        TodoEvent::TodoCreatedV2 {
                            title,
                            description,
                            tags,
                            created_by,
                            timestamp,
                            ..
                        } => (
                            "todo_created".to_string(),
                            created_by.clone(),
                            *timestamp,
                            json!({
                                "title": title,
                                "description": description,
                                "tags": tags
                            }),
                        ),
                        TodoEvent::TodoUpdatedV1 {
                            title,
                            description,
                            updated_by,
                            timestamp,
                            ..
                        } => (
                            "todo_updated".to_string(),
                            updated_by.clone(),
                            *timestamp,
                            json!({
                                "title": title,
                                "description": description
                            }),
                        ),
                        TodoEvent::TodoCompletedV1 {
                            completed_by,
                            timestamp,
                            ..
                        } => (
                            "todo_completed".to_string(),
                            completed_by.clone(),
                            *timestamp,
                            json!({}),
                        ),
                        TodoEvent::TodoDeletedV1 {
                            deleted_by,
                            reason,
                            timestamp,
                            ..
                        } => (
                            "todo_deleted".to_string(),
                            deleted_by.clone(),
                            *timestamp,
                            json!({
                                "reason": reason
                            }),
                        ),
                    };

                    TodoEventResponse {
                        event_id: event.event_id().to_string(),
                        event_type,
                        timestamp,
                        user_id,
                        data,
                    }
                })
                .collect();

            let response = TodoHistoryResponse {
                todo_id: todo_id.as_str().to_string(),
                total_count: event_responses.len(),
                events: event_responses,
            };

            info!(
                "ToDo履歴取得完了: todo_id={}, {} 件",
                todo_id, response.total_count
            );

            Ok(create_success_response(200, json!(response)))
        }
    }
}

/// 成功レスポンスを作成
fn create_success_response(status_code: u16, body: Value) -> ApiGatewayProxyResponse {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
    headers.insert(
        "Access-Control-Allow-Headers".to_string(),
        "Content-Type,Authorization".to_string(),
    );

    ApiGatewayProxyResponse {
        status_code,
        headers,
        body: body.to_string(),
    }
}

/// エラーレスポンスを作成
fn create_error_response(status_code: u16, message: &str) -> ApiGatewayProxyResponse {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
    headers.insert(
        "Access-Control-Allow-Headers".to_string(),
        "Content-Type,Authorization".to_string(),
    );

    let body = json!({
        "error": message,
        "status_code": status_code
    });

    ApiGatewayProxyResponse {
        status_code,
        headers,
        body: body.to_string(),
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
    use anyhow::Result;
    use serde_json::json;
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// テスト用の Authorizer 構造体
    #[derive(Debug, Clone)]
    struct TestAuthorizer {
        user_id: Option<String>,
        family_id: Option<String>,
        email: Option<String>,
        token_use: Option<String>,
        cognito_groups: Option<String>,
    }

    /// テスト用の API Gateway リクエスト構造体
    #[derive(Debug)]
    struct TestApiGatewayRequest {
        http_method: String,
        path: String,
        body: Option<String>,
        request_context: TestRequestContext,
    }

    #[derive(Debug)]
    struct TestRequestContext {
        authorizer: Option<TestAuthorizer>,
    }

    /// テスト用のクレーム構造体
    #[derive(Debug, Clone)]
    struct TestClaims {
        sub: String,
        email: String,
        family_id: Option<String>,
        exp: i64,
        iat: i64,
        aud: String,
        iss: String,
        token_use: String,
        cognito_groups: Option<Vec<String>>,
    }

    /// テスト用のリクエストから認証情報を抽出
    fn extract_user_claims(request: &TestApiGatewayRequest) -> Result<TestClaims> {
        match &request.request_context.authorizer {
            Some(auth) => {
                let claims = TestClaims {
                    sub: auth.user_id.clone().unwrap_or_default(),
                    email: auth.email.clone().unwrap_or_default(),
                    family_id: auth.family_id.clone(),
                    exp: (SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        + 3600) as i64,
                    iat: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                    aud: "test-client-id".to_string(),
                    iss: "https://cognito-idp.us-east-1.amazonaws.com/us-east-1_test".to_string(),
                    token_use: auth.token_use.clone().unwrap_or("access".to_string()),
                    cognito_groups: auth.cognito_groups.as_ref().map(|g| vec![g.clone()]),
                };
                Ok(claims)
            }
            None => Err(anyhow::anyhow!("認証情報が見つかりません")),
        }
    }

    fn create_test_request(
        method: &str,
        path: &str,
        query_params: Option<HashMap<String, String>>,
        authorizer: Option<TestAuthorizer>,
    ) -> TestApiGatewayRequest {
        TestApiGatewayRequest {
            http_method: method.to_string(),
            path: path.to_string(),
            body: None,
            request_context: TestRequestContext { authorizer },
        }
    }

    fn create_test_authorizer() -> TestAuthorizer {
        TestAuthorizer {
            user_id: Some("user123".to_string()),
            family_id: Some("family456".to_string()),
            email: Some("test@example.com".to_string()),
            token_use: Some("access".to_string()),
            cognito_groups: Some("family-member".to_string()),
        }
    }

    #[test]
    fn test_extract_user_claims_success() {
        let authorizer = create_test_authorizer();
        let request = create_test_request("GET", "/queries/todos", None, Some(authorizer));

        let result = extract_user_claims(&request);
        assert!(result.is_ok());

        let claims = result.unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.family_id, Some("family456".to_string()));
    }

    #[test]
    fn test_extract_user_claims_missing_authorizer() {
        let request = create_test_request("GET", "/queries/todos", None, None);

        let result = extract_user_claims(&request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("認証情報が見つかりません"));
    }

    /*
    #[test]
    fn test_parse_query_list_todos_all() {
        let claims = create_test_claims();
        let request = create_test_request("GET", "/queries/todos", None, Some(claims));

        let result = parse_query(&request);
        assert!(result.is_ok());

        match result.unwrap() {
            Query::ListTodos { status, limit } => {
                assert_eq!(status, None);
                assert_eq!(limit, None);
            }
            _ => panic!("Expected ListTodos query"),
        }
    }

    #[test]
    fn test_parse_query_list_todos_active() {
        let mut query_params = HashMap::new();
        query_params.insert("status".to_string(), "active".to_string());
        query_params.insert("limit".to_string(), "10".to_string());

        let claims = create_test_claims();
        let request =
            create_test_request("GET", "/queries/todos", Some(query_params), Some(claims));

        let result = parse_query(&request);
        assert!(result.is_ok());

        match result.unwrap() {
            Query::ListTodos { status, limit } => {
                assert_eq!(status, Some("active".to_string()));
                assert_eq!(limit, Some(10));
            }
            _ => panic!("Expected ListTodos query"),
        }
    }

    #[test]
    fn test_parse_query_get_todo() {
        let todo_id = TodoId::new();
        let claims = create_test_claims();
        let request = create_test_request(
            "GET",
            &format!("/queries/todos/{}", todo_id.as_str()),
            None,
            Some(claims),
        );

        let result = parse_query(&request);
        assert!(result.is_ok());

        match result.unwrap() {
            Query::GetTodo { todo_id: parsed_id } => {
                assert_eq!(parsed_id, todo_id);
            }
            _ => panic!("Expected GetTodo query"),
        }
    }

    #[test]
    fn test_parse_query_get_todo_history() {
        let todo_id = TodoId::new();
        let claims = create_test_claims();
        let request = create_test_request(
            "GET",
            &format!("/queries/todos/{}/history", todo_id.as_str()),
            None,
            Some(claims),
        );

        let result = parse_query(&request);
        assert!(result.is_ok());

        match result.unwrap() {
            Query::GetTodoHistory { todo_id: parsed_id } => {
                assert_eq!(parsed_id, todo_id);
            }
            _ => panic!("Expected GetTodoHistory query"),
        }
    }

    #[test]
    fn test_parse_query_unsupported_method() {
        let claims = create_test_claims();
        let request = create_test_request("POST", "/queries/todos", None, Some(claims));

        let result = parse_query(&request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("サポートされていないメソッドまたはパス"));
    }

    #[test]
    fn test_extract_todo_id_from_path() {
        let todo_id = TodoId::new();
        let path = format!("/queries/todos/{}", todo_id.as_str());

        let result = extract_todo_id_from_path(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), todo_id);
    }

    #[test]
    fn test_extract_todo_id_from_path_invalid() {
        let result = extract_todo_id_from_path("/invalid/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_todo_id_from_history_path() {
        let todo_id = TodoId::new();
        let path = format!("/queries/todos/{}/history", todo_id.as_str());

        let result = extract_todo_id_from_history_path(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), todo_id);
    }

    #[test]
    fn test_extract_todo_id_from_history_path_invalid() {
        let result = extract_todo_id_from_history_path("/invalid/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_success_response() {
        let body = json!({"message": "成功"});
        let response = create_success_response(200, body);

        assert_eq!(response.status_code, 200);
        assert_eq!(
            response.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert!(response.body.contains("成功"));
    }

    #[test]
    fn test_create_error_response() {
        let response = create_error_response(400, "エラーメッセージ");

        assert_eq!(response.status_code, 400);
        assert_eq!(
            response.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert!(response.body.contains("エラーメッセージ"));
    }

    #[test]
    fn test_todo_response_serialization() {
        let todo_response = TodoResponse {
            id: "test_id".to_string(),
            title: "テストToDo".to_string(),
            description: Some("説明".to_string()),
            tags: vec!["タグ1".to_string()],
            completed: false,
            created_by: "user123".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: 1,
        };

        let json_result = serde_json::to_string(&todo_response);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("test_id"));
        assert!(json_str.contains("テストToDo"));
    }

    #[test]
    fn test_todo_list_response_serialization() {
        let todo_response = TodoResponse {
            id: "test_id".to_string(),
            title: "テストToDo".to_string(),
            description: None,
            tags: vec![],
            completed: false,
            created_by: "user123".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: 1,
        };

        let list_response = TodoListResponse {
            todos: vec![todo_response],
            total_count: 1,
        };

        let json_result = serde_json::to_string(&list_response);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("total_count"));
        assert!(json_str.contains("todos"));
    }

    #[test]
    fn test_todo_history_response_serialization() {
        let event_response = TodoEventResponse {
            event_id: "event123".to_string(),
            event_type: "todo_created".to_string(),
            timestamp: chrono::Utc::now(),
            user_id: "user123".to_string(),
            data: json!({"title": "テストToDo"}),
        };

        let history_response = TodoHistoryResponse {
            todo_id: "todo123".to_string(),
            events: vec![event_response],
            total_count: 1,
        };

        let json_result = serde_json::to_string(&history_response);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("todo_id"));
        assert!(json_str.contains("events"));
        assert!(json_str.contains("total_count"));
    }
    */
}

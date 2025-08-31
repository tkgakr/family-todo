use anyhow::Result;
use domain::{TodoEvent, TodoId};
use infrastructure::{DynamoDbClient, EventRepository};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{auth::Claims, init_tracing, Config};
use std::collections::HashMap;
use tracing::{error, info};

/// API Gateway プロキシリクエスト構造体
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// 認証情報構造体
#[derive(Debug, Deserialize)]
struct Authorizer {
    claims: Option<HashMap<String, Value>>,
}

/// API Gateway プロキシレスポンス構造体
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayProxyResponse {
    status_code: u16,
    headers: HashMap<String, String>,
    body: String,
}

/// ToDo作成リクエスト
#[derive(Debug, Deserialize)]
struct CreateTodoRequest {
    title: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
}

/// ToDo更新リクエスト
#[derive(Debug, Deserialize)]
struct UpdateTodoRequest {
    title: Option<String>,
    description: Option<String>,
}

/// ToDo削除リクエスト
#[derive(Debug, Deserialize)]
struct DeleteTodoRequest {
    reason: Option<String>,
}

/// コマンドの種類を表す列挙型
#[derive(Debug)]
enum Command {
    CreateTodo {
        title: String,
        description: Option<String>,
        tags: Vec<String>,
    },
    UpdateTodo {
        todo_id: TodoId,
        title: Option<String>,
        description: Option<String>,
    },
    CompleteTodo {
        todo_id: TodoId,
    },
    DeleteTodo {
        todo_id: TodoId,
        reason: Option<String>,
    },
}

/// コマンドハンドラーのメイン関数
async fn function_handler(
    event: LambdaEvent<ApiGatewayProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    info!(
        "CommandHandler開始: method={}, path={}",
        event.payload.http_method, event.payload.path
    );

    // 設定を読み込み
    let config = Config::from_env().map_err(|e| {
        error!("設定読み込みエラー: {}", e);
        Error::from(format!("設定エラー: {}", e))
    })?;

    // DynamoDBクライアントを初期化
    let db_client = DynamoDbClient::new(&config).await.map_err(|e| {
        error!("DynamoDBクライアント初期化エラー: {}", e);
        Error::from(format!("DynamoDBエラー: {}", e))
    })?;

    let event_repo = EventRepository::new(db_client);

    // リクエストを処理
    match handle_request(&event.payload, &event_repo).await {
        Ok(response) => {
            info!("CommandHandler完了: status={}", response.status_code);
            Ok(response)
        }
        Err(e) => {
            error!("CommandHandlerエラー: {}", e);
            Ok(create_error_response(500, "内部サーバーエラー"))
        }
    }
}

/// リクエストを処理する
async fn handle_request(
    request: &ApiGatewayProxyRequest,
    event_repo: &EventRepository,
) -> Result<ApiGatewayProxyResponse> {
    // JWTトークンからユーザー情報を抽出
    let claims = extract_user_claims(request)?;
    let user_id = claims.sub.clone();
    let family_id = claims
        .family_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("family_idがJWTクレームに含まれていません"))?;

    info!(
        "ユーザー認証成功: user_id={}, family_id={}",
        user_id, family_id
    );

    // パスとメソッドに基づいてコマンドをパース
    let command = parse_command(request)?;

    // コマンドを実行
    execute_command(command, &user_id, &family_id, event_repo).await
}

/// JWTトークンからユーザー情報を抽出
fn extract_user_claims(request: &ApiGatewayProxyRequest) -> Result<Claims> {
    let authorizer = request
        .request_context
        .authorizer
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("認証情報が見つかりません"))?;

    let claims_map = authorizer
        .claims
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("JWTクレームが見つかりません"))?;

    // 必要なクレームを抽出
    let sub = claims_map
        .get("sub")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("subクレームが見つかりません"))?
        .to_string();

    let email = claims_map
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("emailクレームが見つかりません"))?
        .to_string();

    let family_id = claims_map
        .get("custom:family_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let exp = claims_map
        .get("exp")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("expクレームが見つかりません"))?;

    let iat = claims_map
        .get("iat")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("iatクレームが見つかりません"))?;

    Ok(Claims {
        sub,
        email,
        family_id,
        exp,
        iat,
    })
}

/// リクエストからコマンドをパース
fn parse_command(request: &ApiGatewayProxyRequest) -> Result<Command> {
    let method = &request.http_method;
    let path = &request.path;

    match (method.as_str(), path.as_str()) {
        ("POST", "/commands/todos") => {
            // ToDo作成
            let body = request
                .body
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("リクエストボディが必要です"))?;

            let create_req: CreateTodoRequest = serde_json::from_str(body)
                .map_err(|e| anyhow::anyhow!("リクエストボディのパースエラー: {}", e))?;

            Ok(Command::CreateTodo {
                title: create_req.title,
                description: create_req.description,
                tags: create_req.tags.unwrap_or_default(),
            })
        }
        ("PUT", path) if path.starts_with("/commands/todos/") => {
            // ToDo更新
            let todo_id = extract_todo_id_from_path(path)?;
            let body = request
                .body
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("リクエストボディが必要です"))?;

            let update_req: UpdateTodoRequest = serde_json::from_str(body)
                .map_err(|e| anyhow::anyhow!("リクエストボディのパースエラー: {}", e))?;

            Ok(Command::UpdateTodo {
                todo_id,
                title: update_req.title,
                description: update_req.description,
            })
        }
        ("POST", path) if path.starts_with("/commands/todos/") && path.ends_with("/complete") => {
            // ToDo完了
            let todo_id = extract_todo_id_from_complete_path(path)?;
            Ok(Command::CompleteTodo { todo_id })
        }
        ("DELETE", path) if path.starts_with("/commands/todos/") => {
            // ToDo削除
            let todo_id = extract_todo_id_from_path(path)?;
            let reason = if let Some(body) = &request.body {
                if !body.trim().is_empty() {
                    let delete_req: DeleteTodoRequest = serde_json::from_str(body)
                        .map_err(|e| anyhow::anyhow!("リクエストボディのパースエラー: {}", e))?;
                    delete_req.reason
                } else {
                    None
                }
            } else {
                None
            };

            Ok(Command::DeleteTodo { todo_id, reason })
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
    if parts.len() >= 4 && parts[1] == "commands" && parts[2] == "todos" {
        let todo_id_str = parts[3];
        TodoId::from_string(todo_id_str.to_string())
            .map_err(|e| anyhow::anyhow!("無効なTodoId: {}", e))
    } else {
        Err(anyhow::anyhow!("パスからTodoIdを抽出できません: {}", path))
    }
}

/// 完了パスからTodoIdを抽出
fn extract_todo_id_from_complete_path(path: &str) -> Result<TodoId> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 5 && parts[1] == "commands" && parts[2] == "todos" && parts[4] == "complete" {
        let todo_id_str = parts[3];
        TodoId::from_string(todo_id_str.to_string())
            .map_err(|e| anyhow::anyhow!("無効なTodoId: {}", e))
    } else {
        Err(anyhow::anyhow!(
            "完了パスからTodoIdを抽出できません: {}",
            path
        ))
    }
}

/// コマンドを実行
async fn execute_command(
    command: Command,
    user_id: &str,
    family_id: &str,
    event_repo: &EventRepository,
) -> Result<ApiGatewayProxyResponse> {
    match command {
        Command::CreateTodo {
            title,
            description,
            tags,
        } => {
            info!("ToDo作成コマンド実行: title={}", title);

            // バリデーション
            if title.trim().is_empty() {
                return Ok(create_error_response(400, "タイトルは必須です"));
            }
            if title.len() > 200 {
                return Ok(create_error_response(
                    400,
                    "タイトルは200文字以内で入力してください",
                ));
            }

            let todo_id = TodoId::new();
            let event = TodoEvent::new_todo_created(
                todo_id.clone(),
                title.trim().to_string(),
                description
                    .map(|d| d.trim().to_string())
                    .filter(|d| !d.is_empty()),
                tags,
                user_id.to_string(),
            );

            // イベントを保存
            event_repo.save_event(family_id, event).await.map_err(|e| {
                error!("イベント保存エラー: {}", e);
                anyhow::anyhow!("ToDo作成に失敗しました: {}", e)
            })?;

            info!("ToDo作成完了: todo_id={}", todo_id);

            Ok(create_success_response(
                201,
                json!({
                    "message": "ToDoが作成されました",
                    "todo_id": todo_id.as_str()
                }),
            ))
        }
        Command::UpdateTodo {
            todo_id,
            title,
            description,
        } => {
            info!("ToDo更新コマンド実行: todo_id={}", todo_id);

            // タイトルのバリデーション
            if let Some(ref title) = title {
                if title.trim().is_empty() {
                    return Ok(create_error_response(400, "タイトルは必須です"));
                }
                if title.len() > 200 {
                    return Ok(create_error_response(
                        400,
                        "タイトルは200文字以内で入力してください",
                    ));
                }
            }

            let event = TodoEvent::new_todo_updated(
                todo_id.clone(),
                title.map(|t| t.trim().to_string()),
                description
                    .map(|d| d.trim().to_string())
                    .filter(|d| !d.is_empty()),
                user_id.to_string(),
            );

            // イベントを保存
            event_repo.save_event(family_id, event).await.map_err(|e| {
                error!("イベント保存エラー: {}", e);
                anyhow::anyhow!("ToDo更新に失敗しました: {}", e)
            })?;

            info!("ToDo更新完了: todo_id={}", todo_id);

            Ok(create_success_response(
                200,
                json!({
                    "message": "ToDoが更新されました",
                    "todo_id": todo_id.as_str()
                }),
            ))
        }
        Command::CompleteTodo { todo_id } => {
            info!("ToDo完了コマンド実行: todo_id={}", todo_id);

            let event = TodoEvent::new_todo_completed(todo_id.clone(), user_id.to_string());

            // イベントを保存
            event_repo.save_event(family_id, event).await.map_err(|e| {
                error!("イベント保存エラー: {}", e);
                anyhow::anyhow!("ToDo完了に失敗しました: {}", e)
            })?;

            info!("ToDo完了完了: todo_id={}", todo_id);

            Ok(create_success_response(
                200,
                json!({
                    "message": "ToDoが完了されました",
                    "todo_id": todo_id.as_str()
                }),
            ))
        }
        Command::DeleteTodo { todo_id, reason } => {
            info!("ToDo削除コマンド実行: todo_id={}", todo_id);

            let event = TodoEvent::new_todo_deleted(todo_id.clone(), user_id.to_string(), reason);

            // イベントを保存
            event_repo.save_event(family_id, event).await.map_err(|e| {
                error!("イベント保存エラー: {}", e);
                anyhow::anyhow!("ToDo削除に失敗しました: {}", e)
            })?;

            info!("ToDo削除完了: todo_id={}", todo_id);

            Ok(create_success_response(
                200,
                json!({
                    "message": "ToDoが削除されました",
                    "todo_id": todo_id.as_str()
                }),
            ))
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
    use serde_json::json;
    use std::collections::HashMap;

    fn create_test_request(
        method: &str,
        path: &str,
        body: Option<&str>,
        claims: Option<HashMap<String, Value>>,
    ) -> ApiGatewayProxyRequest {
        ApiGatewayProxyRequest {
            http_method: method.to_string(),
            path: path.to_string(),
            path_parameters: None,
            query_string_parameters: None,
            headers: None,
            body: body.map(|s| s.to_string()),
            request_context: RequestContext {
                authorizer: claims.map(|c| Authorizer { claims: Some(c) }),
            },
        }
    }

    fn create_test_claims() -> HashMap<String, Value> {
        let mut claims = HashMap::new();
        claims.insert("sub".to_string(), json!("user123"));
        claims.insert("email".to_string(), json!("test@example.com"));
        claims.insert("custom:family_id".to_string(), json!("family456"));
        claims.insert("exp".to_string(), json!(1234567890));
        claims.insert("iat".to_string(), json!(1234567800));
        claims
    }

    #[test]
    fn test_extract_user_claims_success() {
        let claims = create_test_claims();
        let request = create_test_request("POST", "/commands/todos", None, Some(claims));

        let result = extract_user_claims(&request);
        assert!(result.is_ok());

        let claims = result.unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.family_id, Some("family456".to_string()));
    }

    #[test]
    fn test_extract_user_claims_missing_authorizer() {
        let request = create_test_request("POST", "/commands/todos", None, None);

        let result = extract_user_claims(&request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("認証情報が見つかりません"));
    }

    #[test]
    fn test_parse_command_create_todo() {
        let body = json!({
            "title": "テストToDo",
            "description": "テスト説明",
            "tags": ["タグ1", "タグ2"]
        });
        let claims = create_test_claims();
        let request = create_test_request(
            "POST",
            "/commands/todos",
            Some(&body.to_string()),
            Some(claims),
        );

        let result = parse_command(&request);
        assert!(result.is_ok());

        match result.unwrap() {
            Command::CreateTodo {
                title,
                description,
                tags,
            } => {
                assert_eq!(title, "テストToDo");
                assert_eq!(description, Some("テスト説明".to_string()));
                assert_eq!(tags, vec!["タグ1", "タグ2"]);
            }
            _ => panic!("Expected CreateTodo command"),
        }
    }

    #[test]
    fn test_parse_command_update_todo() {
        let todo_id = TodoId::new();
        let body = json!({
            "title": "更新されたタイトル",
            "description": "更新された説明"
        });
        let claims = create_test_claims();
        let request = create_test_request(
            "PUT",
            &format!("/commands/todos/{}", todo_id.as_str()),
            Some(&body.to_string()),
            Some(claims),
        );

        let result = parse_command(&request);
        assert!(result.is_ok());

        match result.unwrap() {
            Command::UpdateTodo {
                todo_id: parsed_id,
                title,
                description,
            } => {
                assert_eq!(parsed_id, todo_id);
                assert_eq!(title, Some("更新されたタイトル".to_string()));
                assert_eq!(description, Some("更新された説明".to_string()));
            }
            _ => panic!("Expected UpdateTodo command"),
        }
    }

    #[test]
    fn test_parse_command_complete_todo() {
        let todo_id = TodoId::new();
        let claims = create_test_claims();
        let request = create_test_request(
            "POST",
            &format!("/commands/todos/{}/complete", todo_id.as_str()),
            None,
            Some(claims),
        );

        let result = parse_command(&request);
        assert!(result.is_ok());

        match result.unwrap() {
            Command::CompleteTodo { todo_id: parsed_id } => {
                assert_eq!(parsed_id, todo_id);
            }
            _ => panic!("Expected CompleteTodo command"),
        }
    }

    #[test]
    fn test_parse_command_delete_todo() {
        let todo_id = TodoId::new();
        let body = json!({
            "reason": "不要になったため"
        });
        let claims = create_test_claims();
        let request = create_test_request(
            "DELETE",
            &format!("/commands/todos/{}", todo_id.as_str()),
            Some(&body.to_string()),
            Some(claims),
        );

        let result = parse_command(&request);
        assert!(result.is_ok());

        match result.unwrap() {
            Command::DeleteTodo {
                todo_id: parsed_id,
                reason,
            } => {
                assert_eq!(parsed_id, todo_id);
                assert_eq!(reason, Some("不要になったため".to_string()));
            }
            _ => panic!("Expected DeleteTodo command"),
        }
    }

    #[test]
    fn test_parse_command_unsupported_method() {
        let claims = create_test_claims();
        let request = create_test_request("GET", "/commands/todos", None, Some(claims));

        let result = parse_command(&request);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("サポートされていないメソッドまたはパス"));
    }

    #[test]
    fn test_extract_todo_id_from_path() {
        let todo_id = TodoId::new();
        let path = format!("/commands/todos/{}", todo_id.as_str());

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
    fn test_extract_todo_id_from_complete_path() {
        let todo_id = TodoId::new();
        let path = format!("/commands/todos/{}/complete", todo_id.as_str());

        let result = extract_todo_id_from_complete_path(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), todo_id);
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
}

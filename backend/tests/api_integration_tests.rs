use lambda_http::{Body, Request, RequestExt, Response};
use std::collections::HashMap;

// テスト対象の関数をインポート
#[path = "../src/http_handler.rs"]
mod http_handler;
use http_handler::function_handler;

// シンプルなAPIリクエストをモックするヘルパー関数
fn create_test_request(query_params: Option<HashMap<String, String>>) -> Request {
    // 基本のリクエストを作成
    let mut request = Request::default();

    // クエリパラメータの設定
    if let Some(params) = query_params {
        request = request.with_query_string_parameters(params);
    }

    request
}

// レスポンスボディをStringに変換するヘルパー関数
fn response_body_to_string(response: &Response<Body>) -> String {
    match response.body() {
        Body::Empty => String::new(),
        Body::Text(text) => text.clone(),
        Body::Binary(binary) => String::from_utf8_lossy(binary).to_string(),
    }
}

#[tokio::test]
async fn test_with_query_param() {
    // クエリパラメータの構築
    let mut query_params = HashMap::new();
    query_params.insert("name".to_string(), "テスト".to_string());

    // リクエストの作成
    let request = create_test_request(Some(query_params));

    // Lambda関数の実行
    let response = function_handler(request)
        .await
        .expect("ハンドラーが失敗しました");

    // レスポンスの検証
    assert_eq!(response.status(), 200);
    let body = response_body_to_string(&response);
    assert_eq!(body, "Hello テスト, this is an AWS Lambda HTTP request");
}

#[tokio::test]
async fn test_without_query_param() {
    // クエリパラメータなしのリクエストを作成
    let request = create_test_request(None);

    // Lambda関数の実行
    let response = function_handler(request)
        .await
        .expect("ハンドラーが失敗しました");

    // レスポンスの検証
    assert_eq!(response.status(), 200);
    let body = response_body_to_string(&response);
    assert_eq!(body, "Hello world, this is an AWS Lambda HTTP request");
}

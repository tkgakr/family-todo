use lambda_http::{Body, Request, RequestExt, Response};

use crate::db::DynamoClient;
use crate::error::ApiError;
use crate::handlers;

pub async fn route(req: Request, db: &DynamoClient) -> Result<Response<Body>, lambda_http::Error> {
    let path = req.uri().path().to_string();
    let method = req.method().as_str().to_string();

    tracing::info!(path = %path, method = %method, "Incoming request");

    let result = match route_inner(req, db, &path, &method).await {
        Ok(mut resp) => {
            add_cors_headers(&mut resp);
            resp
        }
        Err(e) => {
            tracing::error!(error = %e, "Request failed");
            let mut resp = e.into_response();
            add_cors_headers(&mut resp);
            resp
        }
    };

    Ok(result)
}

async fn route_inner(
    req: Request,
    db: &DynamoClient,
    path: &str,
    method: &str,
) -> Result<Response<Body>, ApiError> {
    if method == "OPTIONS" {
        return Ok(Response::builder().status(204).body(Body::Empty).unwrap());
    }

    let (family_id, user_id) = extract_claims(&req)?;

    match (method, path) {
        ("GET", "/todos") => handlers::list_todos(db, &family_id).await,
        ("POST", "/todos") => handlers::create_todo(req, db, &family_id, &user_id).await,
        (_, p) if p.starts_with("/todos/") => {
            let todo_id = &p[7..];
            if todo_id.is_empty() {
                return Err(ApiError::BadRequest("Missing todo ID".to_string()));
            }
            match method {
                "PATCH" => handlers::update_todo(req, db, &family_id, todo_id).await,
                "DELETE" => handlers::delete_todo(db, &family_id, todo_id).await,
                _ => Err(ApiError::NotFound),
            }
        }
        _ => Err(ApiError::NotFound),
    }
}

fn extract_claims(req: &Request) -> Result<(String, String), ApiError> {
    let context = req.request_context_ref();

    // HTTP API v2 with JWT authorizer puts claims in the request context
    if let Some(lambda_http::request::RequestContext::ApiGatewayV2(ctx)) = context {
        if let Some(authorizer) = &ctx.authorizer {
            if let Some(jwt) = &authorizer.jwt {
                let family_id =
                    jwt.claims.get("custom:family_id").cloned().ok_or_else(|| {
                        ApiError::Unauthorized("Missing family_id claim".to_string())
                    })?;
                let user_id = jwt
                    .claims
                    .get("sub")
                    .cloned()
                    .ok_or_else(|| ApiError::Unauthorized("Missing sub claim".to_string()))?;
                return Ok((family_id, user_id));
            }
        }
    }

    Err(ApiError::Unauthorized(
        "Invalid authorization context".to_string(),
    ))
}

fn add_cors_headers(resp: &mut Response<Body>) {
    let headers = resp.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert(
        "Access-Control-Allow-Methods",
        "GET,POST,PATCH,DELETE,OPTIONS".parse().unwrap(),
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        "Content-Type,Authorization".parse().unwrap(),
    );
}

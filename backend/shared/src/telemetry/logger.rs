use serde_json::{json, Value};
use tracing::{event, Level, Span};

pub struct StructuredLogger;

impl StructuredLogger {
    pub fn log_command_start(
        command_name: &str,
        family_id: &str,
        user_id: &str,
        todo_id: Option<&str>,
    ) {
        let mut fields = json!({
            "event_type": "command_start",
            "command": command_name,
            "family_id": family_id,
            "user_id": user_id,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        if let Some(id) = todo_id {
            fields["todo_id"] = json!(id);
        }

        event!(Level::INFO, "{}", fields);
    }

    pub fn log_command_success(
        command_name: &str,
        family_id: &str,
        duration_ms: u64,
        todo_id: Option<&str>,
    ) {
        let mut fields = json!({
            "event_type": "command_success",
            "command": command_name,
            "family_id": family_id,
            "duration_ms": duration_ms,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        if let Some(id) = todo_id {
            fields["todo_id"] = json!(id);
        }

        event!(Level::INFO, "{}", fields);
    }

    pub fn log_command_error(
        command_name: &str,
        family_id: &str,
        error: &str,
        duration_ms: u64,
        todo_id: Option<&str>,
    ) {
        let mut fields = json!({
            "event_type": "command_error",
            "command": command_name,
            "family_id": family_id,
            "error": error,
            "duration_ms": duration_ms,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        if let Some(id) = todo_id {
            fields["todo_id"] = json!(id);
        }

        event!(Level::ERROR, "{}", fields);
    }

    pub fn log_query_start(query_name: &str, family_id: &str, params: Value) {
        let fields = json!({
            "event_type": "query_start",
            "query": query_name,
            "family_id": family_id,
            "parameters": params,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        event!(Level::INFO, "{}", fields);
    }

    pub fn log_query_success(
        query_name: &str,
        family_id: &str,
        duration_ms: u64,
        result_count: usize,
    ) {
        let fields = json!({
            "event_type": "query_success",
            "query": query_name,
            "family_id": family_id,
            "duration_ms": duration_ms,
            "result_count": result_count,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        event!(Level::INFO, "{}", fields);
    }

    pub fn log_event_processed(
        event_type: &str,
        family_id: &str,
        todo_id: &str,
        event_id: &str,
        duration_ms: u64,
    ) {
        let fields = json!({
            "event_type": "event_processed",
            "processed_event_type": event_type,
            "family_id": family_id,
            "todo_id": todo_id,
            "event_id": event_id,
            "duration_ms": duration_ms,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        event!(Level::INFO, "{}", fields);
    }

    pub fn log_snapshot_created(
        family_id: &str,
        todo_id: &str,
        event_count: usize,
        duration_ms: u64,
    ) {
        let fields = json!({
            "event_type": "snapshot_created",
            "family_id": family_id,
            "todo_id": todo_id,
            "event_count": event_count,
            "duration_ms": duration_ms,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        event!(Level::INFO, "{}", fields);
    }
}

pub fn create_span_with_fields(_name: &'static str, family_id: &str, operation: &str) -> Span {
    tracing::info_span!(
        "operation",
        family_id = family_id,
        operation = operation,
        otel.kind = "server"
    )
}

use opentelemetry::global::BoxedSpan;
use opentelemetry::{
    global,
    trace::{SpanKind, Status, TraceContextExt, Tracer},
    Context, KeyValue,
};
use std::time::Instant;
use tracing::instrument;

pub struct TraceHelper;

impl TraceHelper {
    pub fn start_span(name: &str, operation: &str, family_id: &str) -> BoxedSpan {
        let tracer = global::tracer("family-todo-app");
        tracer
            .span_builder(name.to_string())
            .with_kind(SpanKind::Server)
            .with_attributes(vec![
                KeyValue::new("operation.type", operation.to_string()),
                KeyValue::new("family.id", family_id.to_string()),
                KeyValue::new("service.name", "family-todo-app"),
            ])
            .start(&tracer)
    }

    pub fn add_event_to_current_span(name: &str, attributes: Vec<KeyValue>) {
        let context = Context::current();
        let span = context.span();
        span.add_event(name.to_string(), attributes);
    }

    pub fn set_span_status_ok() {
        let context = Context::current();
        let span = context.span();
        span.set_status(Status::Ok);
    }

    pub fn set_span_status_error(message: &str) {
        let context = Context::current();
        let span = context.span();
        span.set_status(Status::error(message.to_string()));
    }

    pub fn add_span_attribute(key: &str, value: &str) {
        let context = Context::current();
        let span = context.span();
        span.set_attribute(KeyValue::new(key.to_string(), value.to_string()));
    }
}

#[instrument(skip_all)]
pub async fn trace_command<F, T, E>(
    command_name: &str,
    family_id: &str,
    todo_id: Option<&str>,
    operation: F,
) -> Result<T, E>
where
    F: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let start_time = Instant::now();

    TraceHelper::add_span_attribute("command.name", command_name);
    TraceHelper::add_span_attribute("family.id", family_id);

    if let Some(id) = todo_id {
        TraceHelper::add_span_attribute("todo.id", id);
    }

    TraceHelper::add_event_to_current_span(
        "command.started",
        vec![KeyValue::new("timestamp", chrono::Utc::now().to_rfc3339())],
    );

    let result = operation.await;
    let duration = start_time.elapsed();

    match &result {
        Ok(_) => {
            TraceHelper::set_span_status_ok();
            TraceHelper::add_event_to_current_span(
                "command.completed",
                vec![
                    KeyValue::new("duration_ms", duration.as_millis() as i64),
                    KeyValue::new("status", "success"),
                ],
            );
        }
        Err(err) => {
            TraceHelper::set_span_status_error(&err.to_string());
            TraceHelper::add_event_to_current_span(
                "command.failed",
                vec![
                    KeyValue::new("duration_ms", duration.as_millis() as i64),
                    KeyValue::new("status", "error"),
                    KeyValue::new("error", err.to_string()),
                ],
            );
        }
    }

    result
}

#[instrument(skip_all)]
pub async fn trace_query<F, T, E>(query_name: &str, family_id: &str, operation: F) -> Result<T, E>
where
    F: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let start_time = Instant::now();

    TraceHelper::add_span_attribute("query.name", query_name);
    TraceHelper::add_span_attribute("family.id", family_id);

    TraceHelper::add_event_to_current_span(
        "query.started",
        vec![KeyValue::new("timestamp", chrono::Utc::now().to_rfc3339())],
    );

    let result = operation.await;
    let duration = start_time.elapsed();

    match &result {
        Ok(_) => {
            TraceHelper::set_span_status_ok();
            TraceHelper::add_event_to_current_span(
                "query.completed",
                vec![
                    KeyValue::new("duration_ms", duration.as_millis() as i64),
                    KeyValue::new("status", "success"),
                ],
            );
        }
        Err(err) => {
            TraceHelper::set_span_status_error(&err.to_string());
            TraceHelper::add_event_to_current_span(
                "query.failed",
                vec![
                    KeyValue::new("duration_ms", duration.as_millis() as i64),
                    KeyValue::new("status", "error"),
                    KeyValue::new("error", err.to_string()),
                ],
            );
        }
    }

    result
}

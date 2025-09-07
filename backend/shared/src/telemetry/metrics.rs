use once_cell::sync::Lazy;
use opentelemetry::{
    global,
    metrics::{Counter, Histogram, Unit},
    KeyValue,
};
use std::time::Instant;

pub struct AppMetrics {
    pub commands_total: Counter<u64>,
    pub command_duration: Histogram<f64>,
    pub queries_total: Counter<u64>,
    pub query_duration: Histogram<f64>,
    pub events_processed_total: Counter<u64>,
    pub event_processing_duration: Histogram<f64>,
    pub snapshots_created_total: Counter<u64>,
    pub snapshot_creation_duration: Histogram<f64>,
    pub concurrent_requests: Counter<u64>,
    pub active_todos_count: Histogram<f64>,
}

static METRICS: Lazy<AppMetrics> = Lazy::new(|| {
    let meter = global::meter("family-todo-app");

    AppMetrics {
        commands_total: meter
            .u64_counter("commands_total")
            .with_description("Total number of commands processed")
            .with_unit(Unit::new("commands"))
            .init(),

        command_duration: meter
            .f64_histogram("command_duration")
            .with_description("Duration of command processing")
            .with_unit(Unit::new("ms"))
            .init(),

        queries_total: meter
            .u64_counter("queries_total")
            .with_description("Total number of queries processed")
            .with_unit(Unit::new("queries"))
            .init(),

        query_duration: meter
            .f64_histogram("query_duration")
            .with_description("Duration of query processing")
            .with_unit(Unit::new("ms"))
            .init(),

        events_processed_total: meter
            .u64_counter("events_processed_total")
            .with_description("Total number of events processed")
            .with_unit(Unit::new("events"))
            .init(),

        event_processing_duration: meter
            .f64_histogram("event_processing_duration")
            .with_description("Duration of event processing")
            .with_unit(Unit::new("ms"))
            .init(),

        snapshots_created_total: meter
            .u64_counter("snapshots_created_total")
            .with_description("Total number of snapshots created")
            .with_unit(Unit::new("snapshots"))
            .init(),

        snapshot_creation_duration: meter
            .f64_histogram("snapshot_creation_duration")
            .with_description("Duration of snapshot creation")
            .with_unit(Unit::new("ms"))
            .init(),

        concurrent_requests: meter
            .u64_counter("concurrent_requests")
            .with_description("Number of concurrent requests")
            .with_unit(Unit::new("requests"))
            .init(),

        active_todos_count: meter
            .f64_histogram("active_todos_count")
            .with_description("Number of active todos per family")
            .with_unit(Unit::new("todos"))
            .init(),
    }
});

pub fn get_metrics() -> &'static AppMetrics {
    &METRICS
}

pub struct MetricTimer {
    start: Instant,
    histogram: &'static Histogram<f64>,
    labels: Vec<KeyValue>,
}

impl MetricTimer {
    pub fn new(histogram: &'static Histogram<f64>, labels: Vec<KeyValue>) -> Self {
        Self {
            start: Instant::now(),
            histogram,
            labels,
        }
    }

    pub fn finish(self) -> u64 {
        let duration = self.start.elapsed();
        let duration_ms = duration.as_millis() as f64;
        self.histogram.record(duration_ms, &self.labels);
        duration.as_millis() as u64
    }
}

pub fn start_command_timer(command: &str, family_id: &str) -> MetricTimer {
    MetricTimer::new(
        &get_metrics().command_duration,
        vec![
            KeyValue::new("command", command.to_string()),
            KeyValue::new("family_id", family_id.to_string()),
        ],
    )
}

pub fn start_query_timer(query: &str, family_id: &str) -> MetricTimer {
    MetricTimer::new(
        &get_metrics().query_duration,
        vec![
            KeyValue::new("query", query.to_string()),
            KeyValue::new("family_id", family_id.to_string()),
        ],
    )
}

pub fn start_event_processing_timer(event_type: &str, family_id: &str) -> MetricTimer {
    MetricTimer::new(
        &get_metrics().event_processing_duration,
        vec![
            KeyValue::new("event_type", event_type.to_string()),
            KeyValue::new("family_id", family_id.to_string()),
        ],
    )
}

pub fn start_snapshot_timer(family_id: &str) -> MetricTimer {
    MetricTimer::new(
        &get_metrics().snapshot_creation_duration,
        vec![KeyValue::new("family_id", family_id.to_string())],
    )
}

pub fn increment_command_counter(command: &str, family_id: &str, status: &str) {
    get_metrics().commands_total.add(
        1,
        &[
            KeyValue::new("command", command.to_string()),
            KeyValue::new("family_id", family_id.to_string()),
            KeyValue::new("status", status.to_string()),
        ],
    );
}

pub fn increment_query_counter(query: &str, family_id: &str) {
    get_metrics().queries_total.add(
        1,
        &[
            KeyValue::new("query", query.to_string()),
            KeyValue::new("family_id", family_id.to_string()),
        ],
    );
}

pub fn increment_event_processed_counter(event_type: &str, family_id: &str) {
    get_metrics().events_processed_total.add(
        1,
        &[
            KeyValue::new("event_type", event_type.to_string()),
            KeyValue::new("family_id", family_id.to_string()),
        ],
    );
}

pub fn increment_snapshot_counter(family_id: &str) {
    get_metrics()
        .snapshots_created_total
        .add(1, &[KeyValue::new("family_id", family_id.to_string())]);
}

pub fn record_active_todos_count(family_id: &str, count: usize) {
    get_metrics().active_todos_count.record(
        count as f64,
        &[KeyValue::new("family_id", family_id.to_string())],
    );
}

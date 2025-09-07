use aws_lambda_events::event::dynamodb::{Event, EventRecord};
use chrono::Utc;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use shared::domain::{
    aggregates::{Todo, TodoSnapshot},
    events::TodoEvent,
    identifiers::{FamilyId, TodoId},
};
use shared::infra::dynamodb::DynamoDbRepository;
use std::env;
use tracing::{error, info, warn};

const SNAPSHOT_THRESHOLD: usize = 50;

async fn function_handler(event: LambdaEvent<Event>) -> Result<(), Error> {
    let table_name = env::var("TABLE_NAME").expect("TABLE_NAME must be set");
    let repository = DynamoDbRepository::new(table_name);

    for record in event.payload.records {
        if let Err(e) = process_record(&repository, record).await {
            error!(error = %e, "Failed to process record");
        }
    }

    Ok(())
}

async fn process_record(
    repository: &DynamoDbRepository,
    record: EventRecord,
) -> anyhow::Result<()> {
    if record.event_name != "INSERT" {
        return Ok(());
    }

    let image = &record.change.new_image;

    let pk = match image.get("PK") {
        Some(serde_dynamo::AttributeValue::S(s)) => s,
        _ => return Ok(()),
    };

    let sk = match image.get("SK") {
        Some(serde_dynamo::AttributeValue::S(s)) => s,
        _ => return Ok(()),
    };

    if !pk.starts_with("FAMILY#") || !sk.starts_with("EVENT#") {
        return Ok(());
    }

    let family_id_str = pk.strip_prefix("FAMILY#").unwrap();
    let family_id = FamilyId::from_string(family_id_str.to_string())?;

    let event_data = match image.get("Data") {
        Some(av) => match av {
            serde_dynamo::AttributeValue::S(s) => s,
            _ => return Err(anyhow::anyhow!("Data is not a string")),
        },
        None => return Err(anyhow::anyhow!("No Data in record")),
    };

    let event: TodoEvent = serde_json::from_str(event_data)?;
    let todo_id = event.todo_id();

    let event_count = repository
        .get_events_for_todo(&family_id, todo_id)
        .await?
        .len();

    if event_count >= SNAPSHOT_THRESHOLD {
        match create_snapshot_for_todo(repository, &family_id, todo_id).await {
            Ok(()) => {
                info!(
                    todo_id = %todo_id.as_str(),
                    family_id = %family_id.as_str(),
                    event_count = event_count,
                    "Snapshot created successfully"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    todo_id = %todo_id.as_str(),
                    family_id = %family_id.as_str(),
                    "Failed to create snapshot"
                );
            }
        }
    }

    Ok(())
}

async fn create_snapshot_for_todo(
    repository: &DynamoDbRepository,
    family_id: &FamilyId,
    todo_id: &TodoId,
) -> anyhow::Result<()> {
    let events = repository.get_events_for_todo(family_id, todo_id).await?;

    if events.is_empty() {
        warn!(todo_id = %todo_id.as_str(), "No events found for todo");
        return Ok(());
    }

    let mut todo = Todo::default();
    let mut last_event_id = String::new();
    let mut stream_version = 0u64;

    for event in events {
        last_event_id = event.event_id().as_str().to_string();
        todo.apply(event);
        stream_version += 1;
    }

    let snapshot = TodoSnapshot {
        todo_id: todo_id.clone(),
        state: todo,
        last_event_id,
        stream_version,
        created_at: Utc::now(),
    };

    repository.save_snapshot(family_id, &snapshot).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}

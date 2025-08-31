use anyhow::{anyhow, Result};
use aws_lambda_events::event::dynamodb::EventRecord as DynamoDbEventRecord;
use shared::{
    domain::{
        events::TodoEvent,
        identifiers::FamilyId,
    },
    infra::EventProcessor,
};
use tracing::{error, info, warn, instrument};

use crate::error_handling::{BatchItemFailures, is_retryable_error};

pub struct StreamProcessor {
    event_processor: EventProcessor,
}

impl StreamProcessor {
    pub fn new(table_name: String) -> Self {
        Self {
            event_processor: EventProcessor::new(table_name),
        }
    }

    #[instrument(skip(self, records))]
    pub async fn process_records(
        &self,
        records: Vec<DynamoDbEventRecord>,
    ) -> Result<BatchItemFailures> {
        let mut failures = BatchItemFailures::new();
        
        for record in records {
            let sequence_number = record.event_id.clone().unwrap_or_else(|| "unknown".to_string());
            
            match self.process_record(&record).await {
                Ok(_) => {
                    info!(
                        sequence_number = %sequence_number,
                        event_name = ?record.event_name,
                        "Successfully processed record"
                    );
                }
                Err(e) if is_retryable_error(&e) => {
                    warn!(
                        sequence_number = %sequence_number,
                        error = %e,
                        "Retryable error occurred"
                    );
                    failures.add_failure(sequence_number);
                }
                Err(e) => {
                    error!(
                        sequence_number = %sequence_number,
                        error = %e,
                        "Non-retryable error occurred"
                    );
                    // For non-retryable errors, we log but don't add to failures
                    // This allows the processing to continue
                    self.send_to_dlq(&record, &e).await;
                }
            }
        }
        
        Ok(failures)
    }

    #[instrument(skip(self, record))]
    async fn process_record(&self, record: &DynamoDbEventRecord) -> Result<()> {
        // Only process INSERT events (new items)
        if record.event_name.as_ref() != Some("INSERT") {
            return Ok(());
        }

        // Extract DynamoDB record
        let dynamodb_record = &record.change;

        // Check if this is an event record
        let new_image = &dynamodb_record.new_image;
        
        let entity_type = new_image
            .get("EntityType")
            .and_then(|v| v.s.as_ref())
            .ok_or_else(|| anyhow!("No EntityType found"))?;

        if entity_type != "Event" {
            return Ok(()); // Not an event, skip
        }

        // Extract the event data
        let event_data = new_image
            .get("Data")
            .and_then(|v| v.s.as_ref())
            .ok_or_else(|| anyhow!("No event data found"))?;

        // Extract primary key to get family_id
        let pk = new_image
            .get("PK")
            .and_then(|v| v.s.as_ref())
            .ok_or_else(|| anyhow!("No partition key found"))?;

        let family_id_str = pk.strip_prefix("FAMILY#")
            .ok_or_else(|| anyhow!("Invalid partition key format"))?;

        let family_id = FamilyId::from_string(family_id_str.to_string())
            .map_err(|e| anyhow!("Invalid family ID: {}", e))?;

        // Parse the event
        let event: TodoEvent = serde_json::from_str(event_data)
            .map_err(|e| anyhow!("Failed to parse event: {}", e))?;

        // Process the event
        self.event_processor.process_event(&family_id, event).await
            .map_err(|e| anyhow!("Failed to process event: {}", e))?;

        Ok(())
    }

    async fn send_to_dlq(&self, record: &DynamoDbEventRecord, error: &anyhow::Error) {
        // In a real implementation, we would send the failed record to a DLQ
        // For now, we just log it
        error!(
            sequence_number = record.event_id.as_ref().unwrap_or("unknown"),
            error = %error,
            record = ?record,
            "Sending record to DLQ"
        );

        // TODO: Implement actual DLQ sending with SQS
        // let dlq_message = serde_json::json!({
        //     "original_record": record,
        //     "error": error.to_string(),
        //     "timestamp": chrono::Utc::now().to_rfc3339(),
        //     "retry_count": 0
        // });
        // 
        // sqs_client.send_message()
        //     .queue_url(&dlq_url)
        //     .message_body(dlq_message.to_string())
        //     .send()
        //     .await;
    }
}
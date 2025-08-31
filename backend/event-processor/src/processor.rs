use anyhow::Result;
use aws_lambda_events::event::dynamodb::EventRecord as DynamoDbEventRecord;
use shared::infra::EventProcessor;
use tracing::{error, info, warn, instrument};

use crate::error_handling::{BatchItemFailures, is_retryable_error};

pub struct StreamProcessor {
    #[allow(dead_code)]
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
            let sequence_number = record.event_id.clone();
            
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
                    failures.add_failure(sequence_number.clone());
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
        if record.event_name != "INSERT" {
            return Ok(());
        }

        // For now, skip processing as DynamoDB AttributeValue handling needs proper setup
        // This would require either:
        // 1. Using aws-sdk-dynamodb types
        // 2. Implementing custom AttributeValue deserialization
        // 3. Converting the record to a simpler format
        
        // Placeholder implementation - in real scenario we would:
        // - Extract EntityType from new_image
        // - Check if it's an Event
        // - Extract Data and PK fields
        // - Parse the event and process it
        
        info!("DynamoDB record processing skipped - AttributeValue handling needs implementation");

        Ok(())
    }

    async fn send_to_dlq(&self, record: &DynamoDbEventRecord, error: &anyhow::Error) {
        // In a real implementation, we would send the failed record to a DLQ
        // For now, we just log it
        error!(
            sequence_number = record.event_id,
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
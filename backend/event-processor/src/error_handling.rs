use serde::{Deserialize, Serialize};
use shared::domain::error::ProcessError;

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchItemFailures {
    #[serde(rename = "batchItemFailures")]
    pub batch_item_failures: Vec<BatchItemFailure>,
}

impl BatchItemFailures {
    pub fn new() -> Self {
        Self {
            batch_item_failures: Vec::new(),
        }
    }

    pub fn add_failure(&mut self, sequence_number: String) {
        self.batch_item_failures.push(BatchItemFailure {
            item_identifier: sequence_number,
        });
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchItemFailure {
    #[serde(rename = "itemIdentifier")]
    pub item_identifier: String,
}

pub fn is_retryable_error(error: &anyhow::Error) -> bool {
    let error_string = error.to_string().to_lowercase();

    // Check for retryable errors
    error_string.contains("throttling")
        || error_string.contains("service unavailable")
        || error_string.contains("internal server error")
        || error_string.contains("timeout")
        || error_string.contains("connection")
        || error_string.contains("network")
}

#[allow(dead_code)]
pub fn is_retryable(error: &ProcessError) -> bool {
    matches!(
        error,
        ProcessError::TemporaryFailure(_)
            | ProcessError::ThrottlingException(_)
            | ProcessError::ServiceUnavailable(_)
    )
}

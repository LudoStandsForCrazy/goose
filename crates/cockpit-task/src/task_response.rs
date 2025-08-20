use goose::conversation::message::Message;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Response message structure for task execution results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    /// Request identifier
    pub request_id: String,

    /// Execution status
    pub status: TaskStatus,

    /// All messages from the conversation
    pub messages: Vec<Message>,

    /// Final response content (last assistant message)
    pub final_response: Option<String>,

    /// Structured output if final_output_tool was used
    pub structured_output: Option<serde_json::Value>,

    /// Error information if execution failed
    pub error: Option<String>,

    /// Execution metadata
    pub metadata: TaskMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Success,
    Error,
    Timeout,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// Session ID that was created
    pub session_id: String,

    /// Number of agent turns taken
    pub turns_taken: u32,

    /// Number of tool calls made
    pub tool_calls_made: u32,

    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Token usage if available
    pub total_tokens: Option<u32>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,

    /// Timestamp of execution
    pub timestamp: u64,

    /// Whether final output tool was called
    pub final_output_called: bool,

    /// Working directory used
    pub working_dir: Option<String>,
}

impl TaskResponse {
    /// Create a successful response
    pub fn success(
        request_id: String,
        messages: Vec<Message>,
        final_response: Option<String>,
        structured_output: Option<serde_json::Value>,
        metadata: TaskMetadata,
    ) -> Self {
        Self {
            request_id,
            status: TaskStatus::Success,
            messages,
            final_response,
            structured_output,
            error: None,
            metadata,
        }
    }

    /// Create an error response
    pub fn error(request_id: String, error: String) -> Self {
        Self {
            request_id: request_id.clone(),
            status: TaskStatus::Error,
            messages: vec![],
            final_response: None,
            structured_output: None,
            error: Some(error),
            metadata: TaskMetadata {
                session_id: format!("error_{}", request_id),
                turns_taken: 0,
                tool_calls_made: 0,
                execution_time_ms: 0,
                total_tokens: None,
                input_tokens: None,
                output_tokens: None,
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                final_output_called: false,
                working_dir: None,
            },
        }
    }

    /// Create a timeout response
    pub fn timeout(request_id: String) -> Self {
        Self {
            request_id: request_id.clone(),
            status: TaskStatus::Timeout,
            messages: vec![],
            final_response: None,
            structured_output: None,
            error: Some("Task execution timed out".to_string()),
            metadata: TaskMetadata {
                session_id: format!("timeout_{}", request_id),
                turns_taken: 0,
                tool_calls_made: 0,
                execution_time_ms: 0,
                total_tokens: None,
                input_tokens: None,
                output_tokens: None,
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                final_output_called: false,
                working_dir: None,
            },
        }
    }

    /// Check if the task completed successfully
    pub fn is_success(&self) -> bool {
        matches!(self.status, TaskStatus::Success)
    }

    /// Check if the task failed
    pub fn is_error(&self) -> bool {
        matches!(self.status, TaskStatus::Error)
    }
}

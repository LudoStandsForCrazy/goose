use goose::config::ExtensionConfig;
use goose::recipe::Response;
use serde::{Deserialize, Serialize};

/// Request message structure for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    /// The prompt/instructions to execute
    pub prompt: String,

    /// Extension configuration
    pub extensions: Option<Vec<String>>,
    pub remote_extensions: Option<Vec<String>>,
    pub streamable_http_extensions: Option<Vec<String>>,
    pub builtins: Option<Vec<String>>,
    pub extensions_override: Option<Vec<ExtensionConfig>>,

    /// System prompt additions
    pub additional_system_prompt: Option<String>,

    /// Tool and execution limits
    pub max_tool_repetitions: Option<u32>,
    pub max_turns: Option<u32>,

    /// Recipe-specific features
    pub final_output_response: Option<Response>,

    /// Working directory for the task

    /// Metadata
    pub task_id: Option<String>,
    pub timeout_secs: Option<u64>,
}

impl TaskRequest {
    /// Create a simple task request with just a prompt
    pub fn simple(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            extensions: None,
            remote_extensions: None,
            streamable_http_extensions: None,
            builtins: None,
            extensions_override: None,
            additional_system_prompt: None,
            max_tool_repetitions: None,
            max_turns: None,
            final_output_response: None,
            task_id: None,
            timeout_secs: None,
        }
    }

    /// Set structured output response
    pub fn with_structured_output(mut self, response: Response) -> Self {
        self.final_output_response = Some(response);
        self
    }

    /// Set maximum execution turns
    pub fn with_max_turns(mut self, max_turns: u32) -> Self {
        self.max_turns = Some(max_turns);
        self
    }

    /// Set task timeout
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = Some(timeout_secs);
        self
    }
}

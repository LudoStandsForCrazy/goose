use anyhow::Result;
use goose::conversation::message::{Message, MessageContent};
use goose_cli::session::Session;
use rmcp::model::Role;
use std::time::{Instant, SystemTime};
use tracing::{info, warn};

use crate::task_response::{TaskMetadata, TaskResponse};

pub struct AgentExecutor {
    session: Session,
}

impl AgentExecutor {
    pub fn new(session: Session) -> Self {
        Self { session }
    }

    pub async fn execute(&mut self, prompt: String, request_id: &str) -> Result<TaskResponse> {
        let start_time = Instant::now();
        let session_id = format!("task_{}", request_id);

        info!("Starting agent execution for request {}", request_id);

        let result = self.session.headless(prompt).await;

        let execution_time = start_time.elapsed();

        match result {
            Ok(_) => {
                info!(
                    "Agent execution completed successfully for request {}",
                    request_id
                );

                // Get the actual messages from the session after execution
                let conversation = self.session.message_history().clone();
                let messages = conversation.messages();
                let (turns_taken, tool_calls_made) = self.count_interactions(&messages);

                // Extract final response
                let final_response = self.extract_final_response(&messages);

                // Check for structured output (final output tool)
                let (structured_output, final_output_called) =
                    self.extract_structured_output(&messages);

                let metadata = TaskMetadata {
                    session_id,
                    turns_taken,
                    tool_calls_made,
                    execution_time_ms: execution_time.as_millis() as u64,
                    total_tokens: None, // TODO: Extract from session if available
                    input_tokens: None,
                    output_tokens: None,
                    timestamp: SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    final_output_called,
                    working_dir: std::env::current_dir()
                        .ok()
                        .map(|p| p.to_string_lossy().to_string()),
                };

                Ok(TaskResponse::success(
                    request_id.to_string(),
                    messages.to_vec(),
                    final_response,
                    structured_output,
                    metadata,
                ))
            }
            Err(e) => {
                warn!("Agent execution failed for request {}: {}", request_id, e);
                Ok(TaskResponse::error(
                    request_id.to_string(),
                    format!("Agent execution failed: {}", e),
                ))
            }
        }
    }

    /// Count the number of turns and tool calls in the conversation
    fn count_interactions(&self, messages: &[Message]) -> (u32, u32) {
        let mut turns = 0u32;
        let mut tool_calls = 0u32;

        for message in messages {
            match message.role {
                Role::Assistant => {
                    turns += 1;
                    // Count tool requests in this message
                    for content in &message.content {
                        if let MessageContent::ToolRequest(_) = content {
                            tool_calls += 1;
                        }
                    }
                }
                _ => {}
            }
        }

        (turns, tool_calls)
    }

    /// Extract the final response text from the last assistant message
    fn extract_final_response(&self, messages: &[Message]) -> Option<String> {
        println!("Extracting final response");
        println!("Messages: {:?}", messages);
        messages
            .iter()
            .rev()
            .find(|msg| msg.role == Role::Assistant)
            .and_then(|msg| {
                // Get text content from the message
                msg.content.iter().find_map(|content| match content {
                    MessageContent::Text(text) => Some(text.text.clone()),
                    _ => None,
                })
            })
    }

    /// Extract structured output from final output tool calls
    fn extract_structured_output(&self, messages: &[Message]) -> (Option<serde_json::Value>, bool) {
        for message in messages.iter().rev() {
            if message.role == Role::User {
                for content in &message.content {
                    if let MessageContent::ToolResponse(tool_response) = content {
                        // Check if this is a final output tool response
                        if tool_response.id.contains("final_output") {
                            if let Ok(tool_result) = &tool_response.tool_result {
                                // Try to parse the structured output
                                for result_content in tool_result {
                                    if let Some(text) = result_content.as_text() {
                                        // The final output tool returns JSON as text
                                        if let Ok(json_value) =
                                            serde_json::from_str::<serde_json::Value>(&text.text)
                                        {
                                            return (Some(json_value), true);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        (None, false)
    }
}

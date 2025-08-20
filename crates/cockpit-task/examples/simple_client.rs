use anyhow::Result;
use async_nats::{Client, ConnectOptions};
use serde_json;

// For this example, we'll define the types inline since they're external
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub prompt: String,
    pub recipe_content: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
}

impl TaskRequest {
    pub fn simple(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            recipe_content: None,
            provider: None,
            model: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub request_id: String,
    pub status: TaskStatus,
    pub messages: Vec<serde_json::Value>, // Simplified for example
    pub final_response: Option<String>,
    pub structured_output: Option<serde_json::Value>,
    pub error: Option<String>,
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
    pub session_id: String,
    pub turns_taken: u32,
    pub tool_calls_made: u32,
    pub execution_time_ms: u64,
    pub final_output_called: bool,
}
use std::time::Duration;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to NATS
    //
    let connect_options =
        ConnectOptions::with_user_and_password("app-user".into(), "secret".into())
            .connection_timeout(Duration::from_secs(600))
            .request_timeout(Some(Duration::from_secs(600)));
    let client = async_nats::connect_with_options("nats://localhost:4222", connect_options).await?;

    // Create a simple task request
    let request =
        TaskRequest::simple("Tell me the list of the past 10 world cup football finalist, without any tool, just your modal knowledge");

    // Serialize the request
    let request_payload = serde_json::to_vec(&request)?;

    // Send request and wait for response with timeout
    println!("Sending request...");
    let response_msg = client
        .request("prod.goose.task", request_payload.into())
        .await?;

    // Parse the response
    let response: TaskResponse = serde_json::from_slice(&response_msg.payload)?;

    // Display results
    println!("Task Status: {:?}", response.status);
    println!("Request ID: {}", response.request_id);

    if let Some(final_response) = response.final_response {
        println!("Final Response:\n{}", final_response);
    }

    if let Some(error) = response.error {
        println!("Error: {}", error);
    }

    println!("\nMetadata:");
    println!("  Session ID: {}", response.metadata.session_id);
    println!("  Turns Taken: {}", response.metadata.turns_taken);
    println!("  Tool Calls: {}", response.metadata.tool_calls_made);
    println!(
        "  Execution Time: {}ms",
        response.metadata.execution_time_ms
    );
    println!(
        "  Final Output Called: {}",
        response.metadata.final_output_called
    );

    println!("\nConversation Messages: {} total", response.messages.len());

    Ok(())
}

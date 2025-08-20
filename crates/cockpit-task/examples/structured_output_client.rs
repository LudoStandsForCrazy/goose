use anyhow::Result;
use async_nats::{Client, ConnectOptions};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub prompt: String,
    pub final_output_response: Option<Value>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub max_turns: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub request_id: String,
    pub status: String,
    pub final_response: Option<String>,
    pub structured_output: Option<Value>,
    pub error: Option<String>,
    pub metadata: Value,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Structured Output Example");
    println!("========================");

    let connect_options =
        ConnectOptions::with_user_and_password("app-user".into(), "secret".into());
    let client = async_nats::connect_with_options("nats://localhost:4222", connect_options).await?;

    // Create a task request with structured output
    let request = TaskRequest {
        prompt: "Tell me the list of the past 10 world cup football finalist".to_string(),
        final_output_response: Some(json!({
            "json_schema": {
                "type": "object",
                "properties": {
                    "finalists": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "team_a": {"type": "string"},
                                "team_b": {"type": "string"},
                                "score": {"type": "string"}
                            }
                        },
                        "maxItems": 5
                    }
                },
                "required": ["finalists"]
            }
        })),
        provider: None,
        model: None,
        max_turns: None,
    };

    // Serialize and send the request
    let request_payload = serde_json::to_vec(&request)?;

    println!("Sending structured output request...");
    let response_msg = client
        .request("cockpit.task", request_payload.into())
        .await?;

    // Parse the response
    let response: TaskResponse = serde_json::from_slice(&response_msg.payload)?;

    // Display results
    println!("\nTask Status: {}", response.status);

    if let Some(error) = response.error {
        println!("Error: {}", error);
        return Ok(());
    }

    if let Some(final_response) = response.final_response {
        println!("\nFinal Response:\n{}", final_response);
    }

    if let Some(structured_output) = response.structured_output {
        println!("\nStructured Output:");
        println!("{}", serde_json::to_string_pretty(&structured_output)?);

        // Extract specific fields
        if let Some(summary) = structured_output.get("summary").and_then(|v| v.as_str()) {
            println!("\nSummary: {}", summary);
        }

        if let Some(file_counts) = structured_output.get("file_counts") {
            println!("\nFile Counts:");
            if let Some(total) = file_counts.get("total_files").and_then(|v| v.as_u64()) {
                println!("  Total Files: {}", total);
            }
            if let Some(rust) = file_counts.get("rust_files").and_then(|v| v.as_u64()) {
                println!("  Rust Files: {}", rust);
            }
            if let Some(md) = file_counts.get("markdown_files").and_then(|v| v.as_u64()) {
                println!("  Markdown Files: {}", md);
            }
        }

        if let Some(largest_files) = structured_output
            .get("largest_files")
            .and_then(|v| v.as_array())
        {
            println!("\nLargest Files:");
            for file in largest_files {
                if let (Some(name), Some(size)) = (
                    file.get("name").and_then(|v| v.as_str()),
                    file.get("size_kb").and_then(|v| v.as_f64()),
                ) {
                    println!("  {} - {:.1} KB", name, size);
                }
            }
        }
    } else {
        println!("\nNo structured output received (final_output_tool may not have been called)");
    }

    println!("\nExecution completed successfully!");
    Ok(())
}

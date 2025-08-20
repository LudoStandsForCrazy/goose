# Cockpit Task - NATS-based Goose Agent Executor

A service that executes Goose agent tasks via NATS request-response pattern. This allows you to run Goose agents in a distributed, non-interactive mode for recipes and structured output tasks.

## Features

- **NATS Integration**: Uses NATS request-response pattern for task distribution
- **Non-Interactive Execution**: Runs agents autonomously without user interaction
- **Recipe Support**: Execute recipes with structured output
- **Configurable Sessions**: Full control over agent configuration via request messages
- **Timeout Handling**: Configurable timeouts for task execution
- **Structured Responses**: Returns detailed execution results including messages, metadata, and structured output

## Architecture

```
Client Request → NATS → Cockpit Task → Goose Agent → Session → Response
```

The service:
1. Listens on a NATS subject for task requests
2. Parses the request and creates a Goose session using `build_session`
3. Executes the agent autonomously 
4. Collects all results (messages, structured output, metadata)
5. Responds back via NATS

## Configuration

Environment variables:
- `NATS_URL`: NATS server URL (default: `nats://localhost:4222`)
- `NATS_SUBJECT`: Subject to listen on (default: `cockpit.task`)
- `REQUEST_TIMEOUT_SECS`: Task timeout in seconds (default: `300`)
- `MAX_TURNS`: Maximum agent turns (default: `50`)

## Task Request Format

```json
{
  "prompt": "Analyze the current directory and create a summary",
  "provider": "anthropic",
  "model": "claude-3-7-sonnet-20241022",
  "max_turns": 10,
  "extensions": ["developer"],
  "final_output_response": {
    "json_schema": {
      "type": "object",
      "properties": {
        "summary": {"type": "string"},
        "file_count": {"type": "number"}
      }
    }
  }
}
```

## Task Response Format

```json
{
  "request_id": "uuid",
  "status": "Success",
  "messages": [...],
  "final_response": "Analysis complete...",
  "structured_output": {
    "summary": "Found 42 files...",
    "file_count": 42
  },
  "metadata": {
    "session_id": "task_uuid",
    "turns_taken": 3,
    "tool_calls_made": 5,
    "execution_time_ms": 15000,
    "final_output_called": true
  }
}
```

## Usage

### Running the Service

```bash
# Set configuration
export NATS_URL="nats://localhost:4222"
export NATS_SUBJECT="cockpit.task"

# Run the service
cargo run --bin cockpit-task
```

### Sending Requests

Using NATS CLI:
```bash
# Simple task
nats request cockpit.task '{"prompt": "List files in current directory"}'

# Recipe with structured output
nats request cockpit.task '{
  "prompt": "Analyze the codebase structure",
  "final_output_response": {
    "json_schema": {
      "type": "object", 
      "properties": {
        "languages": {"type": "array", "items": {"type": "string"}},
        "total_files": {"type": "number"}
      }
    }
  }
}'
```

## Request Types

### Simple Text Task
```json
{
  "prompt": "What files are in the current directory?"
}
```

### Recipe with Structured Output
```json
{
  "prompt": "Analyze the project structure",
  "final_output_response": {
    "json_schema": {
      "type": "object",
      "properties": {
        "summary": {"type": "string"},
        "technologies": {"type": "array", "items": {"type": "string"}}
      }
    }
  }
}
```

### Custom Provider/Model
```json
{
  "prompt": "Review this code",
  "provider": "openai", 
  "model": "gpt-4",
  "extensions": ["developer"],
  "max_turns": 5
}
```

## Building

```bash
cargo build --release
```

## Testing

Start NATS server:
```bash
nats-server
```

Run the service:
```bash
cargo run
```

Send test request:
```bash
nats request cockpit.task '{"prompt": "Hello, what can you help me with?"}'
```

## Integration

This service can be integrated into larger systems for:
- **Distributed Task Processing**: Scale agent execution across multiple nodes
- **API Backends**: Provide agent capabilities via REST APIs
- **Workflow Orchestration**: Chain multiple agent tasks together
- **Batch Processing**: Process multiple tasks concurrently

The NATS request-response pattern makes it easy to integrate with any language or system that supports NATS.
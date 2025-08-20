use anyhow::Result;
use async_nats::{Client, ConnectOptions, Message as NatsMessage};
use futures::StreamExt;
use goose::session;
use goose_cli::session::{build_session, SessionBuilderConfig};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

mod agent_executor;
mod task_request;
mod task_response;

use agent_executor::AgentExecutor;
use task_request::TaskRequest;
use task_response::TaskResponse;

#[derive(Debug, Clone)]
pub struct CockpitTaskConfig {
    pub nats_url: String,
    pub subject: String,
    pub request_timeout: Duration,
    pub max_turns: Option<u32>,
    pub max_concurrent_tasks: usize,
}

#[derive(Debug)]
struct TaskMessage {
    request_msg: NatsMessage,
    reply_subject: String,
    config: CockpitTaskConfig,
    client: Client,
}

pub struct CockpitTaskService {
    config: CockpitTaskConfig,
    client: Client,
    task_sender: mpsc::Sender<TaskMessage>,
    cancellation_token: CancellationToken,
}

impl CockpitTaskService {
    pub async fn new(config: CockpitTaskConfig) -> Result<Self> {
        let connect_options =
            ConnectOptions::with_user_and_password("app-user".into(), "secret".into());
        let client = async_nats::connect_with_options(&config.nats_url, connect_options).await?;
        info!("Connected to NATS at {}", config.nats_url);

        // Create bounded channel for task backpressure
        let (task_sender, task_receiver) = mpsc::channel::<TaskMessage>(config.max_concurrent_tasks);
        let cancellation_token = CancellationToken::new();

        // Spawn worker pool
        Self::spawn_worker_pool(task_receiver, config.max_concurrent_tasks, cancellation_token.clone()).await;

        Ok(Self {
            config,
            client,
            task_sender,
            cancellation_token,
        })
    }

    async fn spawn_worker_pool(
        task_receiver: mpsc::Receiver<TaskMessage>,
        worker_count: usize,
        cancellation_token: CancellationToken,
    ) {
        info!("Starting worker pool with {} workers", worker_count);

        // Share the receiver among workers using Arc<Mutex<Receiver>>
        let receiver = Arc::new(tokio::sync::Mutex::new(task_receiver));

        for worker_id in 0..worker_count {
            let receiver = receiver.clone();
            let token = cancellation_token.clone();

            tokio::spawn(async move {
                info!("Worker {} started", worker_id);

                loop {
                    tokio::select! {
                        task = async {
                            let mut guard = receiver.lock().await;
                            guard.recv().await
                        } => {
                            match task {
                                Some(task_msg) => {
                                    if let Err(e) = Self::handle_request(
                                        task_msg.request_msg,
                                        task_msg.reply_subject,
                                        task_msg.config,
                                        task_msg.client,
                                    ).await {
                                        error!("Worker {} error handling request: {}", worker_id, e);
                                    }
                                }
                                None => {
                                    info!("Worker {} shutting down - channel closed", worker_id);
                                    break;
                                }
                            }
                        }
                        _ = token.cancelled() => {
                            info!("Worker {} shutting down - cancellation requested", worker_id);
                            break;
                        }
                    }
                }
            });
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting Cockpit Task Service on subject: {} (max concurrent: {})",
            self.config.subject, self.config.max_concurrent_tasks
        );

        let subject = self.config.subject.clone();
        let mut subscriber = self.client.subscribe(subject).await?;

        loop {
            tokio::select! {
                message = subscriber.next() => {
                    match message {
                        Some(request_msg) => {
                            if let Some(reply_subject) = request_msg.reply.clone() {
                                let task_message = TaskMessage {
                                    request_msg,
                                    reply_subject: reply_subject.to_string(),
                                    config: self.config.clone(),
                                    client: self.client.clone(),
                                };

                                // Send to worker pool with backpressure
                                if let Err(_) = self.task_sender.send(task_message).await {
                                    error!("Failed to send task to worker pool - shutting down");
                                    break;
                                }
                            } else {
                                warn!("Received message without reply subject, ignoring");
                            }
                        }
                        None => {
                            warn!("NATS subscription closed");
                            break;
                        }
                    }
                }
                _ = self.cancellation_token.cancelled() => {
                    info!("Received shutdown signal, stopping message processing");
                    break;
                }
            }
        }

        // Graceful shutdown
        self.graceful_shutdown().await;
        Ok(())
    }

    async fn graceful_shutdown(&self) {
        info!("Starting graceful shutdown...");

        // Stop accepting new tasks
        drop(self.task_sender.clone());

        // Cancel all workers
        self.cancellation_token.cancel();

        // Give workers time to finish current tasks
        tokio::time::sleep(Duration::from_secs(5)).await;
        info!("Graceful shutdown completed");
    }

    pub fn shutdown(&self) {
        info!("Initiating shutdown...");
        self.cancellation_token.cancel();
    }

    async fn handle_request(
        request_msg: NatsMessage,
        reply_subject: String,
        config: CockpitTaskConfig,
        client: Client,
    ) -> Result<()> {
        let request_id = Uuid::new_v4().to_string();
        info!("Processing request {}", request_id);

        // Parse the request
        let task_request: TaskRequest = serde_json::from_slice(&request_msg.payload)?;

        // Execute the task with timeout
        let response = Self::execute_task(task_request, &config, &request_id).await;

        // Send the response
        let response_payload = serde_json::to_vec(&response)?;
        client
            .publish(reply_subject, response_payload.into())
            .await?;

        info!("Completed request {}", request_id);
        Ok(())
    }

    async fn execute_task(
        request: TaskRequest,
        config: &CockpitTaskConfig,
        request_id: &str,
    ) -> TaskResponse {
        match Self::execute_task_internal(request, config, request_id).await {
            Ok(response) => response,
            Err(e) => {
                error!("Task execution failed: {}", e);
                TaskResponse::error(request_id.to_string(), e.to_string())
            }
        }
    }

    async fn execute_task_internal(
        request: TaskRequest,
        _config: &CockpitTaskConfig,
        request_id: &str,
    ) -> Result<TaskResponse> {
        info!("Executing task for request {}", request_id);

        // Create the session using build_session
        let session_config = SessionBuilderConfig {
            identifier: Some(session::Identifier::Name(format!("task_{}", request_id))),
            resume: false,
            no_session: false,
            extensions: Vec::new(),
            remote_extensions: Vec::new(),
            streamable_http_extensions: Vec::new(),
            builtins: Vec::new(),
            extensions_override: None,
            additional_system_prompt: None,
            settings: None,
            provider: None,
            model: None,
            debug: false,
            max_tool_repetitions: None,
            max_turns: None,
            scheduled_job_id: None,
            interactive: false, // Always non-interactive
            quiet: true,
            sub_recipes: None,
            final_output_response: request.final_output_response,
            retry_config: None,
        };

        let session = build_session(session_config).await;

        // Execute the agent
        let mut executor = AgentExecutor::new(session);
        let result = executor.execute(request.prompt, request_id).await?;

        Ok(result)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load config from environment or use defaults
    let config = CockpitTaskConfig {
        nats_url: std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string()),
        subject: std::env::var("NATS_SUBJECT").unwrap_or_else(|_| "prod.goose.task".to_string()),
        request_timeout: Duration::from_secs(
            std::env::var("REQUEST_TIMEOUT_SECS")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
        ),
        max_turns: std::env::var("MAX_TURNS").ok().and_then(|s| s.parse().ok()),
        max_concurrent_tasks: std::env::var("MAX_CONCURRENT_TASKS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or(10),
    };

    info!("Starting Cockpit Task Service with config: {:?}", config);

    let service = Arc::new(CockpitTaskService::new(config).await?);

    // Setup signal handling for graceful shutdown
    let service_clone = service.clone();
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => info!("Received SIGTERM"),
            _ = sigint.recv() => info!("Received SIGINT"),
        }

        service_clone.shutdown();
    });

    service.start().await?;

    Ok(())
}

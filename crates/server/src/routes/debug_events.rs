use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};

use crate::DeploymentImpl;

/// Debug event types for workflow monitoring
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DebugEvent {
    /// Task entered a new column
    TaskColumnChanged {
        task_id: String,
        task_title: String,
        from_column: Option<String>,
        to_column: String,
        column_has_agent: bool,
        agent_name: Option<String>,
    },
    /// Attempt/workspace created
    AttemptCreated {
        task_id: String,
        workspace_id: String,
        branch: String,
        reusing_existing: bool,
    },
    /// Agent execution starting
    AgentStarting {
        task_id: String,
        workspace_id: String,
        agent_name: String,
        executor: String,
        system_prompt_length: usize,
        system_prompt_preview: String, // First 200 chars
        start_command_length: Option<usize>,
        start_command_preview: Option<String>, // First 200 chars
        column_name: String,
    },
    /// Full prompt being sent to the agent (system prompt + task + start command)
    FullPromptBuilt {
        task_id: String,
        workspace_id: String,
        agent_name: String,
        full_prompt_length: usize,
        full_prompt: String, // The complete prompt being sent
    },
    /// Agent execution started (container/process running)
    AgentStarted {
        task_id: String,
        workspace_id: String,
        session_id: String,
    },
    /// Commit detected
    CommitMade {
        task_id: String,
        workspace_id: String,
        commit_hash: String,
        commit_message: String,
    },
    /// Agent execution completed
    AgentCompleted {
        task_id: String,
        workspace_id: String,
        session_id: String,
        success: bool,
    },
    /// Decision file read
    DecisionFileRead {
        task_id: String,
        workspace_id: String,
        decision: Option<serde_json::Value>,
    },
    /// Auto-transition triggered
    AutoTransition {
        task_id: String,
        from_column: String,
        to_column: String,
        reason: String,
    },
    /// Generic info message
    Info {
        message: String,
        context: Option<serde_json::Value>,
    },
    /// Warning
    Warn {
        message: String,
        context: Option<serde_json::Value>,
    },
    /// Error
    Error {
        message: String,
        context: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugEventEnvelope {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    #[serde(flatten)]
    pub event: DebugEvent,
}

impl DebugEventEnvelope {
    pub fn new(event: DebugEvent) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event,
        }
    }
}

/// Global debug event broadcaster
static DEBUG_TX: std::sync::OnceLock<broadcast::Sender<DebugEventEnvelope>> = std::sync::OnceLock::new();

/// Get or create the debug event broadcaster
pub fn debug_broadcaster() -> &'static broadcast::Sender<DebugEventEnvelope> {
    DEBUG_TX.get_or_init(|| {
        let (tx, _) = broadcast::channel(256);
        tx
    })
}

/// Emit a debug event (can be called from anywhere)
pub fn emit_debug_event(event: DebugEvent) {
    let envelope = DebugEventEnvelope::new(event);
    // Ignore send errors (no subscribers)
    let _ = debug_broadcaster().send(envelope);
}

/// Convenience macros for emitting debug events
#[macro_export]
macro_rules! debug_info {
    ($msg:expr) => {
        $crate::routes::debug_events::emit_debug_event(
            $crate::routes::debug_events::DebugEvent::Info {
                message: $msg.to_string(),
                context: None,
            }
        )
    };
    ($msg:expr, $ctx:expr) => {
        $crate::routes::debug_events::emit_debug_event(
            $crate::routes::debug_events::DebugEvent::Info {
                message: $msg.to_string(),
                context: Some(serde_json::json!($ctx)),
            }
        )
    };
}

#[macro_export]
macro_rules! debug_warn {
    ($msg:expr) => {
        $crate::routes::debug_events::emit_debug_event(
            $crate::routes::debug_events::DebugEvent::Warn {
                message: $msg.to_string(),
                context: None,
            }
        )
    };
    ($msg:expr, $ctx:expr) => {
        $crate::routes::debug_events::emit_debug_event(
            $crate::routes::debug_events::DebugEvent::Warn {
                message: $msg.to_string(),
                context: Some(serde_json::json!($ctx)),
            }
        )
    };
}

#[macro_export]
macro_rules! debug_error {
    ($msg:expr) => {
        $crate::routes::debug_events::emit_debug_event(
            $crate::routes::debug_events::DebugEvent::Error {
                message: $msg.to_string(),
                context: None,
            }
        )
    };
    ($msg:expr, $ctx:expr) => {
        $crate::routes::debug_events::emit_debug_event(
            $crate::routes::debug_events::DebugEvent::Error {
                message: $msg.to_string(),
                context: Some(serde_json::json!($ctx)),
            }
        )
    };
}

/// WebSocket handler for debug events
async fn debug_events_ws(
    ws: WebSocketUpgrade,
    State(_deployment): State<DeploymentImpl>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_debug_socket)
}

async fn handle_debug_socket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = debug_broadcaster().subscribe();

    // Spawn task to send events to client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let json = serde_json::to_string(&event).unwrap_or_default();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages (just for keepalive/close)
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(data)) => {
                // Pong is handled automatically by axum
                let _ = data;
            }
            Err(_) => break,
            _ => {}
        }
    }

    send_task.abort();
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().route("/debug/events/ws", get(debug_events_ws))
}

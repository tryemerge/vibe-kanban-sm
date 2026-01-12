use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, SqlitePool, Type};
use strum_macros::{Display, EnumString};
use ts_rs::TS;
use uuid::Uuid;

/// Type of task event for workflow tracking
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display)]
#[sqlx(type_name = "event_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TaskEventType {
    /// Task entered a new column
    ColumnEnter,
    /// Task exited a column
    ColumnExit,
    /// Agent started execution
    AgentStart,
    /// Agent completed successfully
    AgentComplete,
    /// Agent execution failed
    AgentFailed,
    /// Git commit made
    Commit,
    /// Manual action by user
    ManualAction,
    /// Task created
    TaskCreated,
    /// Task status changed
    StatusChange,
    /// Transition took the else path (condition didn't match)
    /// Used for counting failures toward escalation
    ElseTransition,
}

/// What triggered this event
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display)]
#[sqlx(type_name = "trigger_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EventTriggerType {
    /// Manually triggered by user
    Manual,
    /// Triggered by automation rule
    Automation,
    /// Triggered by drag and drop
    DragDrop,
    /// Triggered by system (e.g., on task creation)
    System,
}

/// Who/what caused this event
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS, EnumString, Display, Default)]
#[sqlx(type_name = "actor_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ActorType {
    /// Human user
    User,
    /// AI agent
    Agent,
    /// System/automation
    #[default]
    System,
}

/// A task event for workflow history tracking
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskEvent {
    pub id: Uuid,
    pub task_id: Uuid,
    pub event_type: TaskEventType,

    // Column transition context
    pub from_column_id: Option<Uuid>,
    pub to_column_id: Option<Uuid>,

    // Execution context
    pub workspace_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub executor: Option<String>,

    // Automation context
    pub automation_rule_id: Option<Uuid>,
    pub trigger_type: Option<EventTriggerType>,

    // Commit context
    pub commit_hash: Option<String>,
    pub commit_message: Option<String>,

    // Flexible metadata
    #[sqlx(json)]
    pub metadata: Option<JsonValue>,

    // Actor info
    pub actor_type: ActorType,
    pub actor_id: Option<String>,

    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

/// Enriched task event with related entity names for display
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskEventWithNames {
    #[serde(flatten)]
    #[ts(flatten)]
    pub event: TaskEvent,
    pub from_column_name: Option<String>,
    pub to_column_name: Option<String>,
    pub agent_name: Option<String>,
    pub agent_color: Option<String>,
}

/// Create a new task event
#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateTaskEvent {
    pub task_id: Uuid,
    pub event_type: TaskEventType,
    pub from_column_id: Option<Uuid>,
    pub to_column_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub executor: Option<String>,
    pub automation_rule_id: Option<Uuid>,
    pub trigger_type: Option<EventTriggerType>,
    pub commit_hash: Option<String>,
    pub commit_message: Option<String>,
    pub metadata: Option<JsonValue>,
    pub actor_type: Option<ActorType>,
    pub actor_id: Option<String>,
}

impl TaskEvent {
    /// Create a new task event
    pub async fn create(pool: &SqlitePool, data: &CreateTaskEvent) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let actor_type = data.actor_type.clone().unwrap_or_default();
        let metadata_json = data
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_default());

        sqlx::query_as!(
            TaskEvent,
            r#"INSERT INTO task_events (
                id, task_id, event_type,
                from_column_id, to_column_id,
                workspace_id, session_id, executor,
                automation_rule_id, trigger_type,
                commit_hash, commit_message,
                metadata, actor_type, actor_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING
                id as "id!: Uuid",
                task_id as "task_id!: Uuid",
                event_type as "event_type!: TaskEventType",
                from_column_id as "from_column_id: Uuid",
                to_column_id as "to_column_id: Uuid",
                workspace_id as "workspace_id: Uuid",
                session_id as "session_id: Uuid",
                executor,
                automation_rule_id as "automation_rule_id: Uuid",
                trigger_type as "trigger_type: EventTriggerType",
                commit_hash,
                commit_message,
                metadata as "metadata: JsonValue",
                actor_type as "actor_type!: ActorType",
                actor_id,
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            data.task_id,
            data.event_type,
            data.from_column_id,
            data.to_column_id,
            data.workspace_id,
            data.session_id,
            data.executor,
            data.automation_rule_id,
            data.trigger_type,
            data.commit_hash,
            data.commit_message,
            metadata_json,
            actor_type,
            data.actor_id
        )
        .fetch_one(pool)
        .await
    }

    /// Find all events for a task, ordered by creation time (newest first)
    pub async fn find_by_task_id(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskEvent,
            r#"SELECT
                id as "id!: Uuid",
                task_id as "task_id!: Uuid",
                event_type as "event_type!: TaskEventType",
                from_column_id as "from_column_id: Uuid",
                to_column_id as "to_column_id: Uuid",
                workspace_id as "workspace_id: Uuid",
                session_id as "session_id: Uuid",
                executor,
                automation_rule_id as "automation_rule_id: Uuid",
                trigger_type as "trigger_type: EventTriggerType",
                commit_hash,
                commit_message,
                metadata as "metadata: JsonValue",
                actor_type as "actor_type!: ActorType",
                actor_id,
                created_at as "created_at!: DateTime<Utc>"
            FROM task_events
            WHERE task_id = $1
            ORDER BY created_at DESC"#,
            task_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find events for a task with column names resolved
    pub async fn find_by_task_id_with_names(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<TaskEventWithNames>, sqlx::Error> {
        let records = sqlx::query!(
            r#"SELECT
                e.id as "id!: Uuid",
                e.task_id as "task_id!: Uuid",
                e.event_type as "event_type!: TaskEventType",
                e.from_column_id as "from_column_id: Uuid",
                e.to_column_id as "to_column_id: Uuid",
                e.workspace_id as "workspace_id: Uuid",
                e.session_id as "session_id: Uuid",
                e.executor,
                e.automation_rule_id as "automation_rule_id: Uuid",
                e.trigger_type as "trigger_type: EventTriggerType",
                e.commit_hash,
                e.commit_message,
                e.metadata as "metadata: JsonValue",
                e.actor_type as "actor_type!: ActorType",
                e.actor_id,
                e.created_at as "created_at!: DateTime<Utc>",
                fc.name as "from_column_name?",
                tc.name as "to_column_name?",
                a.name as "agent_name?",
                a.color as "agent_color?"
            FROM task_events e
            LEFT JOIN kanban_columns fc ON e.from_column_id = fc.id
            LEFT JOIN kanban_columns tc ON e.to_column_id = tc.id
            LEFT JOIN agents a ON tc.agent_id = a.id
            WHERE e.task_id = $1
            ORDER BY e.created_at DESC"#,
            task_id
        )
        .fetch_all(pool)
        .await?;

        Ok(records
            .into_iter()
            .map(|r| TaskEventWithNames {
                event: TaskEvent {
                    id: r.id,
                    task_id: r.task_id,
                    event_type: r.event_type,
                    from_column_id: r.from_column_id,
                    to_column_id: r.to_column_id,
                    workspace_id: r.workspace_id,
                    session_id: r.session_id,
                    executor: r.executor,
                    automation_rule_id: r.automation_rule_id,
                    trigger_type: r.trigger_type,
                    commit_hash: r.commit_hash,
                    commit_message: r.commit_message,
                    metadata: r.metadata,
                    actor_type: r.actor_type,
                    actor_id: r.actor_id,
                    created_at: r.created_at,
                },
                from_column_name: r.from_column_name,
                to_column_name: r.to_column_name,
                agent_name: r.agent_name,
                agent_color: r.agent_color,
            })
            .collect())
    }

    /// Find events by workspace
    pub async fn find_by_workspace_id(
        pool: &SqlitePool,
        workspace_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskEvent,
            r#"SELECT
                id as "id!: Uuid",
                task_id as "task_id!: Uuid",
                event_type as "event_type!: TaskEventType",
                from_column_id as "from_column_id: Uuid",
                to_column_id as "to_column_id: Uuid",
                workspace_id as "workspace_id: Uuid",
                session_id as "session_id: Uuid",
                executor,
                automation_rule_id as "automation_rule_id: Uuid",
                trigger_type as "trigger_type: EventTriggerType",
                commit_hash,
                commit_message,
                metadata as "metadata: JsonValue",
                actor_type as "actor_type!: ActorType",
                actor_id,
                created_at as "created_at!: DateTime<Utc>"
            FROM task_events
            WHERE workspace_id = $1
            ORDER BY created_at DESC"#,
            workspace_id
        )
        .fetch_all(pool)
        .await
    }

    /// Delete all events for a task
    pub async fn delete_by_task_id(pool: &SqlitePool, task_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM task_events WHERE task_id = $1", task_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Count how many times a task has transitioned FROM a specific column
    /// Used for loop prevention in conditional transitions
    pub async fn count_column_transitions(
        pool: &SqlitePool,
        task_id: Uuid,
        from_column_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64"
               FROM task_events
               WHERE task_id = $1
                 AND event_type = 'column_enter'
                 AND from_column_id = $2"#,
            task_id,
            from_column_id
        )
        .fetch_one(pool)
        .await?;

        Ok(count)
    }

    /// Count how many times a task took the else path FROM a specific column
    /// Used for escalation logic - escalate after N failures
    pub async fn count_else_transitions(
        pool: &SqlitePool,
        task_id: Uuid,
        from_column_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64"
               FROM task_events
               WHERE task_id = $1
                 AND event_type = 'else_transition'
                 AND from_column_id = $2"#,
            task_id,
            from_column_id
        )
        .fetch_one(pool)
        .await?;

        Ok(count)
    }
}

// Helper functions for creating specific event types
impl CreateTaskEvent {
    /// Create a column transition event
    pub fn column_transition(
        task_id: Uuid,
        from_column_id: Option<Uuid>,
        to_column_id: Uuid,
        trigger_type: EventTriggerType,
        actor_type: ActorType,
        actor_id: Option<String>,
    ) -> Self {
        Self {
            task_id,
            event_type: TaskEventType::ColumnEnter,
            from_column_id,
            to_column_id: Some(to_column_id),
            workspace_id: None,
            session_id: None,
            executor: None,
            automation_rule_id: None,
            trigger_type: Some(trigger_type),
            commit_hash: None,
            commit_message: None,
            metadata: None,
            actor_type: Some(actor_type),
            actor_id,
        }
    }

    /// Create an agent start event
    pub fn agent_start(
        task_id: Uuid,
        workspace_id: Uuid,
        session_id: Uuid,
        executor: String,
        trigger_type: EventTriggerType,
        automation_rule_id: Option<Uuid>,
    ) -> Self {
        Self {
            task_id,
            event_type: TaskEventType::AgentStart,
            from_column_id: None,
            to_column_id: None,
            workspace_id: Some(workspace_id),
            session_id: Some(session_id),
            executor: Some(executor),
            automation_rule_id,
            trigger_type: Some(trigger_type),
            commit_hash: None,
            commit_message: None,
            metadata: None,
            actor_type: Some(ActorType::System),
            actor_id: None,
        }
    }

    /// Create an agent completion event
    pub fn agent_complete(task_id: Uuid, workspace_id: Uuid, session_id: Uuid) -> Self {
        Self {
            task_id,
            event_type: TaskEventType::AgentComplete,
            from_column_id: None,
            to_column_id: None,
            workspace_id: Some(workspace_id),
            session_id: Some(session_id),
            executor: None,
            automation_rule_id: None,
            trigger_type: None,
            commit_hash: None,
            commit_message: None,
            metadata: None,
            actor_type: Some(ActorType::Agent),
            actor_id: None,
        }
    }

    /// Create an agent failure event
    pub fn agent_failed(
        task_id: Uuid,
        workspace_id: Uuid,
        session_id: Uuid,
        error_message: Option<String>,
    ) -> Self {
        let metadata = error_message.map(|msg| serde_json::json!({ "error": msg }));
        Self {
            task_id,
            event_type: TaskEventType::AgentFailed,
            from_column_id: None,
            to_column_id: None,
            workspace_id: Some(workspace_id),
            session_id: Some(session_id),
            executor: None,
            automation_rule_id: None,
            trigger_type: None,
            commit_hash: None,
            commit_message: None,
            metadata,
            actor_type: Some(ActorType::Agent),
            actor_id: None,
        }
    }

    /// Create a commit event
    pub fn commit(
        task_id: Uuid,
        workspace_id: Uuid,
        commit_hash: String,
        commit_message: String,
    ) -> Self {
        Self {
            task_id,
            event_type: TaskEventType::Commit,
            from_column_id: None,
            to_column_id: None,
            workspace_id: Some(workspace_id),
            session_id: None,
            executor: None,
            automation_rule_id: None,
            trigger_type: None,
            commit_hash: Some(commit_hash),
            commit_message: Some(commit_message),
            metadata: None,
            actor_type: Some(ActorType::Agent),
            actor_id: None,
        }
    }

    /// Create a task created event
    pub fn task_created(task_id: Uuid, actor_type: ActorType, actor_id: Option<String>) -> Self {
        Self {
            task_id,
            event_type: TaskEventType::TaskCreated,
            from_column_id: None,
            to_column_id: None,
            workspace_id: None,
            session_id: None,
            executor: None,
            automation_rule_id: None,
            trigger_type: Some(EventTriggerType::System),
            commit_hash: None,
            commit_message: None,
            metadata: None,
            actor_type: Some(actor_type),
            actor_id,
        }
    }

    /// Create an else transition event (condition didn't match, took else path)
    /// Used for counting failures toward escalation
    pub fn else_transition(task_id: Uuid, from_column_id: Uuid) -> Self {
        Self {
            task_id,
            event_type: TaskEventType::ElseTransition,
            from_column_id: Some(from_column_id),
            to_column_id: None,
            workspace_id: None,
            session_id: None,
            executor: None,
            automation_rule_id: None,
            trigger_type: Some(EventTriggerType::Automation),
            commit_hash: None,
            commit_message: None,
            metadata: None,
            actor_type: Some(ActorType::System),
            actor_id: None,
        }
    }
}

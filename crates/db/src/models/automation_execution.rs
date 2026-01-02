use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, Sqlite, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Execution status for automation runs
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionStatus::Pending => "pending",
            ExecutionStatus::Running => "running",
            ExecutionStatus::Completed => "completed",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(ExecutionStatus::Pending),
            "running" => Some(ExecutionStatus::Running),
            "completed" => Some(ExecutionStatus::Completed),
            "failed" => Some(ExecutionStatus::Failed),
            "skipped" => Some(ExecutionStatus::Skipped),
            _ => None,
        }
    }
}

/// Trigger context for automation execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TriggerContext {
    pub from_column_id: Option<Uuid>,
    pub from_column_name: Option<String>,
    pub to_column_id: Uuid,
    pub to_column_name: String,
    pub trigger_type: String,
    pub transition_name: Option<String>,
}

/// Log of an automation rule execution
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct AutomationExecution {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub task_id: Uuid,
    pub attempt_id: Option<Uuid>,
    pub status: String,
    pub trigger_context: Option<String>,
    pub result: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

/// Execution with rule and task info for UI display
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AutomationExecutionWithDetails {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub rule_name: Option<String>,
    pub action_type: String,
    pub task_id: Uuid,
    pub task_title: String,
    pub attempt_id: Option<Uuid>,
    pub status: String,
    pub trigger_context: Option<String>,
    pub result: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

impl AutomationExecution {
    /// Parse status string to enum
    pub fn get_status(&self) -> Option<ExecutionStatus> {
        ExecutionStatus::from_str(&self.status)
    }

    /// Parse trigger_context JSON
    pub fn get_trigger_context(&self) -> Option<TriggerContext> {
        self.trigger_context
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }

    /// Parse result JSON
    pub fn get_result<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        self.result
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }

    /// Find all executions for a task
    pub async fn find_by_task(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            AutomationExecution,
            r#"SELECT id as "id!: Uuid",
                      rule_id as "rule_id!: Uuid",
                      task_id as "task_id!: Uuid",
                      attempt_id as "attempt_id: Uuid",
                      status,
                      trigger_context,
                      result,
                      started_at as "started_at: DateTime<Utc>",
                      completed_at as "completed_at: DateTime<Utc>",
                      created_at as "created_at!: DateTime<Utc>"
               FROM automation_executions
               WHERE task_id = $1
               ORDER BY created_at DESC"#,
            task_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all executions with details for UI display
    pub async fn find_by_task_with_details(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<AutomationExecutionWithDetails>, sqlx::Error> {
        sqlx::query_as!(
            AutomationExecutionWithDetails,
            r#"SELECT ae.id as "id!: Uuid",
                      ae.rule_id as "rule_id!: Uuid",
                      ar.name as rule_name,
                      ar.action_type,
                      ae.task_id as "task_id!: Uuid",
                      t.title as "task_title!",
                      ae.attempt_id as "attempt_id: Uuid",
                      ae.status,
                      ae.trigger_context,
                      ae.result,
                      ae.started_at as "started_at: DateTime<Utc>",
                      ae.completed_at as "completed_at: DateTime<Utc>",
                      ae.created_at as "created_at!: DateTime<Utc>"
               FROM automation_executions ae
               JOIN automation_rules ar ON ar.id = ae.rule_id
               JOIN tasks t ON t.id = ae.task_id
               WHERE ae.task_id = $1
               ORDER BY ae.created_at DESC"#,
            task_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find pending or running executions
    pub async fn find_active(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            AutomationExecution,
            r#"SELECT id as "id!: Uuid",
                      rule_id as "rule_id!: Uuid",
                      task_id as "task_id!: Uuid",
                      attempt_id as "attempt_id: Uuid",
                      status,
                      trigger_context,
                      result,
                      started_at as "started_at: DateTime<Utc>",
                      completed_at as "completed_at: DateTime<Utc>",
                      created_at as "created_at!: DateTime<Utc>"
               FROM automation_executions
               WHERE status IN ('pending', 'running')
               ORDER BY created_at ASC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            AutomationExecution,
            r#"SELECT id as "id!: Uuid",
                      rule_id as "rule_id!: Uuid",
                      task_id as "task_id!: Uuid",
                      attempt_id as "attempt_id: Uuid",
                      status,
                      trigger_context,
                      result,
                      started_at as "started_at: DateTime<Utc>",
                      completed_at as "completed_at: DateTime<Utc>",
                      created_at as "created_at!: DateTime<Utc>"
               FROM automation_executions
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new execution (initially pending)
    pub async fn create<'e, E>(
        executor: E,
        rule_id: Uuid,
        task_id: Uuid,
        trigger_context: Option<&TriggerContext>,
    ) -> Result<Self, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let id = Uuid::new_v4();
        let trigger_json = trigger_context.map(|c| serde_json::to_string(c).unwrap());

        sqlx::query_as!(
            AutomationExecution,
            r#"INSERT INTO automation_executions (id, rule_id, task_id, status, trigger_context)
               VALUES ($1, $2, $3, 'pending', $4)
               RETURNING id as "id!: Uuid",
                         rule_id as "rule_id!: Uuid",
                         task_id as "task_id!: Uuid",
                         attempt_id as "attempt_id: Uuid",
                         status,
                         trigger_context,
                         result,
                         started_at as "started_at: DateTime<Utc>",
                         completed_at as "completed_at: DateTime<Utc>",
                         created_at as "created_at!: DateTime<Utc>""#,
            id,
            rule_id,
            task_id,
            trigger_json
        )
        .fetch_one(executor)
        .await
    }

    /// Mark execution as running
    pub async fn start(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE automation_executions
               SET status = 'running', started_at = datetime('now', 'subsec')
               WHERE id = $1"#,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark execution as completed
    pub async fn complete(
        pool: &SqlitePool,
        id: Uuid,
        result: Option<serde_json::Value>,
        attempt_id: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        let result_json = result.map(|r| r.to_string());
        sqlx::query!(
            r#"UPDATE automation_executions
               SET status = 'completed', result = $2, attempt_id = $3, completed_at = datetime('now', 'subsec')
               WHERE id = $1"#,
            id,
            result_json,
            attempt_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark execution as failed
    pub async fn fail(
        pool: &SqlitePool,
        id: Uuid,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        let error_json = serde_json::json!({ "error": error }).to_string();
        sqlx::query!(
            r#"UPDATE automation_executions
               SET status = 'failed', result = $2, completed_at = datetime('now', 'subsec')
               WHERE id = $1"#,
            id,
            error_json
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark execution as skipped
    pub async fn skip(
        pool: &SqlitePool,
        id: Uuid,
        reason: &str,
    ) -> Result<(), sqlx::Error> {
        let reason_json = serde_json::json!({ "skipped": reason }).to_string();
        sqlx::query!(
            r#"UPDATE automation_executions
               SET status = 'skipped', result = $2, completed_at = datetime('now', 'subsec')
               WHERE id = $1"#,
            id,
            reason_json
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}

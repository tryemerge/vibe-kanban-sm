use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use ts_rs::TS;
use uuid::Uuid;

/// Trigger condition for when to auto-start a task
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerCondition {
    /// Any successful completion
    Completed,
    /// Specific status (e.g., "approved")
    CompletedWithStatus(String),
    /// PR merged
    Merged,
}

impl std::fmt::Display for TriggerCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TriggerCondition::Completed => write!(f, "completed"),
            TriggerCondition::CompletedWithStatus(status) => write!(f, "completed_with_status:{}", status),
            TriggerCondition::Merged => write!(f, "merged"),
        }
    }
}

impl std::str::FromStr for TriggerCondition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "completed" {
            Ok(TriggerCondition::Completed)
        } else if s == "merged" {
            Ok(TriggerCondition::Merged)
        } else if let Some(status) = s.strip_prefix("completed_with_status:") {
            Ok(TriggerCondition::CompletedWithStatus(status.to_string()))
        } else {
            Err(format!("Invalid trigger condition: {}", s))
        }
    }
}

/// Task auto-start trigger
/// When trigger_task_id completes, automatically start task_id
/// Uses "ALL" semantics: task only starts when ALL triggers have fired
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskTrigger {
    pub id: Uuid,
    /// The task that will auto-start
    pub task_id: Uuid,
    /// The task to watch for completion
    pub trigger_task_id: Uuid,
    /// Condition for when to trigger (stored as string)
    pub trigger_on: String,
    /// If true, trigger persists after firing; if false, removed after firing
    pub is_persistent: bool,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    /// When this trigger was satisfied (null if not yet fired)
    #[ts(type = "Date | null")]
    pub fired_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateTaskTrigger {
    pub task_id: Uuid,
    pub trigger_task_id: Uuid,
    #[serde(default = "default_trigger_on")]
    pub trigger_on: String,
    #[serde(default)]
    pub is_persistent: bool,
}

fn default_trigger_on() -> String {
    "completed".to_string()
}

impl TaskTrigger {
    /// Find all triggers for a task (what this task is waiting for)
    pub async fn find_by_task(pool: &PgPool, task_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskTrigger,
            r#"SELECT id as "id!: Uuid",
                      task_id as "task_id!: Uuid",
                      trigger_task_id as "trigger_task_id!: Uuid",
                      trigger_on,
                      is_persistent as "is_persistent!: bool",
                      created_at as "created_at!: DateTime<Utc>",
                      fired_at as "fired_at: DateTime<Utc>"
               FROM task_triggers
               WHERE task_id = $1"#,
            task_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all triggers that will fire when a task completes (unfired only)
    pub async fn find_unfired_by_trigger_task(
        pool: &PgPool,
        trigger_task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskTrigger,
            r#"SELECT id as "id!: Uuid",
                      task_id as "task_id!: Uuid",
                      trigger_task_id as "trigger_task_id!: Uuid",
                      trigger_on,
                      is_persistent as "is_persistent!: bool",
                      created_at as "created_at!: DateTime<Utc>",
                      fired_at as "fired_at: DateTime<Utc>"
               FROM task_triggers
               WHERE trigger_task_id = $1 AND fired_at IS NULL"#,
            trigger_task_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all triggers that will fire when a task completes (including already fired)
    pub async fn find_by_trigger_task(
        pool: &PgPool,
        trigger_task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskTrigger,
            r#"SELECT id as "id!: Uuid",
                      task_id as "task_id!: Uuid",
                      trigger_task_id as "trigger_task_id!: Uuid",
                      trigger_on,
                      is_persistent as "is_persistent!: bool",
                      created_at as "created_at!: DateTime<Utc>",
                      fired_at as "fired_at: DateTime<Utc>"
               FROM task_triggers
               WHERE trigger_task_id = $1"#,
            trigger_task_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find a trigger by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskTrigger,
            r#"SELECT id as "id!: Uuid",
                      task_id as "task_id!: Uuid",
                      trigger_task_id as "trigger_task_id!: Uuid",
                      trigger_on,
                      is_persistent as "is_persistent!: bool",
                      created_at as "created_at!: DateTime<Utc>",
                      fired_at as "fired_at: DateTime<Utc>"
               FROM task_triggers
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new trigger
    pub async fn create(pool: &PgPool, data: &CreateTaskTrigger) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            TaskTrigger,
            r#"INSERT INTO task_triggers (id, task_id, trigger_task_id, trigger_on, is_persistent)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id as "id!: Uuid",
                         task_id as "task_id!: Uuid",
                         trigger_task_id as "trigger_task_id!: Uuid",
                         trigger_on,
                         is_persistent as "is_persistent!: bool",
                         created_at as "created_at!: DateTime<Utc>",
                         fired_at as "fired_at: DateTime<Utc>""#,
            id,
            data.task_id,
            data.trigger_task_id,
            data.trigger_on,
            data.is_persistent
        )
        .fetch_one(pool)
        .await
    }

    /// Mark a trigger as fired
    pub async fn mark_fired(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE task_triggers SET fired_at = NOW() WHERE id = $1",
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Check if all triggers for a task have fired
    pub async fn all_triggers_fired(pool: &PgPool, task_id: Uuid) -> Result<bool, sqlx::Error> {
        let unfired_count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64" FROM task_triggers
               WHERE task_id = $1 AND fired_at IS NULL"#,
            task_id
        )
        .fetch_one(pool)
        .await?;
        Ok(unfired_count == 0)
    }

    /// Reset fired status for all triggers of a task (when task is moved back to todo)
    pub async fn reset_fired_status(pool: &PgPool, task_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE task_triggers SET fired_at = NULL WHERE task_id = $1",
            task_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete a trigger
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM task_triggers WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Delete all triggers for a task (when task is deleted)
    pub async fn delete_by_task(pool: &PgPool, task_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM task_triggers WHERE task_id = $1", task_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Get trigger condition as enum
    pub fn condition(&self) -> TriggerCondition {
        self.trigger_on.parse().unwrap_or(TriggerCondition::Completed)
    }
}

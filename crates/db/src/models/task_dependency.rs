use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use ts_rs::TS;
use uuid::Uuid;

/// A hard blocking dependency between two tasks.
/// task_id CANNOT start until depends_on_task_id reaches a done/terminal column.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskDependency {
    pub id: Uuid,
    /// The blocked task
    pub task_id: Uuid,
    /// The prerequisite task that must complete first
    pub depends_on_task_id: Uuid,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    /// When this dependency was satisfied (null if still blocking)
    #[ts(type = "Date | null")]
    pub satisfied_at: Option<DateTime<Utc>>,
    /// Whether this dependency was auto-created by task group ordering
    pub is_auto_group: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateTaskDependency {
    pub task_id: Uuid,
    pub depends_on_task_id: Uuid,
}

impl TaskDependency {
    /// List all dependencies for a task (what this task is waiting for)
    pub async fn find_by_task(pool: &PgPool, task_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskDependency,
            r#"SELECT id as "id!: Uuid",
                      task_id as "task_id!: Uuid",
                      depends_on_task_id as "depends_on_task_id!: Uuid",
                      created_at as "created_at!: DateTime<Utc>",
                      satisfied_at as "satisfied_at: DateTime<Utc>",
                      is_auto_group as "is_auto_group!: bool"
               FROM task_dependencies
               WHERE task_id = $1"#,
            task_id
        )
        .fetch_all(pool)
        .await
    }

    /// List all dependencies that point TO a given prerequisite task
    /// (i.e., who is blocked waiting for this task)
    pub async fn find_by_prerequisite(
        pool: &PgPool,
        depends_on_task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskDependency,
            r#"SELECT id as "id!: Uuid",
                      task_id as "task_id!: Uuid",
                      depends_on_task_id as "depends_on_task_id!: Uuid",
                      created_at as "created_at!: DateTime<Utc>",
                      satisfied_at as "satisfied_at: DateTime<Utc>",
                      is_auto_group as "is_auto_group!: bool"
               FROM task_dependencies
               WHERE depends_on_task_id = $1"#,
            depends_on_task_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find a dependency by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskDependency,
            r#"SELECT id as "id!: Uuid",
                      task_id as "task_id!: Uuid",
                      depends_on_task_id as "depends_on_task_id!: Uuid",
                      created_at as "created_at!: DateTime<Utc>",
                      satisfied_at as "satisfied_at: DateTime<Utc>",
                      is_auto_group as "is_auto_group!: bool"
               FROM task_dependencies
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new dependency
    pub async fn create(pool: &PgPool, data: &CreateTaskDependency) -> Result<Self, sqlx::Error> {
        Self::create_with_auto_group(pool, data, false).await
    }

    /// Create a new dependency, optionally marking it as auto-created by task group ordering
    pub async fn create_with_auto_group(pool: &PgPool, data: &CreateTaskDependency, is_auto_group: bool) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            TaskDependency,
            r#"INSERT INTO task_dependencies (id, task_id, depends_on_task_id, is_auto_group)
               VALUES ($1, $2, $3, $4)
               RETURNING id as "id!: Uuid",
                         task_id as "task_id!: Uuid",
                         depends_on_task_id as "depends_on_task_id!: Uuid",
                         created_at as "created_at!: DateTime<Utc>",
                         satisfied_at as "satisfied_at: DateTime<Utc>",
                         is_auto_group as "is_auto_group!: bool""#,
            id,
            data.task_id,
            data.depends_on_task_id,
            is_auto_group,
        )
        .fetch_one(pool)
        .await
    }

    /// Delete all auto-group dependencies for a task (both directions)
    pub async fn delete_auto_group_by_task(pool: &PgPool, task_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM task_dependencies WHERE (task_id = $1 OR depends_on_task_id = $1) AND is_auto_group = TRUE",
            task_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Check if a task has any unsatisfied dependencies (the blocking guard)
    pub async fn has_unsatisfied(pool: &PgPool, task_id: Uuid) -> Result<bool, sqlx::Error> {
        let count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64"
               FROM task_dependencies
               WHERE task_id = $1 AND satisfied_at IS NULL"#,
            task_id
        )
        .fetch_one(pool)
        .await?;
        Ok(count > 0)
    }

    /// Satisfy all dependencies pointing to a given prerequisite task.
    /// Called when depends_on_task_id reaches a done terminal column.
    pub async fn satisfy_by_prerequisite(
        pool: &PgPool,
        depends_on_task_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "UPDATE task_dependencies SET satisfied_at = NOW() WHERE depends_on_task_id = $1 AND satisfied_at IS NULL",
            depends_on_task_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Reset satisfaction for all dependencies pointing to a given prerequisite task.
    /// Called when depends_on_task_id moves BACK from a done status.
    pub async fn unsatisfy_by_prerequisite(
        pool: &PgPool,
        depends_on_task_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "UPDATE task_dependencies SET satisfied_at = NULL WHERE depends_on_task_id = $1",
            depends_on_task_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Delete a dependency by ID
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM task_dependencies WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Delete all dependencies for a task
    pub async fn delete_by_task(pool: &PgPool, task_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM task_dependencies WHERE task_id = $1", task_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

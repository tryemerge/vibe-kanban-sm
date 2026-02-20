use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use ts_rs::TS;
use uuid::Uuid;

/// A hard blocking dependency between two task groups.
/// task_group_id CANNOT proceed until depends_on_group_id reaches completion.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskGroupDependency {
    pub id: Uuid,
    /// The blocked group
    pub task_group_id: Uuid,
    /// The prerequisite group that must complete first
    pub depends_on_group_id: Uuid,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    /// When this dependency was satisfied (null if still blocking)
    #[ts(type = "Date | null")]
    pub satisfied_at: Option<DateTime<Utc>>,
}

impl TaskGroupDependency {
    /// List all dependencies for a group (what this group is waiting for)
    pub async fn find_by_group(
        pool: &PgPool,
        task_group_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskGroupDependency,
            r#"SELECT id as "id!: Uuid",
                      task_group_id as "task_group_id!: Uuid",
                      depends_on_group_id as "depends_on_group_id!: Uuid",
                      created_at as "created_at!: DateTime<Utc>",
                      satisfied_at as "satisfied_at: DateTime<Utc>"
               FROM task_group_dependencies
               WHERE task_group_id = $1"#,
            task_group_id
        )
        .fetch_all(pool)
        .await
    }

    /// List all dependencies that point TO a given prerequisite group
    /// (i.e., who is blocked waiting for this group)
    pub async fn find_by_prerequisite(
        pool: &PgPool,
        depends_on_group_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskGroupDependency,
            r#"SELECT id as "id!: Uuid",
                      task_group_id as "task_group_id!: Uuid",
                      depends_on_group_id as "depends_on_group_id!: Uuid",
                      created_at as "created_at!: DateTime<Utc>",
                      satisfied_at as "satisfied_at: DateTime<Utc>"
               FROM task_group_dependencies
               WHERE depends_on_group_id = $1"#,
            depends_on_group_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find a dependency by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskGroupDependency,
            r#"SELECT id as "id!: Uuid",
                      task_group_id as "task_group_id!: Uuid",
                      depends_on_group_id as "depends_on_group_id!: Uuid",
                      created_at as "created_at!: DateTime<Utc>",
                      satisfied_at as "satisfied_at: DateTime<Utc>"
               FROM task_group_dependencies
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new dependency
    pub async fn create(
        pool: &PgPool,
        task_group_id: Uuid,
        depends_on_group_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            TaskGroupDependency,
            r#"INSERT INTO task_group_dependencies (id, task_group_id, depends_on_group_id)
               VALUES ($1, $2, $3)
               RETURNING id as "id!: Uuid",
                         task_group_id as "task_group_id!: Uuid",
                         depends_on_group_id as "depends_on_group_id!: Uuid",
                         created_at as "created_at!: DateTime<Utc>",
                         satisfied_at as "satisfied_at: DateTime<Utc>""#,
            id,
            task_group_id,
            depends_on_group_id,
        )
        .fetch_one(pool)
        .await
    }

    /// Check if a group has any unsatisfied dependencies (the blocking guard)
    pub async fn has_unsatisfied(
        pool: &PgPool,
        task_group_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64"
               FROM task_group_dependencies
               WHERE task_group_id = $1 AND satisfied_at IS NULL"#,
            task_group_id
        )
        .fetch_one(pool)
        .await?;
        Ok(count > 0)
    }

    /// Satisfy all dependencies pointing to a given prerequisite group.
    /// Called when depends_on_group_id reaches completion.
    pub async fn satisfy_by_prerequisite(
        pool: &PgPool,
        depends_on_group_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "UPDATE task_group_dependencies SET satisfied_at = NOW() WHERE depends_on_group_id = $1 AND satisfied_at IS NULL",
            depends_on_group_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Reset satisfaction for all dependencies pointing to a given prerequisite group.
    /// Called when depends_on_group_id moves back from a completed status.
    pub async fn unsatisfy_by_prerequisite(
        pool: &PgPool,
        depends_on_group_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "UPDATE task_group_dependencies SET satisfied_at = NULL WHERE depends_on_group_id = $1",
            depends_on_group_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Delete a dependency by ID
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM task_group_dependencies WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

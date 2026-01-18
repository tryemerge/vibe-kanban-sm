use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use ts_rs::TS;
use uuid::Uuid;

/// Agent file lock
/// Allows agents to claim exclusive access to files during execution
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct FileLock {
    pub id: Uuid,
    /// Path to the locked file (relative to project root)
    pub file_path: String,
    /// Project the lock belongs to
    pub project_id: Uuid,
    /// Task that acquired the lock
    pub task_id: Uuid,
    /// Workspace session that holds the lock
    pub workspace_id: Uuid,
    #[ts(type = "Date")]
    pub acquired_at: DateTime<Utc>,
    /// Optional expiration time (lock auto-releases after this)
    #[ts(type = "Date | null")]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateFileLock {
    pub file_path: String,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub workspace_id: Uuid,
    /// Optional TTL in seconds
    pub ttl_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct FileLockStatus {
    pub is_locked: bool,
    pub lock: Option<FileLock>,
}

impl FileLock {
    /// Check if a file is locked in a project
    pub async fn is_locked(
        pool: &PgPool,
        project_id: Uuid,
        file_path: &str,
    ) -> Result<FileLockStatus, sqlx::Error> {
        // Check for active lock (not expired)
        let lock = sqlx::query_as!(
            FileLock,
            r#"SELECT id as "id!: Uuid",
                      file_path,
                      project_id as "project_id!: Uuid",
                      task_id as "task_id!: Uuid",
                      workspace_id as "workspace_id!: Uuid",
                      acquired_at as "acquired_at!: DateTime<Utc>",
                      expires_at as "expires_at: DateTime<Utc>"
               FROM file_locks
               WHERE project_id = $1 AND file_path = $2
               AND (expires_at IS NULL OR expires_at > NOW())"#,
            project_id,
            file_path
        )
        .fetch_optional(pool)
        .await?;

        Ok(FileLockStatus {
            is_locked: lock.is_some(),
            lock,
        })
    }

    /// Find all locks for a project
    pub async fn find_by_project(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            FileLock,
            r#"SELECT id as "id!: Uuid",
                      file_path,
                      project_id as "project_id!: Uuid",
                      task_id as "task_id!: Uuid",
                      workspace_id as "workspace_id!: Uuid",
                      acquired_at as "acquired_at!: DateTime<Utc>",
                      expires_at as "expires_at: DateTime<Utc>"
               FROM file_locks
               WHERE project_id = $1
               AND (expires_at IS NULL OR expires_at > NOW())"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all locks held by a workspace
    pub async fn find_by_workspace(
        pool: &PgPool,
        workspace_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            FileLock,
            r#"SELECT id as "id!: Uuid",
                      file_path,
                      project_id as "project_id!: Uuid",
                      task_id as "task_id!: Uuid",
                      workspace_id as "workspace_id!: Uuid",
                      acquired_at as "acquired_at!: DateTime<Utc>",
                      expires_at as "expires_at: DateTime<Utc>"
               FROM file_locks
               WHERE workspace_id = $1"#,
            workspace_id
        )
        .fetch_all(pool)
        .await
    }

    /// Acquire a lock on a file
    /// Returns None if the file is already locked by another workspace
    pub async fn acquire(
        pool: &PgPool,
        data: &CreateFileLock,
    ) -> Result<Option<Self>, sqlx::Error> {
        // First check if file is already locked
        let status = Self::is_locked(pool, data.project_id, &data.file_path).await?;
        if status.is_locked {
            // Check if it's our lock (same workspace can refresh)
            if let Some(existing) = &status.lock {
                if existing.workspace_id == data.workspace_id {
                    // Refresh the lock
                    let expires_at = data
                        .ttl_seconds
                        .map(|ttl| Utc::now() + chrono::Duration::seconds(ttl));
                    sqlx::query!(
                        "UPDATE file_locks SET expires_at = $1, acquired_at = NOW() WHERE id = $2",
                        expires_at,
                        existing.id
                    )
                    .execute(pool)
                    .await?;
                    return Self::find_by_id(pool, existing.id).await;
                }
            }
            return Ok(None);
        }

        // Acquire the lock
        let id = Uuid::new_v4();
        let expires_at = data
            .ttl_seconds
            .map(|ttl| Utc::now() + chrono::Duration::seconds(ttl));

        let lock = sqlx::query_as!(
            FileLock,
            r#"INSERT INTO file_locks (id, file_path, project_id, task_id, workspace_id, expires_at)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (file_path, project_id) DO NOTHING
               RETURNING id as "id!: Uuid",
                         file_path,
                         project_id as "project_id!: Uuid",
                         task_id as "task_id!: Uuid",
                         workspace_id as "workspace_id!: Uuid",
                         acquired_at as "acquired_at!: DateTime<Utc>",
                         expires_at as "expires_at: DateTime<Utc>""#,
            id,
            data.file_path,
            data.project_id,
            data.task_id,
            data.workspace_id,
            expires_at
        )
        .fetch_optional(pool)
        .await?;

        Ok(lock)
    }

    /// Release a lock by ID
    pub async fn release(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM file_locks WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Release a lock on a file for a specific workspace
    pub async fn release_by_path(
        pool: &PgPool,
        project_id: Uuid,
        file_path: &str,
        workspace_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM file_locks WHERE project_id = $1 AND file_path = $2 AND workspace_id = $3",
            project_id,
            file_path,
            workspace_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Release all locks held by a workspace (called when workspace completes)
    pub async fn release_all_by_workspace(
        pool: &PgPool,
        workspace_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM file_locks WHERE workspace_id = $1", workspace_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Find by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            FileLock,
            r#"SELECT id as "id!: Uuid",
                      file_path,
                      project_id as "project_id!: Uuid",
                      task_id as "task_id!: Uuid",
                      workspace_id as "workspace_id!: Uuid",
                      acquired_at as "acquired_at!: DateTime<Utc>",
                      expires_at as "expires_at: DateTime<Utc>"
               FROM file_locks
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Clean up expired locks
    pub async fn cleanup_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM file_locks WHERE expires_at IS NOT NULL AND expires_at < NOW()")
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

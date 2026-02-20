use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

use super::{
    kanban_column::KanbanColumn,
    project::Project,
    task::Task,
    workspace_repo::{RepoWithTargetBranch, WorkspaceRepo},
};

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Task not found")]
    TaskNotFound,
    #[error("Project not found")]
    ProjectNotFound,
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Branch not found: {0}")]
    BranchNotFound(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerInfo {
    pub workspace_id: Uuid,
    pub task_id: Uuid,
    pub project_id: Uuid,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "workspace_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceStatus {
    SetupRunning,
    SetupComplete,
    SetupFailed,
    ExecutorRunning,
    ExecutorComplete,
    ExecutorFailed,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Workspace {
    pub id: Uuid,
    pub task_id: Uuid,
    pub container_ref: Option<String>,
    pub branch: String,
    pub agent_working_dir: Option<String>,
    pub setup_completed_at: Option<DateTime<Utc>>,
    /// When set, this attempt has been cancelled (but history is preserved)
    #[ts(type = "Date | null")]
    pub cancelled_at: Option<DateTime<Utc>>,
    /// Final context captured before worktree deletion - what the agent learned/decided
    pub final_context: Option<String>,
    /// Brief summary of what was accomplished in this attempt
    pub completion_summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// The TaskGroup that owns this workspace (ADR-015: group-level worktrees)
    pub task_group_id: Option<Uuid>,
}

/// GitHub PR creation parameters
pub struct CreatePrParams<'a> {
    pub workspace_id: Uuid,
    pub task_id: Uuid,
    pub project_id: Uuid,
    pub github_token: &'a str,
    pub title: &'a str,
    pub body: Option<&'a str>,
    pub base_branch: Option<&'a str>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateFollowUpAttempt {
    pub prompt: String,
}

/// Context data for resume operations (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptResumeContext {
    pub execution_history: String,
    pub cumulative_diffs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceContext {
    pub workspace: Workspace,
    pub task: Task,
    pub project: Project,
    pub workspace_repos: Vec<RepoWithTargetBranch>,
    /// The current kanban column for the task (if assigned to one)
    pub column: Option<KanbanColumn>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateWorkspace {
    pub branch: String,
    pub agent_working_dir: Option<String>,
}

impl Workspace {
    pub async fn parent_task(&self, pool: &PgPool) -> Result<Option<Task>, sqlx::Error> {
        Task::find_by_id(pool, self.task_id).await
    }

    /// Fetch all workspaces, optionally filtered by task_id. Newest first.
    pub async fn fetch_all(
        pool: &PgPool,
        task_id: Option<Uuid>,
    ) -> Result<Vec<Self>, WorkspaceError> {
        let workspaces = match task_id {
            Some(tid) => sqlx::query_as!(
                Workspace,
                r#"SELECT id AS "id!: Uuid",
                              task_id AS "task_id!: Uuid",
                              container_ref,
                              branch,
                              agent_working_dir,
                              setup_completed_at AS "setup_completed_at: DateTime<Utc>",
                              cancelled_at AS "cancelled_at: DateTime<Utc>",
                              final_context,
                              completion_summary,
                              created_at AS "created_at!: DateTime<Utc>",
                              updated_at AS "updated_at!: DateTime<Utc>",
                              task_group_id AS "task_group_id: Uuid"
                       FROM workspaces
                       WHERE task_id = $1
                       ORDER BY created_at DESC"#,
                tid
            )
            .fetch_all(pool)
            .await
            .map_err(WorkspaceError::Database)?,
            None => sqlx::query_as!(
                Workspace,
                r#"SELECT id AS "id!: Uuid",
                              task_id AS "task_id!: Uuid",
                              container_ref,
                              branch,
                              agent_working_dir,
                              setup_completed_at AS "setup_completed_at: DateTime<Utc>",
                              cancelled_at AS "cancelled_at: DateTime<Utc>",
                              final_context,
                              completion_summary,
                              created_at AS "created_at!: DateTime<Utc>",
                              updated_at AS "updated_at!: DateTime<Utc>",
                              task_group_id AS "task_group_id: Uuid"
                       FROM workspaces
                       ORDER BY created_at DESC"#
            )
            .fetch_all(pool)
            .await
            .map_err(WorkspaceError::Database)?,
        };

        Ok(workspaces)
    }

    /// Load workspace with full validation - ensures workspace belongs to task and task belongs to project
    pub async fn load_context(
        pool: &PgPool,
        workspace_id: Uuid,
        task_id: Uuid,
        project_id: Uuid,
    ) -> Result<WorkspaceContext, WorkspaceError> {
        let workspace = sqlx::query_as!(
            Workspace,
            r#"SELECT  w.id                AS "id!: Uuid",
                       w.task_id           AS "task_id!: Uuid",
                       w.container_ref,
                       w.branch,
                       w.agent_working_dir,
                       w.setup_completed_at AS "setup_completed_at: DateTime<Utc>",
                       w.cancelled_at      AS "cancelled_at: DateTime<Utc>",
                       w.final_context,
                       w.completion_summary,
                       w.created_at        AS "created_at!: DateTime<Utc>",
                       w.updated_at        AS "updated_at!: DateTime<Utc>",
                       w.task_group_id     AS "task_group_id: Uuid"
               FROM    workspaces w
               JOIN    tasks t ON w.task_id = t.id
               JOIN    projects p ON t.project_id = p.id
               WHERE   w.id = $1 AND t.id = $2 AND p.id = $3"#,
            workspace_id,
            task_id,
            project_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or(WorkspaceError::TaskNotFound)?;

        // Load task and project (we know they exist due to JOIN validation)
        let task = Task::find_by_id(pool, task_id)
            .await?
            .ok_or(WorkspaceError::TaskNotFound)?;

        let project = Project::find_by_id(pool, project_id)
            .await?
            .ok_or(WorkspaceError::ProjectNotFound)?;

        let workspace_repos =
            WorkspaceRepo::find_repos_with_target_branch_for_workspace(pool, workspace_id).await?;

        // Fetch the column info if the task has a column_id
        let column = if let Some(column_id) = task.column_id {
            KanbanColumn::find_by_id(pool, column_id).await?
        } else {
            None
        };

        Ok(WorkspaceContext {
            workspace,
            task,
            project,
            workspace_repos,
            column,
        })
    }

    /// Update container reference
    pub async fn update_container_ref(
        pool: &PgPool,
        workspace_id: Uuid,
        container_ref: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query!(
            "UPDATE workspaces SET container_ref = $1, updated_at = $2 WHERE id = $3",
            container_ref,
            now,
            workspace_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn clear_container_ref(
        pool: &PgPool,
        workspace_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE workspaces SET container_ref = NULL, updated_at = NOW() WHERE id = $1",
            workspace_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Workspace,
            r#"SELECT  id                AS "id!: Uuid",
                       task_id           AS "task_id!: Uuid",
                       container_ref,
                       branch,
                       agent_working_dir,
                       setup_completed_at AS "setup_completed_at: DateTime<Utc>",
                       cancelled_at      AS "cancelled_at: DateTime<Utc>",
                       final_context,
                       completion_summary,
                       created_at        AS "created_at!: DateTime<Utc>",
                       updated_at        AS "updated_at!: DateTime<Utc>",
                       task_group_id     AS "task_group_id: Uuid"
               FROM    workspaces
               WHERE   id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find the most recent active (non-cancelled) workspace for a task
    pub async fn find_active_for_task(
        pool: &PgPool,
        task_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Workspace,
            r#"SELECT  id                AS "id!: Uuid",
                       task_id           AS "task_id!: Uuid",
                       container_ref,
                       branch,
                       agent_working_dir,
                       setup_completed_at AS "setup_completed_at: DateTime<Utc>",
                       cancelled_at      AS "cancelled_at: DateTime<Utc>",
                       final_context,
                       completion_summary,
                       created_at        AS "created_at!: DateTime<Utc>",
                       updated_at        AS "updated_at!: DateTime<Utc>",
                       task_group_id     AS "task_group_id: Uuid"
               FROM    workspaces
               WHERE   task_id = $1
                 AND   cancelled_at IS NULL
               ORDER BY created_at DESC
               LIMIT 1"#,
            task_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find workspace by row number (for Electric sync compatibility)
    /// Note: PostgreSQL doesn't have rowid, so we use a subquery with row_number
    pub async fn find_by_rowid(pool: &PgPool, rowid: i64) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Workspace,
            r#"SELECT  id                AS "id!: Uuid",
                       task_id           AS "task_id!: Uuid",
                       container_ref,
                       branch,
                       agent_working_dir,
                       setup_completed_at AS "setup_completed_at: DateTime<Utc>",
                       cancelled_at      AS "cancelled_at: DateTime<Utc>",
                       final_context,
                       completion_summary,
                       created_at        AS "created_at!: DateTime<Utc>",
                       updated_at        AS "updated_at!: DateTime<Utc>",
                       task_group_id     AS "task_group_id: Uuid"
               FROM (
                   SELECT *, ROW_NUMBER() OVER (ORDER BY created_at) as rn
                   FROM workspaces
               ) sub
               WHERE rn = $1"#,
            rowid
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn container_ref_exists(
        pool: &PgPool,
        container_ref: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"SELECT EXISTS(SELECT 1 FROM workspaces WHERE container_ref = $1) as "exists!: bool""#,
            container_ref
        )
        .fetch_one(pool)
        .await?;

        Ok(result.exists)
    }

    /// Find workspaces that are expired (72+ hours since last activity) and eligible for cleanup
    pub async fn find_expired_for_cleanup(
        pool: &PgPool,
    ) -> Result<Vec<Workspace>, sqlx::Error> {
        sqlx::query_as!(
            Workspace,
            r#"
            SELECT
                w.id as "id!: Uuid",
                w.task_id as "task_id!: Uuid",
                w.container_ref,
                w.branch as "branch!",
                w.agent_working_dir,
                w.setup_completed_at as "setup_completed_at: DateTime<Utc>",
                w.cancelled_at as "cancelled_at: DateTime<Utc>",
                w.final_context,
                w.completion_summary,
                w.created_at as "created_at!: DateTime<Utc>",
                w.updated_at as "updated_at!: DateTime<Utc>",
                w.task_group_id as "task_group_id: Uuid"
            FROM workspaces w
            LEFT JOIN sessions s ON w.id = s.workspace_id
            LEFT JOIN execution_processes ep ON s.id = ep.session_id AND ep.completed_at IS NOT NULL
            WHERE w.container_ref IS NOT NULL
                AND w.id NOT IN (
                    SELECT DISTINCT s2.workspace_id
                    FROM sessions s2
                    JOIN execution_processes ep2 ON s2.id = ep2.session_id
                    WHERE ep2.completed_at IS NULL
                )
            GROUP BY w.id, w.task_id, w.container_ref, w.branch, w.agent_working_dir,
                     w.setup_completed_at, w.cancelled_at, w.final_context, w.completion_summary,
                     w.created_at, w.updated_at, w.task_group_id
            HAVING NOW() - INTERVAL '72 hours' > MAX(COALESCE(ep.completed_at, w.updated_at))
            ORDER BY MAX(COALESCE(ep.completed_at, w.updated_at)) ASC
            "#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &PgPool,
        data: &CreateWorkspace,
        id: Uuid,
        task_id: Uuid,
    ) -> Result<Self, WorkspaceError> {
        Ok(sqlx::query_as!(
            Workspace,
            r#"INSERT INTO workspaces (id, task_id, container_ref, branch, agent_working_dir, setup_completed_at)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id as "id!: Uuid", task_id as "task_id!: Uuid", container_ref, branch, agent_working_dir, setup_completed_at as "setup_completed_at: DateTime<Utc>", cancelled_at as "cancelled_at: DateTime<Utc>", final_context, completion_summary, created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>", task_group_id as "task_group_id: Uuid""#,
            id,
            task_id,
            Option::<String>::None,
            data.branch,
            data.agent_working_dir,
            Option::<DateTime<Utc>>::None
        )
        .fetch_one(pool)
        .await?)
    }

    pub async fn update_branch_name(
        pool: &PgPool,
        workspace_id: Uuid,
        new_branch_name: &str,
    ) -> Result<(), WorkspaceError> {
        sqlx::query!(
            "UPDATE workspaces SET branch = $1, updated_at = NOW() WHERE id = $2",
            new_branch_name,
            workspace_id,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete a workspace by ID
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM workspaces WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Mark a workspace as cancelled (preserves history but marks as no longer active)
    pub async fn set_cancelled(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query!(
            "UPDATE workspaces SET cancelled_at = $1, container_ref = NULL, updated_at = $2 WHERE id = $3",
            now,
            now,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn resolve_container_ref(
        pool: &PgPool,
        container_ref: &str,
    ) -> Result<ContainerInfo, sqlx::Error> {
        #[derive(Debug)]
        struct QueryResult {
            workspace_id: Uuid,
            task_id: Uuid,
            project_id: Uuid,
        }
        let result: QueryResult = sqlx::query_as!(
            QueryResult,
            r#"SELECT w.id as "workspace_id!: Uuid",
                      w.task_id as "task_id!: Uuid",
                      t.project_id as "project_id!: Uuid"
               FROM workspaces w
               JOIN tasks t ON w.task_id = t.id
               WHERE w.container_ref = $1"#,
            container_ref
        )
        .fetch_optional(pool)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;

        Ok(ContainerInfo {
            workspace_id: result.workspace_id,
            task_id: result.task_id,
            project_id: result.project_id,
        })
    }

    /// Save final context and completion summary before worktree deletion
    pub async fn save_final_context(
        pool: &PgPool,
        workspace_id: Uuid,
        final_context: Option<&str>,
        completion_summary: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE workspaces SET final_context = $1, completion_summary = $2, updated_at = NOW() WHERE id = $3",
            final_context,
            completion_summary,
            workspace_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}

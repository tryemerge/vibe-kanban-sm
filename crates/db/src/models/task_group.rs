use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use ts_rs::TS;
use uuid::Uuid;

/// A named group of tasks within a project (e.g., sprint, epic, milestone).
/// Groups are ordered by position and can optionally track when work started.
/// As of ADR-015, groups own the workspace/worktree that all tasks in the group share.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TaskGroup {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub position: i32,
    #[ts(type = "Date | null")]
    pub started_at: Option<DateTime<Utc>>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "string")]
    pub state: String,
    pub is_backlog: bool,
    pub execution_dag: Option<String>,
    /// The workspace/worktree for this group (all tasks share it)
    pub workspace_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateTaskGroup {
    pub project_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub is_backlog: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct UpdateTaskGroup {
    pub name: Option<String>,
    pub color: Option<String>,
}

impl TaskGroup {
    /// List all groups for a project, ordered by position
    pub async fn find_by_project(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskGroup,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      name,
                      color as "color: String",
                      position as "position!: i32",
                      started_at as "started_at: DateTime<Utc>",
                      created_at as "created_at!: DateTime<Utc>",
                      state,
                      is_backlog as "is_backlog!: bool",
                      execution_dag as "execution_dag: String",
                      workspace_id as "workspace_id: Uuid"
               FROM task_groups
               WHERE project_id = $1
               ORDER BY position"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find a group by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskGroup,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      name,
                      color as "color: String",
                      position as "position!: i32",
                      started_at as "started_at: DateTime<Utc>",
                      created_at as "created_at!: DateTime<Utc>",
                      state,
                      is_backlog as "is_backlog!: bool",
                      execution_dag as "execution_dag: String",
                      workspace_id as "workspace_id: Uuid"
               FROM task_groups
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new group with position auto-assigned as max+1
    pub async fn create(pool: &PgPool, data: &CreateTaskGroup) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let is_backlog = data.is_backlog.unwrap_or(false);
        sqlx::query_as!(
            TaskGroup,
            r#"INSERT INTO task_groups (id, project_id, name, color, position, is_backlog)
               VALUES ($1, $2, $3, $4,
                       COALESCE((SELECT MAX(position) FROM task_groups WHERE project_id = $2), -1) + 1, $5)
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         name,
                         color as "color: String",
                         position as "position!: i32",
                         started_at as "started_at: DateTime<Utc>",
                         created_at as "created_at!: DateTime<Utc>",
                         state,
                         is_backlog as "is_backlog!: bool",
                         execution_dag as "execution_dag: String",
                         workspace_id as "workspace_id: Uuid""#,
            id,
            data.project_id,
            data.name,
            data.color,
            is_backlog,
        )
        .fetch_one(pool)
        .await
    }

    /// Update a group's name and/or color. Returns None if not found.
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        data: &UpdateTaskGroup,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskGroup,
            r#"UPDATE task_groups
               SET name = COALESCE($2, name),
                   color = COALESCE($3, color)
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         name,
                         color as "color: String",
                         position as "position!: i32",
                         started_at as "started_at: DateTime<Utc>",
                         created_at as "created_at!: DateTime<Utc>",
                         state,
                         is_backlog as "is_backlog!: bool",
                         execution_dag as "execution_dag: String",
                         workspace_id as "workspace_id: Uuid""#,
            id,
            data.name,
            data.color,
        )
        .fetch_optional(pool)
        .await
    }

    /// Delete a group by ID. Returns the number of rows affected.
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM task_groups WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Reorder groups within a project. Each group_id is assigned position
    /// equal to its index in the provided list.
    pub async fn reorder(
        pool: &PgPool,
        project_id: Uuid,
        group_ids: Vec<Uuid>,
    ) -> Result<(), sqlx::Error> {
        for (index, group_id) in group_ids.iter().enumerate() {
            sqlx::query!(
                "UPDATE task_groups SET position = $1 WHERE id = $2 AND project_id = $3",
                index as i32,
                group_id,
                project_id
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    /// Mark a group as started (sets started_at to NOW and transitions to 'executing' if in 'ready' state)
    pub async fn mark_started(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE task_groups
             SET started_at = NOW(),
                 state = CASE WHEN state = 'ready' THEN 'executing' ELSE state END
             WHERE id = $1 AND started_at IS NULL",
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Check whether a group has been started (by checking if state is not 'draft')
    pub async fn is_started(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let state: Option<String> = sqlx::query_scalar!(
            "SELECT state FROM task_groups WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?;
        Ok(state.map(|s| s != "draft").unwrap_or(false))
    }

    /// Transition a group's state. Validates the transition is legal.
    /// Valid transitions: draft→analyzing, analyzing→ready/failed, ready→executing,
    /// executing→done/failed, failed→draft
    pub async fn transition_state(
        pool: &PgPool,
        id: Uuid,
        from: &str,
        to: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        // Validate transition
        let is_valid = matches!((from, to),
            ("draft", "analyzing") |
            ("analyzing", "ready") |
            ("analyzing", "failed") |
            ("ready", "executing") |
            ("executing", "done") |
            ("executing", "failed") |
            ("failed", "draft")
        );

        if !is_valid {
            return Err(sqlx::Error::Protocol(format!("Invalid state transition: {} -> {}", from, to).into()));
        }

        sqlx::query_as!(
            TaskGroup,
            r#"UPDATE task_groups
               SET state = $2
               WHERE id = $1 AND state = $3
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         name,
                         color as "color: String",
                         position as "position!: i32",
                         started_at as "started_at: DateTime<Utc>",
                         created_at as "created_at!: DateTime<Utc>",
                         state,
                         is_backlog as "is_backlog!: bool",
                         execution_dag as "execution_dag: String",
                         workspace_id as "workspace_id: Uuid""#,
            id, to, from
        )
        .fetch_optional(pool)
        .await
    }

    /// Find backlog groups in 'draft' state where all inter-group dependencies are satisfied
    pub async fn find_promotable_backlogs(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskGroup,
            r#"SELECT tg.id as "id!: Uuid",
                      tg.project_id as "project_id!: Uuid",
                      tg.name,
                      tg.color as "color: String",
                      tg.position as "position!: i32",
                      tg.started_at as "started_at: DateTime<Utc>",
                      tg.created_at as "created_at!: DateTime<Utc>",
                      tg.state,
                      tg.is_backlog as "is_backlog!: bool",
                      tg.execution_dag as "execution_dag: String",
                      tg.workspace_id as "workspace_id: Uuid"
               FROM task_groups tg
               WHERE tg.project_id = $1
                 AND tg.is_backlog = TRUE
                 AND tg.state = 'draft'
                 AND NOT EXISTS (
                   SELECT 1 FROM task_group_dependencies tgd
                   WHERE tgd.task_group_id = tg.id
                     AND tgd.satisfied_at IS NULL
                 )"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Update a group's execution DAG
    pub async fn update_execution_dag(
        pool: &PgPool,
        id: Uuid,
        dag: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskGroup,
            r#"UPDATE task_groups
               SET execution_dag = $2
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         name,
                         color as "color: String",
                         position as "position!: i32",
                         started_at as "started_at: DateTime<Utc>",
                         created_at as "created_at!: DateTime<Utc>",
                         state,
                         is_backlog as "is_backlog!: bool",
                         execution_dag as "execution_dag: String",
                         workspace_id as "workspace_id: Uuid""#,
            id, dag
        )
        .fetch_optional(pool)
        .await
    }

    /// Get all tasks in this group
    pub async fn get_tasks(pool: &PgPool, group_id: Uuid) -> Result<Vec<super::task::Task>, sqlx::Error> {
        use super::task::{Task, TaskState, TaskStatus};
        use chrono::{DateTime, Utc};
        use serde_json::Value as JsonValue;

        sqlx::query_as!(
            Task,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                title,
                description,
                status as "status!: TaskStatus",
                column_id as "column_id: Uuid",
                parent_workspace_id as "parent_workspace_id: Uuid",
                shared_task_id as "shared_task_id: Uuid",
                task_group_id as "task_group_id: Uuid",
                task_state as "task_state!: TaskState",
                workflow_decisions as "workflow_decisions: JsonValue",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM tasks
               WHERE task_group_id = $1
               ORDER BY created_at ASC"#,
            group_id
        )
        .fetch_all(pool)
        .await
    }

    /// Create a workspace for this group when it enters "executing" state.
    /// The workspace is shared by all tasks in the group.
    ///
    /// This method:
    /// 1. Finds the first task in the group (by created_at)
    /// 2. Creates a workspace associated with that task
    /// 3. Links the workspace to the group via task_group_id
    /// 4. Updates the group's workspace_id field
    pub async fn create_workspace(
        pool: &PgPool,
        group_id: Uuid,
        branch_name: &str,
    ) -> Result<super::workspace::Workspace, sqlx::Error> {
        use super::workspace::{CreateWorkspace, Workspace};

        // Get all tasks in the group
        let tasks = Self::get_tasks(pool, group_id).await?;

        if tasks.is_empty() {
            return Err(sqlx::Error::Protocol(
                format!("Cannot create workspace for group {}: no tasks in group", group_id).into()
            ));
        }

        // Use the first task as the primary task for the workspace
        let primary_task_id = tasks[0].id;

        // Create workspace with the group's branch
        let workspace_id = Uuid::new_v4();
        let create_data = CreateWorkspace {
            branch: branch_name.to_string(),
            agent_working_dir: None,
        };

        let mut workspace = Workspace::create(pool, &create_data, workspace_id, primary_task_id).await
            .map_err(|e| match e {
                super::workspace::WorkspaceError::Database(db_err) => db_err,
                other => sqlx::Error::Protocol(format!("Workspace creation error: {}", other).into()),
            })?;

        // Link workspace to group by updating task_group_id
        sqlx::query!(
            "UPDATE workspaces SET task_group_id = $1, updated_at = NOW() WHERE id = $2",
            group_id,
            workspace_id
        )
        .execute(pool)
        .await?;

        // Update workspace object to reflect the change
        workspace.task_group_id = Some(group_id);

        // Update group's workspace_id to point to this workspace
        sqlx::query!(
            "UPDATE task_groups SET workspace_id = $1 WHERE id = $2",
            workspace_id,
            group_id
        )
        .execute(pool)
        .await?;

        Ok(workspace)
    }
}

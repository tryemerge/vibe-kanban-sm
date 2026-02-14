use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, Postgres, PgPool};
use ts_rs::TS;
use uuid::Uuid;

/// Scope level for a state transition (determines override priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TransitionScope {
    /// Board-level default (lowest priority)
    Board,
    /// Project-level override
    Project,
    /// Task-level override (highest priority)
    Task,
}

/// Defines an allowed transition between two Kanban columns
/// Supports hierarchical overrides: task -> project -> board
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct StateTransition {
    pub id: Uuid,
    /// Board ID for board-level transitions (NULL for project/task level)
    pub board_id: Option<Uuid>,
    /// Project ID for project-level transitions (NULL for board/task level)
    pub project_id: Option<Uuid>,
    /// Task ID for task-level transitions (NULL for board/project level)
    pub task_id: Option<Uuid>,
    pub from_column_id: Uuid,
    /// Where to go when condition matches (success path)
    pub to_column_id: Uuid,
    /// Where to go when condition doesn't match (else path)
    pub else_column_id: Option<Uuid>,
    /// Where to go when max_failures is reached (escalation path)
    pub escalation_column_id: Option<Uuid>,
    pub name: Option<String>,
    pub requires_confirmation: bool,
    /// JSON key to check in .vibe/decision.json (e.g., "decision")
    pub condition_key: Option<String>,
    /// Value to match for this transition (e.g., "approve" or "reject")
    pub condition_value: Option<String>,
    /// Number of times the else path can be taken before escalation
    pub max_failures: Option<i32>,
    pub is_template: bool,
    pub template_group_id: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

impl StateTransition {
    /// Get the scope level of this transition
    pub fn scope(&self) -> TransitionScope {
        if self.task_id.is_some() {
            TransitionScope::Task
        } else if self.project_id.is_some() {
            TransitionScope::Project
        } else {
            TransitionScope::Board
        }
    }
}

/// Transition with column names for UI display
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct StateTransitionWithColumns {
    pub id: Uuid,
    pub board_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub from_column_id: Uuid,
    pub from_column_name: String,
    /// Where to go when condition matches (success path)
    pub to_column_id: Uuid,
    pub to_column_name: String,
    /// Where to go when condition doesn't match (else path)
    pub else_column_id: Option<Uuid>,
    pub else_column_name: Option<String>,
    /// Where to go when max_failures is reached (escalation path)
    pub escalation_column_id: Option<Uuid>,
    pub escalation_column_name: Option<String>,
    pub name: Option<String>,
    pub requires_confirmation: bool,
    pub condition_key: Option<String>,
    pub condition_value: Option<String>,
    /// Number of times the else path can be taken before escalation
    pub max_failures: Option<i32>,
    /// Computed scope for UI display
    pub scope: TransitionScope,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateStateTransition {
    pub from_column_id: Uuid,
    /// Where to go when condition matches (success path)
    pub to_column_id: Uuid,
    /// Where to go when condition doesn't match (else path)
    pub else_column_id: Option<Uuid>,
    /// Where to go when max_failures is reached (escalation path)
    pub escalation_column_id: Option<Uuid>,
    pub name: Option<String>,
    pub requires_confirmation: Option<bool>,
    pub condition_key: Option<String>,
    pub condition_value: Option<String>,
    /// Number of times the else path can be taken before escalation
    pub max_failures: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct UpdateStateTransition {
    pub from_column_id: Option<Uuid>,
    pub to_column_id: Option<Uuid>,
    /// Double Option: None = keep existing, Some(None) = set null, Some(Some(id)) = set value
    #[serde(default, deserialize_with = "crate::serde_helpers::deserialize_optional_nullable")]
    #[ts(optional, type = "string | null")]
    pub else_column_id: Option<Option<Uuid>>,
    #[serde(default, deserialize_with = "crate::serde_helpers::deserialize_optional_nullable")]
    #[ts(optional, type = "string | null")]
    pub escalation_column_id: Option<Option<Uuid>>,
    pub name: Option<String>,
    pub requires_confirmation: Option<bool>,
    pub condition_key: Option<String>,
    pub condition_value: Option<String>,
    pub max_failures: Option<i32>,
}

impl StateTransition {
    /// Find a transition by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            StateTransition,
            r#"SELECT id as "id!: Uuid",
                      board_id as "board_id: Uuid",
                      project_id as "project_id: Uuid",
                      task_id as "task_id: Uuid",
                      from_column_id as "from_column_id!: Uuid",
                      to_column_id as "to_column_id!: Uuid",
                      else_column_id as "else_column_id: Uuid",
                      escalation_column_id as "escalation_column_id: Uuid",
                      name,
                      requires_confirmation as "requires_confirmation!: bool",
                      condition_key,
                      condition_value,
                      max_failures,
                      is_template as "is_template!: bool",
                      template_group_id,
                      created_at as "created_at!: DateTime<Utc>"
               FROM state_transitions
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find all transitions for a board (board-level only)
    pub async fn find_by_board(
        pool: &PgPool,
        board_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            StateTransition,
            r#"SELECT id as "id!: Uuid",
                      board_id as "board_id: Uuid",
                      project_id as "project_id: Uuid",
                      task_id as "task_id: Uuid",
                      from_column_id as "from_column_id!: Uuid",
                      to_column_id as "to_column_id!: Uuid",
                      else_column_id as "else_column_id: Uuid",
                      escalation_column_id as "escalation_column_id: Uuid",
                      name,
                      requires_confirmation as "requires_confirmation!: bool",
                      condition_key,
                      condition_value,
                      max_failures,
                      is_template as "is_template!: bool",
                      template_group_id,
                      created_at as "created_at!: DateTime<Utc>"
               FROM state_transitions
               WHERE board_id = $1 AND project_id IS NULL AND task_id IS NULL AND is_template = FALSE"#,
            board_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all transitions for a project (project-level only, not board defaults)
    pub async fn find_by_project(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            StateTransition,
            r#"SELECT id as "id!: Uuid",
                      board_id as "board_id: Uuid",
                      project_id as "project_id: Uuid",
                      task_id as "task_id: Uuid",
                      from_column_id as "from_column_id!: Uuid",
                      to_column_id as "to_column_id!: Uuid",
                      else_column_id as "else_column_id: Uuid",
                      escalation_column_id as "escalation_column_id: Uuid",
                      name,
                      requires_confirmation as "requires_confirmation!: bool",
                      condition_key,
                      condition_value,
                      max_failures,
                      is_template as "is_template!: bool",
                      template_group_id,
                      created_at as "created_at!: DateTime<Utc>"
               FROM state_transitions
               WHERE project_id = $1 AND task_id IS NULL"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all transitions for a task (task-level only)
    pub async fn find_by_task(
        pool: &PgPool,
        task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            StateTransition,
            r#"SELECT id as "id!: Uuid",
                      board_id as "board_id: Uuid",
                      project_id as "project_id: Uuid",
                      task_id as "task_id: Uuid",
                      from_column_id as "from_column_id!: Uuid",
                      to_column_id as "to_column_id!: Uuid",
                      else_column_id as "else_column_id: Uuid",
                      escalation_column_id as "escalation_column_id: Uuid",
                      name,
                      requires_confirmation as "requires_confirmation!: bool",
                      condition_key,
                      condition_value,
                      max_failures,
                      is_template as "is_template!: bool",
                      template_group_id,
                      created_at as "created_at!: DateTime<Utc>"
               FROM state_transitions
               WHERE task_id = $1"#,
            task_id
        )
        .fetch_all(pool)
        .await
    }

    /// Resolve effective transitions for a task with hierarchical override
    /// Priority: task-level > project-level > board-level
    /// Returns transitions grouped by from_column_id with highest priority wins
    pub async fn resolve_effective_transitions(
        pool: &PgPool,
        task_id: Uuid,
        project_id: Uuid,
        board_id: Option<Uuid>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        // Query all applicable transitions, ordered by priority (task first, then project, then board)
        // Use COALESCE to create a priority value: task=1, project=2, board=3
        sqlx::query_as!(
            StateTransition,
            r#"WITH prioritized AS (
                SELECT *,
                    CASE
                        WHEN task_id IS NOT NULL THEN 1
                        WHEN project_id IS NOT NULL THEN 2
                        ELSE 3
                    END as priority
                FROM state_transitions
                WHERE is_template = FALSE
                  AND (task_id = $1
                   OR (project_id = $2 AND task_id IS NULL)
                   OR (board_id = $3 AND project_id IS NULL AND task_id IS NULL))
            ),
            ranked AS (
                SELECT *,
                    ROW_NUMBER() OVER (
                        PARTITION BY from_column_id, to_column_id, condition_key, condition_value
                        ORDER BY priority ASC
                    ) as rn
                FROM prioritized
            )
            SELECT id as "id!: Uuid",
                   board_id as "board_id: Uuid",
                   project_id as "project_id: Uuid",
                   task_id as "task_id: Uuid",
                   from_column_id as "from_column_id!: Uuid",
                   to_column_id as "to_column_id!: Uuid",
                   else_column_id as "else_column_id: Uuid",
                   escalation_column_id as "escalation_column_id: Uuid",
                   name,
                   requires_confirmation as "requires_confirmation!: bool",
                   condition_key,
                   condition_value,
                   max_failures,
                   is_template as "is_template!: bool",
                   template_group_id,
                   created_at as "created_at!: DateTime<Utc>"
            FROM ranked
            WHERE rn = 1"#,
            task_id,
            project_id,
            board_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find transitions from a specific column for a task (with hierarchy resolution)
    pub async fn find_from_column_for_task(
        pool: &PgPool,
        from_column_id: Uuid,
        task_id: Uuid,
        project_id: Uuid,
        board_id: Option<Uuid>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            StateTransition,
            r#"WITH prioritized AS (
                SELECT *,
                    CASE
                        WHEN task_id IS NOT NULL THEN 1
                        WHEN project_id IS NOT NULL THEN 2
                        ELSE 3
                    END as priority
                FROM state_transitions
                WHERE is_template = FALSE
                  AND from_column_id = $1
                  AND (task_id = $2
                       OR (project_id = $3 AND task_id IS NULL)
                       OR (board_id = $4 AND project_id IS NULL AND task_id IS NULL))
            ),
            ranked AS (
                SELECT *,
                    ROW_NUMBER() OVER (
                        PARTITION BY to_column_id, condition_key, condition_value
                        ORDER BY priority ASC
                    ) as rn
                FROM prioritized
            )
            SELECT id as "id!: Uuid",
                   board_id as "board_id: Uuid",
                   project_id as "project_id: Uuid",
                   task_id as "task_id: Uuid",
                   from_column_id as "from_column_id!: Uuid",
                   to_column_id as "to_column_id!: Uuid",
                   else_column_id as "else_column_id: Uuid",
                   escalation_column_id as "escalation_column_id: Uuid",
                   name,
                   requires_confirmation as "requires_confirmation!: bool",
                   condition_key,
                   condition_value,
                   max_failures,
                   is_template as "is_template!: bool",
                   template_group_id,
                   created_at as "created_at!: DateTime<Utc>"
            FROM ranked
            WHERE rn = 1"#,
            from_column_id,
            task_id,
            project_id,
            board_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all transitions with column names for board (board-level)
    pub async fn find_by_board_with_columns(
        pool: &PgPool,
        board_id: Uuid,
    ) -> Result<Vec<StateTransitionWithColumns>, sqlx::Error> {
        let records = sqlx::query!(
            r#"SELECT st.id as "id!: Uuid",
                      st.board_id as "board_id: Uuid",
                      st.project_id as "project_id: Uuid",
                      st.task_id as "task_id: Uuid",
                      st.from_column_id as "from_column_id!: Uuid",
                      fc.name as "from_column_name!",
                      st.to_column_id as "to_column_id!: Uuid",
                      tc.name as "to_column_name!",
                      st.else_column_id as "else_column_id: Uuid",
                      ec.name as "else_column_name: Option<String>",
                      st.escalation_column_id as "escalation_column_id: Uuid",
                      esc.name as "escalation_column_name: Option<String>",
                      st.name,
                      st.requires_confirmation as "requires_confirmation!: bool",
                      st.condition_key,
                      st.condition_value,
                      st.max_failures,
                      st.created_at as "created_at!: DateTime<Utc>"
               FROM state_transitions st
               JOIN kanban_columns fc ON fc.id = st.from_column_id
               JOIN kanban_columns tc ON tc.id = st.to_column_id
               LEFT JOIN kanban_columns ec ON ec.id = st.else_column_id
               LEFT JOIN kanban_columns esc ON esc.id = st.escalation_column_id
               WHERE st.board_id = $1 AND st.project_id IS NULL AND st.task_id IS NULL"#,
            board_id
        )
        .fetch_all(pool)
        .await?;

        Ok(records
            .into_iter()
            .map(|r| StateTransitionWithColumns {
                id: r.id,
                board_id: r.board_id,
                project_id: r.project_id,
                task_id: r.task_id,
                from_column_id: r.from_column_id,
                from_column_name: r.from_column_name,
                to_column_id: r.to_column_id,
                to_column_name: r.to_column_name,
                else_column_id: r.else_column_id,
                else_column_name: r.else_column_name,
                escalation_column_id: r.escalation_column_id,
                escalation_column_name: r.escalation_column_name,
                name: r.name,
                requires_confirmation: r.requires_confirmation,
                condition_key: r.condition_key,
                condition_value: r.condition_value,
                max_failures: r.max_failures,
                scope: TransitionScope::Board,
                created_at: r.created_at,
            })
            .collect())
    }

    /// Find all transitions with column names for project (project-level only)
    pub async fn find_by_project_with_columns(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Vec<StateTransitionWithColumns>, sqlx::Error> {
        let records = sqlx::query!(
            r#"SELECT st.id as "id!: Uuid",
                      st.board_id as "board_id: Uuid",
                      st.project_id as "project_id: Uuid",
                      st.task_id as "task_id: Uuid",
                      st.from_column_id as "from_column_id!: Uuid",
                      fc.name as "from_column_name!",
                      st.to_column_id as "to_column_id!: Uuid",
                      tc.name as "to_column_name!",
                      st.else_column_id as "else_column_id: Uuid",
                      ec.name as "else_column_name: Option<String>",
                      st.escalation_column_id as "escalation_column_id: Uuid",
                      esc.name as "escalation_column_name: Option<String>",
                      st.name,
                      st.requires_confirmation as "requires_confirmation!: bool",
                      st.condition_key,
                      st.condition_value,
                      st.max_failures,
                      st.created_at as "created_at!: DateTime<Utc>"
               FROM state_transitions st
               JOIN kanban_columns fc ON fc.id = st.from_column_id
               JOIN kanban_columns tc ON tc.id = st.to_column_id
               LEFT JOIN kanban_columns ec ON ec.id = st.else_column_id
               LEFT JOIN kanban_columns esc ON esc.id = st.escalation_column_id
               WHERE st.project_id = $1 AND st.task_id IS NULL"#,
            project_id
        )
        .fetch_all(pool)
        .await?;

        Ok(records
            .into_iter()
            .map(|r| StateTransitionWithColumns {
                id: r.id,
                board_id: r.board_id,
                project_id: r.project_id,
                task_id: r.task_id,
                from_column_id: r.from_column_id,
                from_column_name: r.from_column_name,
                to_column_id: r.to_column_id,
                to_column_name: r.to_column_name,
                else_column_id: r.else_column_id,
                else_column_name: r.else_column_name,
                escalation_column_id: r.escalation_column_id,
                escalation_column_name: r.escalation_column_name,
                name: r.name,
                requires_confirmation: r.requires_confirmation,
                condition_key: r.condition_key,
                condition_value: r.condition_value,
                max_failures: r.max_failures,
                scope: TransitionScope::Project,
                created_at: r.created_at,
            })
            .collect())
    }

    /// Check if a transition is allowed (with hierarchy resolution)
    pub async fn is_allowed(
        pool: &PgPool,
        task_id: Uuid,
        project_id: Uuid,
        board_id: Option<Uuid>,
        from_column_id: Uuid,
        to_column_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        // First check if any transitions are defined at any level
        let count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64"
               FROM state_transitions
               WHERE task_id = $1
                  OR (project_id = $2 AND task_id IS NULL)
                  OR (board_id = $3 AND project_id IS NULL AND task_id IS NULL)"#,
            task_id,
            project_id,
            board_id
        )
        .fetch_one(pool)
        .await?;

        // If no transitions defined at any level, all moves are allowed (open workflow)
        if count == 0 {
            return Ok(true);
        }

        // Check if this specific transition exists at any level
        let exists: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64"
               FROM state_transitions
               WHERE from_column_id = $1 AND to_column_id = $2
                 AND (task_id = $3
                      OR (project_id = $4 AND task_id IS NULL)
                      OR (board_id = $5 AND project_id IS NULL AND task_id IS NULL))"#,
            from_column_id,
            to_column_id,
            task_id,
            project_id,
            board_id
        )
        .fetch_one(pool)
        .await?;

        Ok(exists > 0)
    }

    /// Create a board-level transition
    pub async fn create_for_board<'e, E>(
        executor: E,
        board_id: Uuid,
        data: &CreateStateTransition,
    ) -> Result<Self, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4();
        let requires_confirmation: i32 = if data.requires_confirmation.unwrap_or(false) { 1 } else { 0 };
        let is_template: bool = false;
        let template_group_id: Option<String> = None;

        sqlx::query_as!(
            StateTransition,
            r#"INSERT INTO state_transitions (id, board_id, from_column_id, to_column_id, else_column_id, escalation_column_id, name, requires_confirmation, condition_key, condition_value, max_failures, is_template, template_group_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
               RETURNING id as "id!: Uuid",
                         board_id as "board_id: Uuid",
                         project_id as "project_id: Uuid",
                         task_id as "task_id: Uuid",
                         from_column_id as "from_column_id!: Uuid",
                         to_column_id as "to_column_id!: Uuid",
                         else_column_id as "else_column_id: Uuid",
                         escalation_column_id as "escalation_column_id: Uuid",
                         name,
                         requires_confirmation as "requires_confirmation!: bool",
                         condition_key,
                         condition_value,
                         max_failures,
                         is_template as "is_template!: bool",
                         template_group_id,
                         created_at as "created_at!: DateTime<Utc>""#,
            id,
            board_id,
            data.from_column_id,
            data.to_column_id,
            data.else_column_id,
            data.escalation_column_id,
            data.name,
            requires_confirmation,
            data.condition_key,
            data.condition_value,
            data.max_failures,
            is_template,
            template_group_id
        )
        .fetch_one(executor)
        .await
    }

    /// Create a project-level transition
    pub async fn create_for_project<'e, E>(
        executor: E,
        project_id: Uuid,
        data: &CreateStateTransition,
    ) -> Result<Self, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4();
        let requires_confirmation: i32 = if data.requires_confirmation.unwrap_or(false) { 1 } else { 0 };
        let is_template: bool = false;
        let template_group_id: Option<String> = None;

        sqlx::query_as!(
            StateTransition,
            r#"INSERT INTO state_transitions (id, project_id, from_column_id, to_column_id, else_column_id, escalation_column_id, name, requires_confirmation, condition_key, condition_value, max_failures, is_template, template_group_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
               RETURNING id as "id!: Uuid",
                         board_id as "board_id: Uuid",
                         project_id as "project_id: Uuid",
                         task_id as "task_id: Uuid",
                         from_column_id as "from_column_id!: Uuid",
                         to_column_id as "to_column_id!: Uuid",
                         else_column_id as "else_column_id: Uuid",
                         escalation_column_id as "escalation_column_id: Uuid",
                         name,
                         requires_confirmation as "requires_confirmation!: bool",
                         condition_key,
                         condition_value,
                         max_failures,
                         is_template as "is_template!: bool",
                         template_group_id,
                         created_at as "created_at!: DateTime<Utc>""#,
            id,
            project_id,
            data.from_column_id,
            data.to_column_id,
            data.else_column_id,
            data.escalation_column_id,
            data.name,
            requires_confirmation,
            data.condition_key,
            data.condition_value,
            data.max_failures,
            is_template,
            template_group_id
        )
        .fetch_one(executor)
        .await
    }

    /// Create a task-level transition
    pub async fn create_for_task<'e, E>(
        executor: E,
        task_id: Uuid,
        data: &CreateStateTransition,
    ) -> Result<Self, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4();
        let requires_confirmation: i32 = if data.requires_confirmation.unwrap_or(false) { 1 } else { 0 };
        let is_template: bool = false;
        let template_group_id: Option<String> = None;

        sqlx::query_as!(
            StateTransition,
            r#"INSERT INTO state_transitions (id, task_id, from_column_id, to_column_id, else_column_id, escalation_column_id, name, requires_confirmation, condition_key, condition_value, max_failures, is_template, template_group_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
               RETURNING id as "id!: Uuid",
                         board_id as "board_id: Uuid",
                         project_id as "project_id: Uuid",
                         task_id as "task_id: Uuid",
                         from_column_id as "from_column_id!: Uuid",
                         to_column_id as "to_column_id!: Uuid",
                         else_column_id as "else_column_id: Uuid",
                         escalation_column_id as "escalation_column_id: Uuid",
                         name,
                         requires_confirmation as "requires_confirmation!: bool",
                         condition_key,
                         condition_value,
                         max_failures,
                         is_template as "is_template!: bool",
                         template_group_id,
                         created_at as "created_at!: DateTime<Utc>""#,
            id,
            task_id,
            data.from_column_id,
            data.to_column_id,
            data.else_column_id,
            data.escalation_column_id,
            data.name,
            requires_confirmation,
            data.condition_key,
            data.condition_value,
            data.max_failures,
            is_template,
            template_group_id
        )
        .fetch_one(executor)
        .await
    }

    /// Update a transition
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        data: &UpdateStateTransition,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let from_column_id = data.from_column_id.unwrap_or(existing.from_column_id);
        let to_column_id = data.to_column_id.unwrap_or(existing.to_column_id);
        let else_column_id = match &data.else_column_id {
            None => existing.else_column_id,
            Some(inner) => inner.clone(),
        };
        let escalation_column_id = match &data.escalation_column_id {
            None => existing.escalation_column_id,
            Some(inner) => inner.clone(),
        };
        let name = data.name.clone().or(existing.name);
        let requires_confirmation: bool = data.requires_confirmation.unwrap_or(existing.requires_confirmation);
        let requires_confirmation_i32: i32 = if requires_confirmation { 1 } else { 0 };
        let condition_key = data.condition_key.clone().or(existing.condition_key);
        let condition_value = data.condition_value.clone().or(existing.condition_value);
        let max_failures = data.max_failures.or(existing.max_failures);

        sqlx::query_as!(
            StateTransition,
            r#"UPDATE state_transitions
               SET from_column_id = $2, to_column_id = $3, else_column_id = $4,
                   escalation_column_id = $5, name = $6, requires_confirmation = $7,
                   condition_key = $8, condition_value = $9, max_failures = $10
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         board_id as "board_id: Uuid",
                         project_id as "project_id: Uuid",
                         task_id as "task_id: Uuid",
                         from_column_id as "from_column_id!: Uuid",
                         to_column_id as "to_column_id!: Uuid",
                         else_column_id as "else_column_id: Uuid",
                         escalation_column_id as "escalation_column_id: Uuid",
                         name,
                         requires_confirmation as "requires_confirmation!: bool",
                         condition_key,
                         condition_value,
                         max_failures,
                         is_template as "is_template!: bool",
                         template_group_id,
                         created_at as "created_at!: DateTime<Utc>""#,
            id,
            from_column_id,
            to_column_id,
            else_column_id,
            escalation_column_id,
            name,
            requires_confirmation_i32,
            condition_key,
            condition_value,
            max_failures
        )
        .fetch_one(pool)
        .await
    }

    /// Delete a transition
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM state_transitions WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Delete all transitions for a board
    pub async fn delete_by_board(pool: &PgPool, board_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM state_transitions WHERE board_id = $1 AND project_id IS NULL AND task_id IS NULL",
            board_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Delete all transitions for a project
    pub async fn delete_by_project(pool: &PgPool, project_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM state_transitions WHERE project_id = $1 AND task_id IS NULL",
            project_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Delete all transitions for a task
    pub async fn delete_by_task(pool: &PgPool, task_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM state_transitions WHERE task_id = $1",
            task_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Find all template transitions by template group ID
    pub async fn find_by_template_group(
        pool: &PgPool,
        template_group_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            StateTransition,
            r#"SELECT id as "id!: Uuid",
                      board_id as "board_id: Uuid",
                      project_id as "project_id: Uuid",
                      task_id as "task_id: Uuid",
                      from_column_id as "from_column_id!: Uuid",
                      to_column_id as "to_column_id!: Uuid",
                      else_column_id as "else_column_id: Uuid",
                      escalation_column_id as "escalation_column_id: Uuid",
                      name,
                      requires_confirmation as "requires_confirmation!: bool",
                      condition_key,
                      condition_value,
                      max_failures,
                      is_template as "is_template!: bool",
                      template_group_id,
                      created_at as "created_at!: DateTime<Utc>"
               FROM state_transitions
               WHERE template_group_id = $1"#,
            template_group_id
        )
        .fetch_all(pool)
        .await
    }
}

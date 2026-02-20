use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, Postgres, PgPool};
use ts_rs::TS;
use uuid::Uuid;

use super::task::TaskStatus;

/// A customizable Kanban column representing a task state
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct KanbanColumn {
    pub id: Uuid,
    pub board_id: Uuid,
    pub name: String,
    pub slug: String,
    pub position: i32,
    pub color: Option<String>,
    pub is_initial: bool,
    pub is_terminal: bool,
    /// When true, entering this column creates a new attempt/workspace
    pub starts_workflow: bool,
    pub status: TaskStatus, // Workflow status this column maps to
    pub agent_id: Option<Uuid>, // Agent assigned to handle tasks in this column
    /// What the agent should produce before moving to the next column
    pub deliverable: Option<String>,
    /// Question the agent must answer before moving to the next column
    pub question: Option<String>,
    /// JSON array of valid answer options for the question
    pub answer_options: Option<String>,
    pub is_template: bool,
    pub template_group_id: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateKanbanColumn {
    pub name: String,
    pub slug: String,
    pub position: i32,
    pub color: Option<String>,
    pub is_initial: Option<bool>,
    pub is_terminal: Option<bool>,
    pub starts_workflow: Option<bool>,
    pub status: Option<TaskStatus>, // Defaults to 'todo' if not specified
    pub agent_id: Option<Uuid>,
    pub deliverable: Option<String>,
    pub question: Option<String>,
    pub answer_options: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct UpdateKanbanColumn {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub position: Option<i32>,
    pub color: Option<String>,
    pub is_initial: Option<bool>,
    pub is_terminal: Option<bool>,
    pub starts_workflow: Option<bool>,
    pub status: Option<TaskStatus>,
    /// Agent ID - uses double Option to distinguish between "not provided" (None) and "explicitly null" (Some(None))
    /// - None: Keep existing value (field not in request)
    /// - Some(None): Clear the agent (field is null in request)
    /// - Some(Some(uuid)): Set to new agent
    #[serde(default, deserialize_with = "crate::serde_helpers::deserialize_optional_nullable")]
    #[ts(optional, type = "string | null")]
    pub agent_id: Option<Option<Uuid>>,
    pub deliverable: Option<String>,
    pub question: Option<String>,
    pub answer_options: Option<String>,
}

impl KanbanColumn {
    /// Find all columns for a board, ordered by position
    pub async fn find_by_board(
        pool: &PgPool,
        board_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanColumn,
            r#"SELECT id as "id!: Uuid",
                      board_id as "board_id!: Uuid",
                      name,
                      slug,
                      position as "position!: i32",
                      color,
                      is_initial as "is_initial!: bool",
                      is_terminal as "is_terminal!: bool",
                      starts_workflow as "starts_workflow!: bool",
                      status as "status!: TaskStatus",
                      agent_id as "agent_id: Uuid",
                      deliverable,
                      question,
                      answer_options,
                      is_template as "is_template!: bool",
                      template_group_id,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_columns
               WHERE board_id = $1 AND is_template = FALSE
               ORDER BY position ASC"#,
            board_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find a column by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanColumn,
            r#"SELECT id as "id!: Uuid",
                      board_id as "board_id!: Uuid",
                      name,
                      slug,
                      position as "position!: i32",
                      color,
                      is_initial as "is_initial!: bool",
                      is_terminal as "is_terminal!: bool",
                      starts_workflow as "starts_workflow!: bool",
                      status as "status!: TaskStatus",
                      agent_id as "agent_id: Uuid",
                      deliverable,
                      question,
                      answer_options,
                      is_template as "is_template!: bool",
                      template_group_id,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_columns
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find a column by board and slug
    pub async fn find_by_slug(
        pool: &PgPool,
        board_id: Uuid,
        slug: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanColumn,
            r#"SELECT id as "id!: Uuid",
                      board_id as "board_id!: Uuid",
                      name,
                      slug,
                      position as "position!: i32",
                      color,
                      is_initial as "is_initial!: bool",
                      is_terminal as "is_terminal!: bool",
                      starts_workflow as "starts_workflow!: bool",
                      status as "status!: TaskStatus",
                      agent_id as "agent_id: Uuid",
                      deliverable,
                      question,
                      answer_options,
                      is_template as "is_template!: bool",
                      template_group_id,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_columns
               WHERE board_id = $1 AND slug = $2"#,
            board_id,
            slug
        )
        .fetch_optional(pool)
        .await
    }

    /// Find the initial column for a board
    pub async fn find_initial(
        pool: &PgPool,
        board_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanColumn,
            r#"SELECT id as "id!: Uuid",
                      board_id as "board_id!: Uuid",
                      name,
                      slug,
                      position as "position!: i32",
                      color,
                      is_initial as "is_initial!: bool",
                      is_terminal as "is_terminal!: bool",
                      starts_workflow as "starts_workflow!: bool",
                      status as "status!: TaskStatus",
                      agent_id as "agent_id: Uuid",
                      deliverable,
                      question,
                      answer_options,
                      is_template as "is_template!: bool",
                      template_group_id,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_columns
               WHERE board_id = $1 AND is_initial = true
               LIMIT 1"#,
            board_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find the workflow start column for a board (where tasks go when auto-started)
    pub async fn find_workflow_start(
        pool: &PgPool,
        board_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanColumn,
            r#"SELECT id as "id!: Uuid",
                      board_id as "board_id!: Uuid",
                      name,
                      slug,
                      position as "position!: i32",
                      color,
                      is_initial as "is_initial!: bool",
                      is_terminal as "is_terminal!: bool",
                      starts_workflow as "starts_workflow!: bool",
                      status as "status!: TaskStatus",
                      agent_id as "agent_id: Uuid",
                      deliverable,
                      question,
                      answer_options,
                      is_template as "is_template!: bool",
                      template_group_id,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_columns
               WHERE board_id = $1 AND starts_workflow = true
               LIMIT 1"#,
            board_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new column for a board
    pub async fn create_for_board<'e, E>(
        executor: E,
        board_id: Uuid,
        data: &CreateKanbanColumn,
    ) -> Result<Self, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4();
        let is_initial: bool = data.is_initial.unwrap_or(false);
        let is_terminal: bool = data.is_terminal.unwrap_or(false);
        let starts_workflow: bool = data.starts_workflow.unwrap_or(false);
        let status = data.status.clone().unwrap_or(TaskStatus::Todo);
        let status_str = status.to_string();
        let is_template: bool = false; // Regular columns are never templates
        let template_group_id: Option<String> = None;

        sqlx::query_as!(
            KanbanColumn,
            r#"INSERT INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal, starts_workflow, status, agent_id, deliverable, question, answer_options, is_template, template_group_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
               RETURNING id as "id!: Uuid",
                         board_id as "board_id!: Uuid",
                         name,
                         slug,
                         position as "position!: i32",
                         color,
                         is_initial as "is_initial!: bool",
                         is_terminal as "is_terminal!: bool",
                         starts_workflow as "starts_workflow!: bool",
                         status as "status!: TaskStatus",
                         agent_id as "agent_id: Uuid",
                         deliverable,
                         question,
                         answer_options,
                         is_template as "is_template!: bool",
                         template_group_id,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            board_id,
            data.name,
            data.slug,
            data.position,
            data.color,
            is_initial,
            is_terminal,
            starts_workflow,
            status_str,
            data.agent_id,
            data.deliverable,
            data.question,
            data.answer_options,
            is_template,
            template_group_id
        )
        .fetch_one(executor)
        .await
    }

    /// Clone a column as a template
    pub async fn clone_as_template(
        pool: &PgPool,
        source: &KanbanColumn,
        template_board_id: Uuid,
        template_group_id: &str,
        new_agent_id: Option<Uuid>,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let status_str = source.status.to_string();
        let is_template: bool = true;

        sqlx::query_as!(
            KanbanColumn,
            r#"INSERT INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal, starts_workflow, status, agent_id, deliverable, question, answer_options, is_template, template_group_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
               RETURNING id as "id!: Uuid",
                         board_id as "board_id!: Uuid",
                         name,
                         slug,
                         position as "position!: i32",
                         color,
                         is_initial as "is_initial!: bool",
                         is_terminal as "is_terminal!: bool",
                         starts_workflow as "starts_workflow!: bool",
                         status as "status!: TaskStatus",
                         agent_id as "agent_id: Uuid",
                         deliverable,
                         question,
                         answer_options,
                         is_template as "is_template!: bool",
                         template_group_id,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            template_board_id,
            source.name,
            source.slug,
            source.position,
            source.color,
            source.is_initial,
            source.is_terminal,
            source.starts_workflow,
            status_str,
            new_agent_id,
            source.deliverable,
            source.question,
            source.answer_options,
            is_template,
            template_group_id
        )
        .fetch_one(pool)
        .await
    }

    /// Update a column
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        data: &UpdateKanbanColumn,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.clone().unwrap_or(existing.name);
        let slug = data.slug.clone().unwrap_or(existing.slug);
        let position = data.position.unwrap_or(existing.position);
        let color = data.color.clone().or(existing.color);
        let is_initial: bool = data.is_initial.unwrap_or(existing.is_initial);
        let is_terminal: bool = data.is_terminal.unwrap_or(existing.is_terminal);
        let starts_workflow: bool = data.starts_workflow.unwrap_or(existing.starts_workflow);
        let status = data.status.clone().unwrap_or(existing.status);
        let status_str = status.to_string();
        // Handle Option<Option<Uuid>> for agent_id:
        // - None: keep existing value (field not in request)
        // - Some(None): clear the agent (explicitly set to null)
        // - Some(Some(uuid)): set to new agent
        let agent_id = match &data.agent_id {
            None => existing.agent_id,
            Some(inner) => inner.clone(),
        };
        let deliverable = data.deliverable.clone().or(existing.deliverable);
        let question = data.question.clone().or(existing.question);
        let answer_options = data.answer_options.clone().or(existing.answer_options);

        sqlx::query_as!(
            KanbanColumn,
            r#"UPDATE kanban_columns
               SET name = $2, slug = $3, position = $4, color = $5, is_initial = $6, is_terminal = $7, starts_workflow = $8, status = $9, agent_id = $10, deliverable = $11, question = $12, answer_options = $13,
                   updated_at = NOW()
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         board_id as "board_id!: Uuid",
                         name,
                         slug,
                         position as "position!: i32",
                         color,
                         is_initial as "is_initial!: bool",
                         is_terminal as "is_terminal!: bool",
                         starts_workflow as "starts_workflow!: bool",
                         status as "status!: TaskStatus",
                         agent_id as "agent_id: Uuid",
                         deliverable,
                         question,
                         answer_options,
                         is_template as "is_template!: bool",
                         template_group_id,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            slug,
            position,
            color,
            is_initial,
            is_terminal,
            starts_workflow,
            status_str,
            agent_id,
            deliverable,
            question,
            answer_options
        )
        .fetch_one(pool)
        .await
    }

    /// Reorder columns - update positions for all columns in a board
    pub async fn reorder_board(
        pool: &PgPool,
        board_id: Uuid,
        column_ids: &[Uuid],
    ) -> Result<(), sqlx::Error> {
        for (position, column_id) in column_ids.iter().enumerate() {
            let pos = position as i32;
            sqlx::query!(
                r#"UPDATE kanban_columns
                   SET position = $2, updated_at = NOW()
                   WHERE id = $1 AND board_id = $3"#,
                column_id,
                pos,
                board_id
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    /// Delete a column
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result: sqlx::postgres::PgQueryResult =
            sqlx::query!("DELETE FROM kanban_columns WHERE id = $1", id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected())
    }

    /// Delete all columns for a board (used when applying templates)
    pub async fn delete_by_board(pool: &PgPool, board_id: Uuid) -> Result<u64, sqlx::Error> {
        let result: sqlx::postgres::PgQueryResult = sqlx::query!(
            "DELETE FROM kanban_columns WHERE board_id = $1 AND is_template = FALSE",
            board_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Atomically set a column as the initial column for its board.
    /// Clears is_initial from all other non-template columns on the same board first.
    pub async fn set_as_initial(
        pool: &PgPool,
        board_id: Uuid,
        column_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query!(
            "UPDATE kanban_columns SET is_initial = FALSE, updated_at = NOW() WHERE board_id = $1 AND is_initial = TRUE AND is_template = FALSE",
            board_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "UPDATE kanban_columns SET is_initial = TRUE, updated_at = NOW() WHERE id = $1 AND board_id = $2",
            column_id,
            board_id
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Clear the initial column flag for a board (set none as initial).
    pub async fn clear_initial(pool: &PgPool, board_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE kanban_columns SET is_initial = FALSE, updated_at = NOW() WHERE board_id = $1 AND is_initial = TRUE AND is_template = FALSE",
            board_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Atomically set a column as the workflow start column for its board.
    /// Clears starts_workflow from all other non-template columns on the same board first.
    pub async fn set_as_workflow_start(
        pool: &PgPool,
        board_id: Uuid,
        column_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query!(
            "UPDATE kanban_columns SET starts_workflow = FALSE, updated_at = NOW() WHERE board_id = $1 AND starts_workflow = TRUE AND is_template = FALSE",
            board_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "UPDATE kanban_columns SET starts_workflow = TRUE, updated_at = NOW() WHERE id = $1 AND board_id = $2",
            column_id,
            board_id
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Clear the workflow start column flag for a board (set none as workflow start).
    pub async fn clear_workflow_start(pool: &PgPool, board_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE kanban_columns SET starts_workflow = FALSE, updated_at = NOW() WHERE board_id = $1 AND starts_workflow = TRUE AND is_template = FALSE",
            board_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Set or clear the terminal flag for a single column.
    pub async fn set_terminal(
        pool: &PgPool,
        column_id: Uuid,
        is_terminal: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE kanban_columns SET is_terminal = $2, updated_at = NOW() WHERE id = $1",
            column_id,
            is_terminal
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Batch-update terminal columns for a board: set specified columns as terminal,
    /// clear all others.
    pub async fn set_terminal_columns(
        pool: &PgPool,
        board_id: Uuid,
        terminal_column_ids: &[Uuid],
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        // Clear all terminal flags for this board
        sqlx::query!(
            "UPDATE kanban_columns SET is_terminal = FALSE, updated_at = NOW() WHERE board_id = $1 AND is_template = FALSE",
            board_id
        )
        .execute(&mut *tx)
        .await?;
        // Set terminal on specified columns
        for col_id in terminal_column_ids {
            sqlx::query!(
                "UPDATE kanban_columns SET is_terminal = TRUE, updated_at = NOW() WHERE id = $1 AND board_id = $2",
                col_id,
                board_id
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Find all template columns by template group ID
    pub async fn find_by_template_group(
        pool: &PgPool,
        template_group_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanColumn,
            r#"SELECT id as "id!: Uuid",
                      board_id as "board_id!: Uuid",
                      name,
                      slug,
                      position as "position!: i32",
                      color,
                      is_initial as "is_initial!: bool",
                      is_terminal as "is_terminal!: bool",
                      starts_workflow as "starts_workflow!: bool",
                      status as "status!: TaskStatus",
                      agent_id as "agent_id: Uuid",
                      deliverable,
                      question,
                      answer_options,
                      is_template as "is_template!: bool",
                      template_group_id,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_columns
               WHERE template_group_id = $1
               ORDER BY position ASC"#,
            template_group_id
        )
        .fetch_all(pool)
        .await
    }
}

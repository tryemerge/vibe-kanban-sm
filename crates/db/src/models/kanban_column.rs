use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, Sqlite, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// A customizable Kanban column representing a task state
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct KanbanColumn {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub slug: String,
    pub position: i32,
    pub color: Option<String>,
    pub is_initial: bool,
    pub is_terminal: bool,
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
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct UpdateKanbanColumn {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub position: Option<i32>,
    pub color: Option<String>,
    pub is_initial: Option<bool>,
    pub is_terminal: Option<bool>,
}

impl KanbanColumn {
    /// Find all columns for a project, ordered by position
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanColumn,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      name,
                      slug,
                      position as "position!: i32",
                      color,
                      is_initial as "is_initial!: bool",
                      is_terminal as "is_terminal!: bool",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_columns
               WHERE project_id = $1
               ORDER BY position ASC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find a column by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanColumn,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      name,
                      slug,
                      position as "position!: i32",
                      color,
                      is_initial as "is_initial!: bool",
                      is_terminal as "is_terminal!: bool",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_columns
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find a column by project and slug
    pub async fn find_by_slug(
        pool: &SqlitePool,
        project_id: Uuid,
        slug: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanColumn,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      name,
                      slug,
                      position as "position!: i32",
                      color,
                      is_initial as "is_initial!: bool",
                      is_terminal as "is_terminal!: bool",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_columns
               WHERE project_id = $1 AND slug = $2"#,
            project_id,
            slug
        )
        .fetch_optional(pool)
        .await
    }

    /// Find the initial column for a project
    pub async fn find_initial(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            KanbanColumn,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      name,
                      slug,
                      position as "position!: i32",
                      color,
                      is_initial as "is_initial!: bool",
                      is_terminal as "is_terminal!: bool",
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM kanban_columns
               WHERE project_id = $1 AND is_initial = 1
               LIMIT 1"#,
            project_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new column
    pub async fn create<'e, E>(
        executor: E,
        project_id: Uuid,
        data: &CreateKanbanColumn,
    ) -> Result<Self, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let id = Uuid::new_v4();
        let is_initial = data.is_initial.unwrap_or(false);
        let is_terminal = data.is_terminal.unwrap_or(false);

        sqlx::query_as!(
            KanbanColumn,
            r#"INSERT INTO kanban_columns (id, project_id, name, slug, position, color, is_initial, is_terminal)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         name,
                         slug,
                         position as "position!: i32",
                         color,
                         is_initial as "is_initial!: bool",
                         is_terminal as "is_terminal!: bool",
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            project_id,
            data.name,
            data.slug,
            data.position,
            data.color,
            is_initial,
            is_terminal
        )
        .fetch_one(executor)
        .await
    }

    /// Update a column
    pub async fn update(
        pool: &SqlitePool,
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
        let is_initial = data.is_initial.unwrap_or(existing.is_initial);
        let is_terminal = data.is_terminal.unwrap_or(existing.is_terminal);

        sqlx::query_as!(
            KanbanColumn,
            r#"UPDATE kanban_columns
               SET name = $2, slug = $3, position = $4, color = $5, is_initial = $6, is_terminal = $7,
                   updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         name,
                         slug,
                         position as "position!: i32",
                         color,
                         is_initial as "is_initial!: bool",
                         is_terminal as "is_terminal!: bool",
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            slug,
            position,
            color,
            is_initial,
            is_terminal
        )
        .fetch_one(pool)
        .await
    }

    /// Reorder columns - update positions for all columns in a project
    pub async fn reorder(
        pool: &SqlitePool,
        project_id: Uuid,
        column_ids: &[Uuid],
    ) -> Result<(), sqlx::Error> {
        for (position, column_id) in column_ids.iter().enumerate() {
            sqlx::query!(
                r#"UPDATE kanban_columns
                   SET position = $2, updated_at = datetime('now', 'subsec')
                   WHERE id = $1 AND project_id = $3"#,
                column_id,
                position as i32,
                project_id
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    /// Delete a column
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM kanban_columns WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use ts_rs::TS;
use uuid::Uuid;

/// A label that can be applied to tasks within a project
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskLabel {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub position: i32,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new task label
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateTaskLabel {
    pub project_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub position: Option<i32>,
}

/// Data for updating a task label
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UpdateTaskLabel {
    pub name: Option<String>,
    pub color: Option<String>,
    pub position: Option<i32>,
}

/// Assignment of a label to a task
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TaskLabelAssignment {
    pub task_id: Uuid,
    pub label_id: Uuid,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

impl TaskLabel {
    /// Find all labels for a project
    pub async fn find_by_project(pool: &PgPool, project_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskLabel,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                color,
                position as "position!: i32",
                created_at as "created_at!: DateTime<Utc>"
            FROM task_labels
            WHERE project_id = $1
            ORDER BY position ASC, name ASC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find a label by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskLabel,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                color,
                position as "position!: i32",
                created_at as "created_at!: DateTime<Utc>"
            FROM task_labels
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new label
    pub async fn create(pool: &PgPool, data: &CreateTaskLabel) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let position = data.position.unwrap_or(0);

        sqlx::query_as!(
            TaskLabel,
            r#"INSERT INTO task_labels (id, project_id, name, color, position)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                color,
                position as "position!: i32",
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            data.project_id,
            data.name,
            data.color,
            position
        )
        .fetch_one(pool)
        .await
    }

    /// Update a label
    pub async fn update(pool: &PgPool, id: Uuid, data: &UpdateTaskLabel) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.as_ref().unwrap_or(&existing.name);
        let color = data.color.as_ref().or(existing.color.as_ref());
        let position = data.position.unwrap_or(existing.position);

        sqlx::query_as!(
            TaskLabel,
            r#"UPDATE task_labels
            SET name = $2, color = $3, position = $4
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                color,
                position as "position!: i32",
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            name,
            color,
            position
        )
        .fetch_one(pool)
        .await
    }

    /// Delete a label (also removes all assignments)
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM task_labels WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Get labels for a specific task
    pub async fn find_by_task(pool: &PgPool, task_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TaskLabel,
            r#"SELECT
                tl.id as "id!: Uuid",
                tl.project_id as "project_id!: Uuid",
                tl.name,
                tl.color,
                tl.position as "position!: i32",
                tl.created_at as "created_at!: DateTime<Utc>"
            FROM task_labels tl
            INNER JOIN task_label_assignments tla ON tla.label_id = tl.id
            WHERE tla.task_id = $1
            ORDER BY tl.position ASC, tl.name ASC"#,
            task_id
        )
        .fetch_all(pool)
        .await
    }

    /// Assign a label to a task
    pub async fn assign_to_task(pool: &PgPool, task_id: Uuid, label_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"INSERT INTO task_label_assignments (task_id, label_id)
            VALUES ($1, $2)
            ON CONFLICT (task_id, label_id) DO NOTHING"#,
            task_id,
            label_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Remove a label from a task
    pub async fn remove_from_task(pool: &PgPool, task_id: Uuid, label_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM task_label_assignments WHERE task_id = $1 AND label_id = $2",
            task_id,
            label_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get all tasks with a specific label
    pub async fn get_task_ids_by_label(pool: &PgPool, label_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"SELECT task_id as "task_id!: Uuid" FROM task_label_assignments WHERE label_id = $1"#,
            label_id
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.task_id).collect())
    }

    /// Reorder labels within a project
    pub async fn reorder(pool: &PgPool, project_id: Uuid, label_ids: &[Uuid]) -> Result<(), sqlx::Error> {
        for (position, label_id) in label_ids.iter().enumerate() {
            sqlx::query!(
                "UPDATE task_labels SET position = $1 WHERE id = $2 AND project_id = $3",
                position as i32,
                label_id,
                project_id
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }
}

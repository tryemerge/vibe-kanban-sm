use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use ts_rs::TS;
use uuid::Uuid;

/// A Kanban board containing columns
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Board {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_template: bool,
    pub template_group_id: Option<String>,
    pub template_name: Option<String>,
    pub template_description: Option<String>,
    pub template_icon: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

/// Template info for listing templates
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TemplateInfo {
    pub id: Uuid,
    pub template_group_id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateBoard {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct UpdateBoard {
    pub name: Option<String>,
    pub description: Option<String>,
}

impl Board {
    /// Find all non-template boards
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Board,
            r#"SELECT id as "id!: Uuid",
                      name,
                      description,
                      is_template as "is_template!: bool",
                      template_group_id,
                      template_name,
                      template_description,
                      template_icon,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM boards
               WHERE is_template = FALSE
               ORDER BY name ASC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find all template boards for the template gallery
    pub async fn find_templates(pool: &PgPool) -> Result<Vec<TemplateInfo>, sqlx::Error> {
        sqlx::query_as!(
            TemplateInfo,
            r#"SELECT id as "id!: Uuid",
                      template_group_id as "template_group_id!",
                      template_name as "name!",
                      template_description as "description!",
                      template_icon as "icon!"
               FROM boards
               WHERE is_template = TRUE
               ORDER BY template_name ASC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find a board by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Board,
            r#"SELECT id as "id!: Uuid",
                      name,
                      description,
                      is_template as "is_template!: bool",
                      template_group_id,
                      template_name,
                      template_description,
                      template_icon,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM boards
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new board
    pub async fn create(pool: &PgPool, data: &CreateBoard) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();

        sqlx::query_as!(
            Board,
            r#"INSERT INTO boards (id, name, description, is_template, template_group_id, template_name, template_description, template_icon)
               VALUES ($1, $2, $3, FALSE, NULL, NULL, NULL, NULL)
               RETURNING id as "id!: Uuid",
                         name,
                         description,
                         is_template as "is_template!: bool",
                         template_group_id,
                         template_name,
                         template_description,
                         template_icon,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.name,
            data.description
        )
        .fetch_one(pool)
        .await
    }

    /// Update a board
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        data: &UpdateBoard,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.clone().unwrap_or(existing.name);
        let description = data.description.clone().or(existing.description);

        sqlx::query_as!(
            Board,
            r#"UPDATE boards
               SET name = $2, description = $3, updated_at = NOW()
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         name,
                         description,
                         is_template as "is_template!: bool",
                         template_group_id,
                         template_name,
                         template_description,
                         template_icon,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            description
        )
        .fetch_one(pool)
        .await
    }

    /// Create a template board from a source board
    pub async fn create_template(
        pool: &PgPool,
        name: &str,
        description: Option<&str>,
        template_group_id: &str,
        template_name: &str,
        template_description: &str,
        template_icon: &str,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();

        sqlx::query_as!(
            Board,
            r#"INSERT INTO boards (id, name, description, is_template, template_group_id, template_name, template_description, template_icon)
               VALUES ($1, $2, $3, TRUE, $4, $5, $6, $7)
               RETURNING id as "id!: Uuid",
                         name,
                         description,
                         is_template as "is_template!: bool",
                         template_group_id,
                         template_name,
                         template_description,
                         template_icon,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            description,
            template_group_id,
            template_name,
            template_description,
            template_icon
        )
        .fetch_one(pool)
        .await
    }

    /// Delete a board
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result: sqlx::postgres::PgQueryResult =
            sqlx::query!("DELETE FROM boards WHERE id = $1", id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected())
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// A Kanban board containing columns
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Board {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
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
    /// Find all boards
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Board,
            r#"SELECT id as "id!: Uuid",
                      name,
                      description,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM boards
               ORDER BY name ASC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find a board by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Board,
            r#"SELECT id as "id!: Uuid",
                      name,
                      description,
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
    pub async fn create(pool: &SqlitePool, data: &CreateBoard) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();

        sqlx::query_as!(
            Board,
            r#"INSERT INTO boards (id, name, description)
               VALUES ($1, $2, $3)
               RETURNING id as "id!: Uuid",
                         name,
                         description,
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
        pool: &SqlitePool,
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
               SET name = $2, description = $3, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         name,
                         description,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            description
        )
        .fetch_one(pool)
        .await
    }

    /// Delete a board
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result: sqlx::sqlite::SqliteQueryResult =
            sqlx::query!("DELETE FROM boards WHERE id = $1", id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected())
    }
}

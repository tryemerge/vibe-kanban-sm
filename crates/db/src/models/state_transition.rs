use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, Sqlite, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Defines an allowed transition between two Kanban columns
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct StateTransition {
    pub id: Uuid,
    pub project_id: Uuid,
    pub from_column_id: Uuid,
    pub to_column_id: Uuid,
    pub name: Option<String>,
    pub requires_confirmation: bool,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

/// Transition with column names for UI display
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct StateTransitionWithColumns {
    pub id: Uuid,
    pub project_id: Uuid,
    pub from_column_id: Uuid,
    pub from_column_name: String,
    pub to_column_id: Uuid,
    pub to_column_name: String,
    pub name: Option<String>,
    pub requires_confirmation: bool,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateStateTransition {
    pub from_column_id: Uuid,
    pub to_column_id: Uuid,
    pub name: Option<String>,
    pub requires_confirmation: Option<bool>,
}

impl StateTransition {
    /// Find a transition by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            StateTransition,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      from_column_id as "from_column_id!: Uuid",
                      to_column_id as "to_column_id!: Uuid",
                      name,
                      requires_confirmation as "requires_confirmation!: bool",
                      created_at as "created_at!: DateTime<Utc>"
               FROM state_transitions
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find all transitions for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            StateTransition,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      from_column_id as "from_column_id!: Uuid",
                      to_column_id as "to_column_id!: Uuid",
                      name,
                      requires_confirmation as "requires_confirmation!: bool",
                      created_at as "created_at!: DateTime<Utc>"
               FROM state_transitions
               WHERE project_id = $1"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all transitions with column names for UI display
    pub async fn find_by_project_with_columns(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<StateTransitionWithColumns>, sqlx::Error> {
        sqlx::query_as!(
            StateTransitionWithColumns,
            r#"SELECT st.id as "id!: Uuid",
                      st.project_id as "project_id!: Uuid",
                      st.from_column_id as "from_column_id!: Uuid",
                      fc.name as "from_column_name!",
                      st.to_column_id as "to_column_id!: Uuid",
                      tc.name as "to_column_name!",
                      st.name,
                      st.requires_confirmation as "requires_confirmation!: bool",
                      st.created_at as "created_at!: DateTime<Utc>"
               FROM state_transitions st
               JOIN kanban_columns fc ON fc.id = st.from_column_id
               JOIN kanban_columns tc ON tc.id = st.to_column_id
               WHERE st.project_id = $1"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find transitions from a specific column (valid next states)
    pub async fn find_from_column(
        pool: &SqlitePool,
        from_column_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            StateTransition,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      from_column_id as "from_column_id!: Uuid",
                      to_column_id as "to_column_id!: Uuid",
                      name,
                      requires_confirmation as "requires_confirmation!: bool",
                      created_at as "created_at!: DateTime<Utc>"
               FROM state_transitions
               WHERE from_column_id = $1"#,
            from_column_id
        )
        .fetch_all(pool)
        .await
    }

    /// Check if a transition is allowed
    pub async fn is_allowed(
        pool: &SqlitePool,
        project_id: Uuid,
        from_column_id: Uuid,
        to_column_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        // First check if any transitions are defined for this project
        let count: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64" FROM state_transitions WHERE project_id = $1"#,
            project_id
        )
        .fetch_one(pool)
        .await?;

        // If no transitions defined, all moves are allowed (open workflow)
        if count == 0 {
            return Ok(true);
        }

        // Check if this specific transition exists
        let exists: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64"
               FROM state_transitions
               WHERE from_column_id = $1 AND to_column_id = $2"#,
            from_column_id,
            to_column_id
        )
        .fetch_one(pool)
        .await?;

        Ok(exists > 0)
    }

    /// Create a new transition
    pub async fn create<'e, E>(
        executor: E,
        project_id: Uuid,
        data: &CreateStateTransition,
    ) -> Result<Self, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let id = Uuid::new_v4();
        let requires_confirmation = data.requires_confirmation.unwrap_or(false);

        sqlx::query_as!(
            StateTransition,
            r#"INSERT INTO state_transitions (id, project_id, from_column_id, to_column_id, name, requires_confirmation)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         from_column_id as "from_column_id!: Uuid",
                         to_column_id as "to_column_id!: Uuid",
                         name,
                         requires_confirmation as "requires_confirmation!: bool",
                         created_at as "created_at!: DateTime<Utc>""#,
            id,
            project_id,
            data.from_column_id,
            data.to_column_id,
            data.name,
            requires_confirmation
        )
        .fetch_one(executor)
        .await
    }

    /// Delete a transition
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM state_transitions WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Delete all transitions for a project
    pub async fn delete_by_project(pool: &SqlitePool, project_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM state_transitions WHERE project_id = $1",
            project_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}

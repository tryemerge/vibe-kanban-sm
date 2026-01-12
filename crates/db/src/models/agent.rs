use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, Sqlite, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ContextFile {
    pub pattern: String,
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub role: String,
    pub system_prompt: String,
    pub capabilities: Option<String>,   // JSON array
    pub tools: Option<String>,          // JSON array
    pub description: Option<String>,
    pub context_files: Option<String>,  // JSON array of ContextFile
    pub executor: String,               // Executor type: CLAUDE_CODE, GEMINI, etc.
    pub color: Option<String>,          // Hex color for visual identification
    pub start_command: Option<String>,  // Initial instruction when auto-starting in a column
    pub deliverable: Option<String>,    // What the agent should produce before handoff
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateAgent {
    pub name: String,
    pub role: String,
    pub system_prompt: String,
    pub capabilities: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub description: Option<String>,
    pub context_files: Option<Vec<ContextFile>>,
    pub executor: Option<String>,
    pub color: Option<String>,
    pub start_command: Option<String>,
    pub deliverable: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateAgent {
    pub name: Option<String>,
    pub role: Option<String>,
    pub system_prompt: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub tools: Option<Vec<String>>,
    pub description: Option<String>,
    pub context_files: Option<Vec<ContextFile>>,
    pub executor: Option<String>,
    pub color: Option<String>,
    pub start_command: Option<String>,
    pub deliverable: Option<String>,
}

impl Agent {
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Agent,
            r#"SELECT
                id as "id!: Uuid",
                name,
                role,
                system_prompt,
                capabilities,
                tools,
                description,
                context_files,
                executor,
                color,
                start_command,
                deliverable,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM agents
               ORDER BY created_at DESC"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Agent,
            r#"SELECT
                id as "id!: Uuid",
                name,
                role,
                system_prompt,
                capabilities,
                tools,
                description,
                context_files,
                executor,
                color,
                start_command,
                deliverable,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM agents
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        data: CreateAgent,
        agent_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let capabilities_json = data
            .capabilities
            .map(|caps| serde_json::to_string(&caps).ok())
            .flatten();
        let tools_json = data
            .tools
            .map(|tools| serde_json::to_string(&tools).ok())
            .flatten();
        let context_files_json = data
            .context_files
            .map(|files| serde_json::to_string(&files).ok())
            .flatten();
        let executor = data.executor.unwrap_or_else(|| "CLAUDE_CODE".to_string());

        sqlx::query_as!(
            Agent,
            r#"INSERT INTO agents (id, name, role, system_prompt, capabilities, tools, description, context_files, executor, color, start_command, deliverable)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
               RETURNING
                id as "id!: Uuid",
                name,
                role,
                system_prompt,
                capabilities,
                tools,
                description,
                context_files,
                executor,
                color,
                start_command,
                deliverable,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            agent_id,
            data.name,
            data.role,
            data.system_prompt,
            capabilities_json,
            tools_json,
            data.description,
            context_files_json,
            executor,
            data.color,
            data.start_command,
            data.deliverable
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateAgent,
    ) -> Result<Self, sqlx::Error> {
        // Get existing agent to preserve unchanged fields
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.unwrap_or(existing.name);
        let role = data.role.unwrap_or(existing.role);
        let system_prompt = data.system_prompt.unwrap_or(existing.system_prompt);
        let capabilities_json = data
            .capabilities
            .map(|caps| serde_json::to_string(&caps).ok())
            .flatten()
            .or(existing.capabilities);
        let tools_json = data
            .tools
            .map(|tools| serde_json::to_string(&tools).ok())
            .flatten()
            .or(existing.tools);
        let description = data.description.or(existing.description);
        let context_files_json = data
            .context_files
            .map(|files| serde_json::to_string(&files).ok())
            .flatten()
            .or(existing.context_files);
        let executor = data.executor.unwrap_or(existing.executor);
        let color = data.color.or(existing.color);
        let start_command = data.start_command.or(existing.start_command);
        let deliverable = data.deliverable.or(existing.deliverable);

        sqlx::query_as!(
            Agent,
            r#"UPDATE agents
               SET name = $2, role = $3, system_prompt = $4, capabilities = $5, tools = $6,
                   description = $7, context_files = $8, executor = $9, color = $10, start_command = $11,
                   deliverable = $12, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING
                id as "id!: Uuid",
                name,
                role,
                system_prompt,
                capabilities,
                tools,
                description,
                context_files,
                executor,
                color,
                start_command,
                deliverable,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            role,
            system_prompt,
            capabilities_json,
            tools_json,
            description,
            context_files_json,
            executor,
            color,
            start_command,
            deliverable
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete<'e, E>(executor: E, id: Uuid) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let result: sqlx::sqlite::SqliteQueryResult =
            sqlx::query!("DELETE FROM agents WHERE id = $1", id)
                .execute(executor)
                .await?;
        Ok(result.rows_affected())
    }
}

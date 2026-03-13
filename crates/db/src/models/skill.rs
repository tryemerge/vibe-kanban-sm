use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Skill {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateSkill {
    pub name: String,
    pub description: Option<String>,
    pub content: String,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateSkill {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
}

impl Skill {
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Skill,
            r#"SELECT
                id as "id!: Uuid",
                name,
                description,
                content,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM skills
               ORDER BY created_at ASC"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Skill,
            r#"SELECT
                id as "id!: Uuid",
                name,
                description,
                content,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM skills
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(pool: &PgPool, data: CreateSkill) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Skill,
            r#"INSERT INTO skills (name, description, content)
               VALUES ($1, $2, $3)
               RETURNING
                id as "id!: Uuid",
                name,
                description,
                content,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            data.name,
            data.description,
            data.content
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        data: UpdateSkill,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Skill,
            r#"UPDATE skills SET
                name        = COALESCE($2, name),
                description = CASE WHEN $3::TEXT IS NOT NULL THEN $3 ELSE description END,
                content     = COALESCE($4, content),
                updated_at  = NOW()
               WHERE id = $1
               RETURNING
                id as "id!: Uuid",
                name,
                description,
                content,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.name,
            data.description,
            data.content
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM skills WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Count how many agents this skill is assigned to.
    pub async fn agent_count(pool: &PgPool, id: Uuid) -> Result<i64, sqlx::Error> {
        let row = sqlx::query!(
            r#"SELECT COUNT(*) as "count!: i64" FROM agent_skills WHERE skill_id = $1"#,
            id
        )
        .fetch_one(pool)
        .await?;
        Ok(row.count)
    }

    /// Load all skills assigned to a given agent, ordered by name.
    pub async fn load_for_agent(pool: &PgPool, agent_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Skill,
            r#"SELECT
                s.id as "id!: Uuid",
                s.name,
                s.description,
                s.content,
                s.created_at as "created_at!: DateTime<Utc>",
                s.updated_at as "updated_at!: DateTime<Utc>"
               FROM skills s
               JOIN agent_skills ags ON ags.skill_id = s.id
               WHERE ags.agent_id = $1
               ORDER BY s.name ASC"#,
            agent_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn assign_to_agent(
        pool: &PgPool,
        agent_id: Uuid,
        skill_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO agent_skills (agent_id, skill_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            agent_id,
            skill_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn unassign_from_agent(
        pool: &PgPool,
        agent_id: Uuid,
        skill_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM agent_skills WHERE agent_id = $1 AND skill_id = $2",
            agent_id,
            skill_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Format assigned skills as a markdown section for injection into system prompts.
    /// Returns None if the slice is empty.
    pub fn build_skills_section(skills: &[Self]) -> Option<String> {
        if skills.is_empty() {
            return None;
        }
        let parts: Vec<String> = skills
            .iter()
            .map(|s| format!("### {}\n\n{}", s.name, s.content))
            .collect();
        Some(format!("## Skills\n\n{}", parts.join("\n\n---\n\n")))
    }
}

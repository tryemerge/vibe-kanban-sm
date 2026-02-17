use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use ts_rs::TS;
use uuid::Uuid;

/// A snapshot of a test/evaluate run for tracking improvements over time
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct EvaluateRun {
    pub id: Uuid,
    pub commit_hash: Option<String>,
    pub commit_message: Option<String>,
    pub project_name: String,
    #[ts(type = "string")]
    pub started_at: DateTime<Utc>,
    #[ts(type = "string")]
    pub completed_at: DateTime<Utc>,
    /// JSON blob: { tasks, artifacts, events, context_previews }
    #[ts(type = "EvaluateRunSummary")]
    pub summary: serde_json::Value,
    pub notes: Option<String>,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
}

/// Summary shape for TypeScript
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct EvaluateRunSummary {
    pub tasks: Vec<EvaluateRunTask>,
    pub artifacts: Vec<EvaluateRunArtifact>,
    pub events: Vec<EvaluateRunEvent>,
    pub stats: EvaluateRunStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct EvaluateRunTask {
    pub title: String,
    pub status: String,
    pub agent_status: Option<String>,
    pub attempts: Vec<EvaluateRunAttempt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct EvaluateRunAttempt {
    pub branch: String,
    pub completion_summary: Option<String>,
    pub final_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct EvaluateRunArtifact {
    pub artifact_type: String,
    pub scope: String,
    pub title: String,
    pub token_estimate: i32,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct EvaluateRunEvent {
    pub event_type: String,
    pub column_name: Option<String>,
    pub commit_message: Option<String>,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct EvaluateRunStats {
    pub total_tasks: i32,
    pub tasks_completed: i32,
    pub total_artifacts: i32,
    pub total_tokens: i32,
    pub total_events: i32,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateEvaluateRun {
    pub commit_hash: Option<String>,
    pub commit_message: Option<String>,
    pub project_name: String,
    #[ts(type = "string")]
    pub started_at: DateTime<Utc>,
    pub summary: serde_json::Value,
    pub notes: Option<String>,
}

impl EvaluateRun {
    pub async fn create(pool: &PgPool, data: &CreateEvaluateRun) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            EvaluateRun,
            r#"INSERT INTO evaluate_runs (commit_hash, commit_message, project_name, started_at, summary, notes)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id as "id!: Uuid",
                         commit_hash,
                         commit_message,
                         project_name,
                         started_at as "started_at!: DateTime<Utc>",
                         completed_at as "completed_at!: DateTime<Utc>",
                         summary as "summary!: serde_json::Value",
                         notes,
                         created_at as "created_at!: DateTime<Utc>"
            "#,
            data.commit_hash,
            data.commit_message,
            data.project_name,
            data.started_at,
            data.summary,
            data.notes,
        )
        .fetch_one(pool)
        .await
    }

    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            EvaluateRun,
            r#"SELECT id as "id!: Uuid",
                      commit_hash,
                      commit_message,
                      project_name,
                      started_at as "started_at!: DateTime<Utc>",
                      completed_at as "completed_at!: DateTime<Utc>",
                      summary as "summary!: serde_json::Value",
                      notes,
                      created_at as "created_at!: DateTime<Utc>"
               FROM evaluate_runs
               ORDER BY completed_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            EvaluateRun,
            r#"SELECT id as "id!: Uuid",
                      commit_hash,
                      commit_message,
                      project_name,
                      started_at as "started_at!: DateTime<Utc>",
                      completed_at as "completed_at!: DateTime<Utc>",
                      summary as "summary!: serde_json::Value",
                      notes,
                      created_at as "created_at!: DateTime<Utc>"
               FROM evaluate_runs
               WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM evaluate_runs WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

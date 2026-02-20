use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GroupEvent {
    pub id: Uuid,
    pub task_group_id: Uuid,
    pub task_id: Option<Uuid>,
    pub event_type: String,
    pub actor_type: String,
    pub summary: String,
    pub payload: Option<String>, // JSON
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct CreateGroupEvent {
    pub task_group_id: Uuid,
    pub task_id: Option<Uuid>,
    pub event_type: String,
    pub actor_type: String,
    pub summary: String,
    pub payload: Option<String>,
}

impl GroupEvent {
    /// Create a new group event
    pub async fn create(
        pool: &PgPool,
        data: &CreateGroupEvent,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO group_events (
                id,
                task_group_id,
                task_id,
                event_type,
                actor_type,
                summary,
                payload,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id as "id!: Uuid",
                task_group_id as "task_group_id!: Uuid",
                task_id as "task_id: Uuid",
                event_type as "event_type!: String",
                actor_type as "actor_type!: String",
                summary as "summary!: String",
                payload as "payload: String",
                created_at as "created_at!: DateTime<Utc>"
            "#,
        )
        .bind(id)
        .bind(data.task_group_id)
        .bind(data.task_id)
        .bind(&data.event_type)
        .bind(&data.actor_type)
        .bind(&data.summary)
        .bind(&data.payload)
        .bind(now)
        .fetch_one(pool)
        .await
    }

    /// Find events by group ID, paginated, newest first
    pub async fn find_by_group(
        pool: &PgPool,
        group_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT
                id as "id!: Uuid",
                task_group_id as "task_group_id!: Uuid",
                task_id as "task_id: Uuid",
                event_type as "event_type!: String",
                actor_type as "actor_type!: String",
                summary as "summary!: String",
                payload as "payload: String",
                created_at as "created_at!: DateTime<Utc>"
            FROM group_events
            WHERE task_group_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(group_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }

    /// Find events by project ID (across all groups), paginated, newest first
    pub async fn find_by_project(
        pool: &PgPool,
        project_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT
                ge.id as "id!: Uuid",
                ge.task_group_id as "task_group_id!: Uuid",
                ge.task_id as "task_id: Uuid",
                ge.event_type as "event_type!: String",
                ge.actor_type as "actor_type!: String",
                ge.summary as "summary!: String",
                ge.payload as "payload: String",
                ge.created_at as "created_at!: DateTime<Utc>"
            FROM group_events ge
            INNER JOIN task_groups tg ON ge.task_group_id = tg.id
            WHERE tg.project_id = $1
            ORDER BY ge.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }

    /// Find events by type within a group
    pub async fn find_by_type(
        pool: &PgPool,
        group_id: Uuid,
        event_type: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            SELECT
                id as "id!: Uuid",
                task_group_id as "task_group_id!: Uuid",
                task_id as "task_id: Uuid",
                event_type as "event_type!: String",
                actor_type as "actor_type!: String",
                summary as "summary!: String",
                payload as "payload: String",
                created_at as "created_at!: DateTime<Utc>"
            FROM group_events
            WHERE task_group_id = $1 AND event_type = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(group_id)
        .bind(event_type)
        .fetch_all(pool)
        .await
    }
}

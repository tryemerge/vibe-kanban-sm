use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, Postgres, PgPool};
use ts_rs::TS;
use uuid::Uuid;

/// Type of context artifact
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    /// Memory about a specific module/file - what it does, patterns, decisions
    ModuleMemory,
    /// Architecture Decision Record
    Adr,
    /// A specific decision made during development
    Decision,
    /// A learned pattern or best practice
    Pattern,
    /// Dependency information
    Dependency,
}

/// Scope determines when an artifact is included in agent context
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactScope {
    /// Include when working on matching file paths (default behavior)
    #[default]
    Path,
    /// Include only for the specific task (uses source_task_id)
    Task,
    /// Always include for all agents in the project
    Global,
}

impl ArtifactScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactScope::Path => "path",
            ArtifactScope::Task => "task",
            ArtifactScope::Global => "global",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "path" => Some(ArtifactScope::Path),
            "task" => Some(ArtifactScope::Task),
            "global" => Some(ArtifactScope::Global),
            _ => None,
        }
    }
}

impl ArtifactType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactType::ModuleMemory => "module_memory",
            ArtifactType::Adr => "adr",
            ArtifactType::Decision => "decision",
            ArtifactType::Pattern => "pattern",
            ArtifactType::Dependency => "dependency",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "module_memory" => Some(ArtifactType::ModuleMemory),
            "adr" => Some(ArtifactType::Adr),
            "decision" => Some(ArtifactType::Decision),
            "pattern" => Some(ArtifactType::Pattern),
            "dependency" => Some(ArtifactType::Dependency),
            _ => None,
        }
    }
}

/// A context artifact stores learned knowledge from agent work
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ContextArtifact {
    pub id: Uuid,
    pub project_id: Uuid,
    pub artifact_type: String,
    pub path: Option<String>,
    pub title: String,
    pub content: String,
    pub metadata: Option<String>,
    pub source_task_id: Option<Uuid>,
    pub source_commit_hash: Option<String>,
    /// Scope determines when this artifact is included in context
    pub scope: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateContextArtifact {
    pub project_id: Uuid,
    pub artifact_type: ArtifactType,
    pub path: Option<String>,
    pub title: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub source_task_id: Option<Uuid>,
    pub source_commit_hash: Option<String>,
    #[serde(default)]
    pub scope: ArtifactScope,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateContextArtifact {
    pub title: Option<String>,
    pub content: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub scope: Option<ArtifactScope>,
}

impl ContextArtifact {
    /// Find all artifacts for a project
    pub async fn find_by_project(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ContextArtifact,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                artifact_type,
                path,
                title,
                content,
                metadata,
                source_task_id as "source_task_id: Uuid",
                source_commit_hash,
                scope,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM context_artifacts
               WHERE project_id = $1
               ORDER BY updated_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find artifacts by type for a project
    pub async fn find_by_project_and_type(
        pool: &PgPool,
        project_id: Uuid,
        artifact_type: &ArtifactType,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let type_str = artifact_type.as_str();
        sqlx::query_as!(
            ContextArtifact,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                artifact_type,
                path,
                title,
                content,
                metadata,
                source_task_id as "source_task_id: Uuid",
                source_commit_hash,
                scope,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM context_artifacts
               WHERE project_id = $1 AND artifact_type = $2
               ORDER BY updated_at DESC"#,
            project_id,
            type_str
        )
        .fetch_all(pool)
        .await
    }

    /// Find module memory for a specific path
    pub async fn find_module_memory(
        pool: &PgPool,
        project_id: Uuid,
        path: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ContextArtifact,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                artifact_type,
                path,
                title,
                content,
                metadata,
                source_task_id as "source_task_id: Uuid",
                source_commit_hash,
                scope,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM context_artifacts
               WHERE project_id = $1
                 AND artifact_type = 'module_memory'
                 AND path = $2"#,
            project_id,
            path
        )
        .fetch_optional(pool)
        .await
    }

    /// Find artifact by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ContextArtifact,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                artifact_type,
                path,
                title,
                content,
                metadata,
                source_task_id as "source_task_id: Uuid",
                source_commit_hash,
                scope,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM context_artifacts
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new artifact
    pub async fn create(
        pool: &PgPool,
        data: CreateContextArtifact,
        artifact_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let type_str = data.artifact_type.as_str();
        let scope_str = data.scope.as_str();
        let metadata_json = data
            .metadata
            .map(|m| serde_json::to_string(&m).ok())
            .flatten();

        sqlx::query_as!(
            ContextArtifact,
            r#"INSERT INTO context_artifacts
               (id, project_id, artifact_type, path, title, content, metadata, source_task_id, source_commit_hash, scope)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                artifact_type,
                path,
                title,
                content,
                metadata,
                source_task_id as "source_task_id: Uuid",
                source_commit_hash,
                scope,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            artifact_id,
            data.project_id,
            type_str,
            data.path,
            data.title,
            data.content,
            metadata_json,
            data.source_task_id,
            data.source_commit_hash,
            scope_str
        )
        .fetch_one(pool)
        .await
    }

    /// Update an artifact
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        data: UpdateContextArtifact,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let title = data.title.unwrap_or(existing.title);
        let content = data.content.unwrap_or(existing.content);
        let metadata_json = data
            .metadata
            .map(|m| serde_json::to_string(&m).ok())
            .flatten()
            .or(existing.metadata);
        let scope_str = data
            .scope
            .map(|s| s.as_str().to_string())
            .unwrap_or(existing.scope);

        sqlx::query_as!(
            ContextArtifact,
            r#"UPDATE context_artifacts
               SET title = $2, content = $3, metadata = $4, scope = $5,
                   updated_at = NOW()
               WHERE id = $1
               RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                artifact_type,
                path,
                title,
                content,
                metadata,
                source_task_id as "source_task_id: Uuid",
                source_commit_hash,
                scope,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            title,
            content,
            metadata_json,
            scope_str
        )
        .fetch_one(pool)
        .await
    }

    /// Upsert a module memory - update if exists for path, create if not
    pub async fn upsert_module_memory(
        pool: &PgPool,
        project_id: Uuid,
        path: &str,
        title: &str,
        content: &str,
        source_task_id: Option<Uuid>,
        source_commit_hash: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        // Check if module memory already exists for this path
        if let Some(existing) = Self::find_module_memory(pool, project_id, path).await? {
            // Update existing
            Self::update(
                pool,
                existing.id,
                UpdateContextArtifact {
                    title: Some(title.to_string()),
                    content: Some(content.to_string()),
                    metadata: None,
                    scope: None, // Preserve existing scope
                },
            )
            .await
        } else {
            // Create new
            Self::create(
                pool,
                CreateContextArtifact {
                    project_id,
                    artifact_type: ArtifactType::ModuleMemory,
                    path: Some(path.to_string()),
                    title: title.to_string(),
                    content: content.to_string(),
                    metadata: None,
                    source_task_id,
                    source_commit_hash: source_commit_hash.map(|s| s.to_string()),
                    scope: ArtifactScope::Path, // Module memory uses path scope by default
                },
                Uuid::new_v4(),
            )
            .await
        }
    }

    /// Delete an artifact
    pub async fn delete<'e, E>(executor: E, id: Uuid) -> Result<u64, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result: sqlx::postgres::PgQueryResult =
            sqlx::query!("DELETE FROM context_artifacts WHERE id = $1", id)
                .execute(executor)
                .await?;
        Ok(result.rows_affected())
    }

    /// Build context string from relevant artifacts for agent prompting
    /// Includes path-based artifacts for the given paths
    pub async fn build_context_for_paths(
        pool: &PgPool,
        project_id: Uuid,
        paths: &[String],
    ) -> Result<String, sqlx::Error> {
        let mut context = String::new();

        for path in paths {
            if let Some(memory) = Self::find_module_memory(pool, project_id, path).await? {
                context.push_str(&format!("## Module: {}\n\n", path));
                context.push_str(&memory.content);
                context.push_str("\n\n");
            }
        }

        Ok(context)
    }

    /// Build full context for agent prompting including global, task, and path artifacts
    pub async fn build_full_context(
        pool: &PgPool,
        project_id: Uuid,
        task_id: Option<Uuid>,
        paths: &[String],
    ) -> Result<String, sqlx::Error> {
        let mut context_parts = Vec::new();

        // 1. Global artifacts (always included)
        let global_artifacts = Self::find_global_artifacts(pool, project_id).await?;
        if !global_artifacts.is_empty() {
            let mut global_section = String::from("# Project Context\n\n");
            for artifact in global_artifacts {
                global_section.push_str(&format!("## {}\n\n", artifact.title));
                global_section.push_str(&artifact.content);
                global_section.push_str("\n\n");
            }
            context_parts.push(global_section);
        }

        // 2. Task-specific artifacts (if task_id provided)
        if let Some(tid) = task_id {
            let task_artifacts = Self::find_task_artifacts(pool, project_id, tid).await?;
            if !task_artifacts.is_empty() {
                let mut task_section = String::from("# Task Context\n\n");
                for artifact in task_artifacts {
                    task_section.push_str(&format!("## {}\n\n", artifact.title));
                    task_section.push_str(&artifact.content);
                    task_section.push_str("\n\n");
                }
                context_parts.push(task_section);
            }
        }

        // 3. Path-based artifacts (module memories for relevant files)
        let path_context = Self::build_context_for_paths(pool, project_id, paths).await?;
        if !path_context.is_empty() {
            context_parts.push(format!("# Module Context\n\n{}", path_context));
        }

        Ok(context_parts.join("\n---\n\n"))
    }

    /// Get recent ADRs for a project
    pub async fn get_recent_adrs(
        pool: &PgPool,
        project_id: Uuid,
        limit: i32,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let limit_i64 = limit as i64;
        sqlx::query_as!(
            ContextArtifact,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                artifact_type,
                path,
                title,
                content,
                metadata,
                source_task_id as "source_task_id: Uuid",
                source_commit_hash,
                scope,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM context_artifacts
               WHERE project_id = $1 AND artifact_type = 'adr'
               ORDER BY created_at DESC
               LIMIT $2"#,
            project_id,
            limit_i64
        )
        .fetch_all(pool)
        .await
    }

    /// Find all global-scoped artifacts for a project
    pub async fn find_global_artifacts(
        pool: &PgPool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ContextArtifact,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                artifact_type,
                path,
                title,
                content,
                metadata,
                source_task_id as "source_task_id: Uuid",
                source_commit_hash,
                scope,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM context_artifacts
               WHERE project_id = $1 AND scope = 'global'
               ORDER BY updated_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find task-scoped artifacts for a specific task
    pub async fn find_task_artifacts(
        pool: &PgPool,
        project_id: Uuid,
        task_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ContextArtifact,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                artifact_type,
                path,
                title,
                content,
                metadata,
                source_task_id as "source_task_id: Uuid",
                source_commit_hash,
                scope,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
               FROM context_artifacts
               WHERE project_id = $1 AND scope = 'task' AND source_task_id = $2
               ORDER BY updated_at DESC"#,
            project_id,
            task_id
        )
        .fetch_all(pool)
        .await
    }
}

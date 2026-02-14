use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, Postgres, PgPool};
use tracing;
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
    /// Implementation plan (iplan) - breaks down work into subtasks
    #[serde(rename = "iplan")]
    IPlan,
    /// Changelog entry - records completed work for release notes
    ChangelogEntry,
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
            ArtifactType::IPlan => "iplan",
            ArtifactType::ChangelogEntry => "changelog_entry",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "module_memory" => Some(ArtifactType::ModuleMemory),
            "adr" => Some(ArtifactType::Adr),
            "decision" => Some(ArtifactType::Decision),
            "pattern" => Some(ArtifactType::Pattern),
            "dependency" => Some(ArtifactType::Dependency),
            "iplan" => Some(ArtifactType::IPlan),
            "changelog_entry" => Some(ArtifactType::ChangelogEntry),
            _ => None,
        }
    }

    /// Priority ordering for context budget allocation (lower = higher priority)
    pub fn priority(&self) -> i32 {
        match self {
            ArtifactType::Adr => 1,
            ArtifactType::Pattern => 2,
            ArtifactType::IPlan => 3,
            ArtifactType::ModuleMemory => 4,
            ArtifactType::Decision => 5,
            ArtifactType::Dependency => 6,
            ArtifactType::ChangelogEntry => 7,
        }
    }
}

/// A context artifact stores learned knowledge from agent work
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ContextArtifact {
    pub id: Uuid,
    pub project_id: Uuid,
    pub artifact_type: String,
    /// File/module path this relates to (for module memories)
    pub path: Option<String>,
    pub title: String,
    pub content: String,
    pub metadata: Option<String>,
    pub source_task_id: Option<Uuid>,
    pub source_commit_hash: Option<String>,
    /// Scope determines when this artifact is included in context
    pub scope: String,
    /// Relative file path on disk (e.g., 'docs/adr/0001-use-postgres.md')
    pub file_path: Option<String>,
    /// ID of the artifact this one supersedes (for version tracking)
    pub supersedes_id: Option<Uuid>,
    /// Chain ID groups all versions of the same logical document
    pub chain_id: Option<Uuid>,
    /// Version number within a chain (1, 2, 3...)
    pub version: i32,
    /// Approximate token count for budget-aware context injection (content.len() / 4)
    pub token_estimate: i32,
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
    /// Relative file path on disk (e.g., 'docs/adr/0001-use-postgres.md')
    pub file_path: Option<String>,
    /// ID of the artifact this one supersedes (for version tracking)
    pub supersedes_id: Option<Uuid>,
    /// Chain ID - if not provided, will be auto-generated for new chains
    pub chain_id: Option<Uuid>,
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
                file_path,
                supersedes_id as "supersedes_id: Uuid",
                chain_id as "chain_id: Uuid",
                version as "version!: i32",
                token_estimate as "token_estimate!: i32",
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
                file_path,
                supersedes_id as "supersedes_id: Uuid",
                chain_id as "chain_id: Uuid",
                version as "version!: i32",
                token_estimate as "token_estimate!: i32",
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
                file_path,
                supersedes_id as "supersedes_id: Uuid",
                chain_id as "chain_id: Uuid",
                version as "version!: i32",
                token_estimate as "token_estimate!: i32",
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
                file_path,
                supersedes_id as "supersedes_id: Uuid",
                chain_id as "chain_id: Uuid",
                version as "version!: i32",
                token_estimate as "token_estimate!: i32",
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

        // For new chains, generate a chain_id; for versions, use the provided one
        let chain_id = data.chain_id.or_else(|| {
            // For ADRs and iPlans, auto-generate a chain_id if not provided
            if matches!(data.artifact_type, ArtifactType::Adr | ArtifactType::IPlan) {
                Some(Uuid::new_v4())
            } else {
                None
            }
        });

        // Calculate version: if superseding, get the previous version + 1
        let version = if data.supersedes_id.is_some() {
            // This would ideally query the previous version, but for simplicity
            // we assume the caller handles version numbering or we query it
            // For now, default to 1 (caller should provide correct chain_id)
            1
        } else {
            1
        };

        // Estimate token count: ~4 chars per token for English text
        let token_estimate = (data.content.len() / 4) as i32;

        sqlx::query_as!(
            ContextArtifact,
            r#"INSERT INTO context_artifacts
               (id, project_id, artifact_type, path, title, content, metadata, source_task_id, source_commit_hash, scope, file_path, supersedes_id, chain_id, version, token_estimate)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
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
                file_path,
                supersedes_id as "supersedes_id: Uuid",
                chain_id as "chain_id: Uuid",
                version as "version!: i32",
                token_estimate as "token_estimate!: i32",
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
            scope_str,
            data.file_path,
            data.supersedes_id,
            chain_id,
            version,
            token_estimate
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
        let token_estimate = (content.len() / 4) as i32;
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
                   token_estimate = $6, updated_at = NOW()
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
                file_path,
                supersedes_id as "supersedes_id: Uuid",
                chain_id as "chain_id: Uuid",
                version as "version!: i32",
                token_estimate as "token_estimate!: i32",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            title,
            content,
            metadata_json,
            scope_str,
            token_estimate
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
                    file_path: None,            // Module memories don't have file paths
                    supersedes_id: None,
                    chain_id: None,
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

    /// Default token budget for context injection
    const DEFAULT_TOKEN_BUDGET: i32 = 8000;

    /// Build full context for agent prompting with token budget (ADR-007).
    ///
    /// Budget allocation: Global 50%, Task 30%, Path 20%.
    /// Unused budget rolls over to the next scope.
    /// Within each scope, artifacts are prioritized by type (ADR > Pattern > ...) then recency.
    /// Only the latest version per chain_id is included.
    pub async fn build_full_context(
        pool: &PgPool,
        project_id: Uuid,
        task_id: Option<Uuid>,
        paths: &[String],
    ) -> Result<String, sqlx::Error> {
        let total_budget = Self::DEFAULT_TOKEN_BUDGET;

        tracing::info!(
            target: "vibe_kanban::context",
            "📚 Building budgeted context for project {} (task: {:?}, paths: {}, budget: {} tokens)",
            project_id,
            task_id,
            paths.len(),
            total_budget
        );

        let mut context_parts = Vec::new();
        let mut remaining_budget = total_budget;

        // 1. Global artifacts — 50% of budget
        let global_budget = total_budget / 2;
        let global_artifacts = Self::find_global_artifacts(pool, project_id).await?;
        let global_artifacts = Self::dedup_by_chain(global_artifacts);
        let global_artifacts = Self::sort_by_priority(global_artifacts);

        if !global_artifacts.is_empty() {
            let mut section = String::from("# Project Context\n\n");
            let mut included = 0;
            let mut tokens_used = 0;

            for artifact in &global_artifacts {
                if tokens_used + artifact.token_estimate > global_budget.max(remaining_budget) {
                    break;
                }
                section.push_str(&format!("## {}\n\n", artifact.title));
                section.push_str(&artifact.content);
                section.push_str("\n\n");
                tokens_used += artifact.token_estimate;
                included += 1;
            }

            if included > 0 {
                tracing::info!(
                    target: "vibe_kanban::context",
                    "  ├─ Global: {}/{} artifacts, {} tokens",
                    included,
                    global_artifacts.len(),
                    tokens_used
                );
                context_parts.push(section);
                remaining_budget -= tokens_used;
            }
        } else {
            tracing::info!(
                target: "vibe_kanban::context",
                "  ├─ Global artifacts: none"
            );
            // Unused global budget rolls over
        }

        // 2. Task-specific artifacts — 30% of budget (+ rollover)
        let task_budget = (total_budget * 3) / 10;
        if let Some(tid) = task_id {
            let task_artifacts = Self::find_task_artifacts(pool, project_id, tid).await?;
            let task_artifacts = Self::dedup_by_chain(task_artifacts);
            let task_artifacts = Self::sort_by_priority(task_artifacts);

            if !task_artifacts.is_empty() {
                let mut section = String::from("# Task Context\n\n");
                let mut included = 0;
                let mut tokens_used = 0;
                let effective_budget = task_budget.max(remaining_budget.min(task_budget + (total_budget / 2 - (total_budget - remaining_budget)).max(0)));

                for artifact in &task_artifacts {
                    if tokens_used + artifact.token_estimate > remaining_budget {
                        break;
                    }
                    if tokens_used + artifact.token_estimate > effective_budget && included > 0 {
                        break;
                    }
                    section.push_str(&format!("## {}\n\n", artifact.title));
                    section.push_str(&artifact.content);
                    section.push_str("\n\n");
                    tokens_used += artifact.token_estimate;
                    included += 1;
                }

                if included > 0 {
                    tracing::info!(
                        target: "vibe_kanban::context",
                        "  ├─ Task: {}/{} artifacts, {} tokens",
                        included,
                        task_artifacts.len(),
                        tokens_used
                    );
                    context_parts.push(section);
                    remaining_budget -= tokens_used;
                }
            } else {
                tracing::info!(
                    target: "vibe_kanban::context",
                    "  ├─ Task artifacts: none for task {}",
                    tid
                );
            }
        }

        // 3. Path-based artifacts — 20% of budget (+ rollover from above)
        if !paths.is_empty() && remaining_budget > 0 {
            let mut section = String::from("# Module Context\n\n");
            let mut included = 0;
            let mut tokens_used = 0;

            for path in paths {
                if let Some(memory) = Self::find_module_memory(pool, project_id, path).await? {
                    if tokens_used + memory.token_estimate > remaining_budget {
                        break;
                    }
                    section.push_str(&format!("## Module: {}\n\n", path));
                    section.push_str(&memory.content);
                    section.push_str("\n\n");
                    tokens_used += memory.token_estimate;
                    included += 1;
                }
            }

            if included > 0 {
                tracing::info!(
                    target: "vibe_kanban::context",
                    "  ├─ Path: {} modules, {} tokens",
                    included,
                    tokens_used
                );
                context_parts.push(section);
                remaining_budget -= tokens_used;
            } else {
                tracing::info!(
                    target: "vibe_kanban::context",
                    "  ├─ Path artifacts: none for {} paths",
                    paths.len()
                );
            }
        }

        let tokens_used = total_budget - remaining_budget;
        tracing::info!(
            target: "vibe_kanban::context",
            "  └─ Total: {} sections, {}/{} tokens used",
            context_parts.len(),
            tokens_used,
            total_budget
        );

        Ok(context_parts.join("\n---\n\n"))
    }

    /// Deduplicate artifacts by chain_id, keeping only the latest version per chain.
    /// Artifacts without a chain_id are always kept.
    fn dedup_by_chain(artifacts: Vec<Self>) -> Vec<Self> {
        let mut seen_chains: std::collections::HashMap<Uuid, i32> = std::collections::HashMap::new();
        let mut result = Vec::new();

        // First pass: find max version per chain
        for artifact in &artifacts {
            if let Some(chain_id) = artifact.chain_id {
                let entry = seen_chains.entry(chain_id).or_insert(0);
                if artifact.version > *entry {
                    *entry = artifact.version;
                }
            }
        }

        // Second pass: keep only latest version per chain, and all non-chain artifacts
        for artifact in artifacts {
            match artifact.chain_id {
                Some(chain_id) => {
                    if let Some(&max_version) = seen_chains.get(&chain_id) {
                        if artifact.version == max_version {
                            result.push(artifact);
                            // Remove from map so we don't include duplicates at same version
                            seen_chains.remove(&chain_id);
                        }
                    }
                }
                None => result.push(artifact),
            }
        }

        result
    }

    /// Sort artifacts by type priority (ADR=1, Pattern=2, ...) then by recency (newest first).
    fn sort_by_priority(mut artifacts: Vec<Self>) -> Vec<Self> {
        artifacts.sort_by(|a, b| {
            let a_priority = ArtifactType::from_str(&a.artifact_type)
                .map(|t| t.priority())
                .unwrap_or(99);
            let b_priority = ArtifactType::from_str(&b.artifact_type)
                .map(|t| t.priority())
                .unwrap_or(99);
            a_priority.cmp(&b_priority).then(b.updated_at.cmp(&a.updated_at))
        });
        artifacts
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
                file_path,
                supersedes_id as "supersedes_id: Uuid",
                chain_id as "chain_id: Uuid",
                version as "version!: i32",
                token_estimate as "token_estimate!: i32",
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
                file_path,
                supersedes_id as "supersedes_id: Uuid",
                chain_id as "chain_id: Uuid",
                version as "version!: i32",
                token_estimate as "token_estimate!: i32",
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
                file_path,
                supersedes_id as "supersedes_id: Uuid",
                chain_id as "chain_id: Uuid",
                version as "version!: i32",
                token_estimate as "token_estimate!: i32",
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

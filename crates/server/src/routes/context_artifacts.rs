use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::context_artifact::{
    ArtifactType, ContextArtifact, ContextPreviewStats, CreateContextArtifact, UpdateContextArtifact,
};
use deployment::Deployment;
use serde::Deserialize;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::load_context_artifact_middleware};

#[derive(Deserialize, TS)]
pub struct ContextArtifactQuery {
    pub project_id: Uuid,
    #[serde(default)]
    pub artifact_type: Option<String>,
}

/// Get all context artifacts for a project, optionally filtered by type
pub async fn get_context_artifacts(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<ContextArtifactQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<ContextArtifact>>>, ApiError> {
    let artifacts = if let Some(type_str) = params.artifact_type {
        if let Some(artifact_type) = ArtifactType::from_str(&type_str) {
            ContextArtifact::find_by_project_and_type(
                &deployment.db().pool,
                params.project_id,
                &artifact_type,
            )
            .await?
        } else {
            return Err(ApiError::BadRequest(format!(
                "Invalid artifact type: {}",
                type_str
            )));
        }
    } else {
        ContextArtifact::find_by_project(&deployment.db().pool, params.project_id).await?
    };

    Ok(ResponseJson(ApiResponse::success(artifacts)))
}

/// Get a single context artifact by ID
pub async fn get_context_artifact(
    Extension(artifact): Extension<ContextArtifact>,
) -> Result<ResponseJson<ApiResponse<ContextArtifact>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(artifact)))
}

/// Create a new context artifact
pub async fn create_context_artifact(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateContextArtifact>,
) -> Result<ResponseJson<ApiResponse<ContextArtifact>>, ApiError> {
    let artifact_id = Uuid::new_v4();
    let artifact =
        ContextArtifact::create(&deployment.db().pool, payload, artifact_id).await?;

    deployment
        .track_if_analytics_allowed(
            "context_artifact_created",
            serde_json::json!({
                "artifact_id": artifact.id.to_string(),
                "artifact_type": artifact.artifact_type,
                "project_id": artifact.project_id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(artifact)))
}

/// Update a context artifact
pub async fn update_context_artifact(
    Extension(artifact): Extension<ContextArtifact>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateContextArtifact>,
) -> Result<ResponseJson<ApiResponse<ContextArtifact>>, ApiError> {
    let updated = ContextArtifact::update(&deployment.db().pool, artifact.id, payload).await?;

    deployment
        .track_if_analytics_allowed(
            "context_artifact_updated",
            serde_json::json!({
                "artifact_id": updated.id.to_string(),
                "artifact_type": updated.artifact_type,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(updated)))
}

/// Delete a context artifact
pub async fn delete_context_artifact(
    Extension(artifact): Extension<ContextArtifact>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let rows_affected = ContextArtifact::delete(&deployment.db().pool, artifact.id).await?;
    if rows_affected == 0 {
        Err(ApiError::Database(sqlx::Error::RowNotFound))
    } else {
        Ok(ResponseJson(ApiResponse::success(())))
    }
}

#[derive(Deserialize, TS)]
pub struct ModuleMemoryQuery {
    pub project_id: Uuid,
    pub path: String,
}

/// Get module memory for a specific path
pub async fn get_module_memory(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<ModuleMemoryQuery>,
) -> Result<ResponseJson<ApiResponse<Option<ContextArtifact>>>, ApiError> {
    let artifact = ContextArtifact::find_module_memory(
        &deployment.db().pool,
        params.project_id,
        &params.path,
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(artifact)))
}

#[derive(Deserialize, TS)]
pub struct UpsertModuleMemoryRequest {
    pub project_id: Uuid,
    pub path: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub source_task_id: Option<Uuid>,
    #[serde(default)]
    pub source_commit_hash: Option<String>,
}

/// Upsert module memory - creates or updates memory for a specific path
pub async fn upsert_module_memory(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpsertModuleMemoryRequest>,
) -> Result<ResponseJson<ApiResponse<ContextArtifact>>, ApiError> {
    let artifact = ContextArtifact::upsert_module_memory(
        &deployment.db().pool,
        payload.project_id,
        &payload.path,
        &payload.title,
        &payload.content,
        payload.source_task_id,
        payload.source_commit_hash.as_deref(),
    )
    .await?;

    deployment
        .track_if_analytics_allowed(
            "module_memory_upserted",
            serde_json::json!({
                "artifact_id": artifact.id.to_string(),
                "path": artifact.path,
                "project_id": artifact.project_id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(artifact)))
}

#[derive(Deserialize, TS)]
pub struct BuildContextQuery {
    pub project_id: Uuid,
    /// Comma-separated list of paths
    pub paths: String,
}

/// Build context string from relevant artifacts for the given paths
pub async fn build_context(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<BuildContextQuery>,
) -> Result<ResponseJson<ApiResponse<String>>, ApiError> {
    let paths: Vec<String> = params.paths.split(',').map(|s| s.trim().to_string()).collect();

    let context =
        ContextArtifact::build_context_for_paths(&deployment.db().pool, params.project_id, &paths)
            .await?;

    Ok(ResponseJson(ApiResponse::success(context)))
}

#[derive(Deserialize, TS)]
pub struct RecentAdrsQuery {
    pub project_id: Uuid,
    #[serde(default = "default_adr_limit")]
    pub limit: i32,
}

fn default_adr_limit() -> i32 {
    10
}

/// Get recent ADRs for a project
pub async fn get_recent_adrs(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<RecentAdrsQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<ContextArtifact>>>, ApiError> {
    let adrs = ContextArtifact::get_recent_adrs(
        &deployment.db().pool,
        params.project_id,
        params.limit,
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(adrs)))
}

#[derive(Deserialize, TS)]
pub struct PreviewContextQuery {
    pub project_id: Uuid,
    #[serde(default)]
    pub task_id: Option<Uuid>,
}

/// Preview the assembled context that an agent would receive for a task.
/// Returns the context string alongside budget usage stats.
pub async fn preview_context(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<PreviewContextQuery>,
) -> Result<ResponseJson<ApiResponse<ContextPreviewStats>>, ApiError> {
    let stats = ContextArtifact::build_full_context_with_stats(
        &deployment.db().pool,
        params.project_id,
        params.task_id,
        &[],
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(stats)))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let artifact_router = Router::new()
        .route("/", get(get_context_artifact).put(update_context_artifact).delete(delete_context_artifact))
        .layer(from_fn_with_state(deployment.clone(), load_context_artifact_middleware));

    let inner = Router::new()
        .route("/", get(get_context_artifacts).post(create_context_artifact))
        .route("/module-memory", get(get_module_memory).post(upsert_module_memory))
        .route("/build-context", get(build_context))
        .route("/preview-context", get(preview_context))
        .route("/adrs", get(get_recent_adrs))
        .nest("/{artifact_id}", artifact_router);

    Router::new().nest("/context-artifacts", inner)
}

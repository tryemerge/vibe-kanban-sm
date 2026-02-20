use axum::{
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::get,
    Router,
};
use serde::Deserialize;
use uuid::Uuid;

use db::models::group_event::GroupEvent;
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{error::ApiError, DeploymentImpl};

/// Router for group event endpoints
pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let _ = deployment;
    Router::new()
        .route(
            "/task-groups/{group_id}/events",
            get(list_group_events),
        )
        .route(
            "/projects/{project_id}/group-events",
            get(list_project_events),
        )
}

#[derive(Debug, Deserialize)]
struct PaginationQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

/// List events for a specific task group (paginated, newest first)
async fn list_group_events(
    Path(group_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<GroupEvent>>>, ApiError> {
    let pool = &deployment.db().pool;

    let events = GroupEvent::find_by_group(
        pool,
        group_id,
        pagination.limit,
        pagination.offset,
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(events)))
}

/// List all group events for a project (orchestration feed, paginated, newest first)
async fn list_project_events(
    Path(project_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<GroupEvent>>>, ApiError> {
    let pool = &deployment.db().pool;

    let events = GroupEvent::find_by_project(
        pool,
        project_id,
        pagination.limit,
        pagination.offset,
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(events)))
}

use axum::{
    Extension, Json, Router,
    extract::State,
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::{
    kanban_column::{CreateKanbanColumn, KanbanColumn, UpdateKanbanColumn},
    project::Project,
};
use deployment::Deployment;
use serde::Deserialize;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    DeploymentImpl, error::ApiError,
    middleware::{load_kanban_column_middleware, load_project_middleware},
};

#[derive(Debug, Deserialize, TS)]
pub struct ReorderColumnsRequest {
    pub column_ids: Vec<Uuid>,
}

/// Get all columns for a project
pub async fn get_project_columns(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<KanbanColumn>>>, ApiError> {
    let columns = KanbanColumn::find_by_project(&deployment.db().pool, project.id).await?;
    Ok(ResponseJson(ApiResponse::success(columns)))
}

/// Create a new column for a project
pub async fn create_column(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateKanbanColumn>,
) -> Result<ResponseJson<ApiResponse<KanbanColumn>>, ApiError> {
    let column = KanbanColumn::create(&deployment.db().pool, project.id, &payload).await?;

    deployment
        .track_if_analytics_allowed(
            "kanban_column_created",
            serde_json::json!({
                "project_id": project.id.to_string(),
                "column_id": column.id.to_string(),
                "column_name": column.name,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(column)))
}

/// Get a single column
pub async fn get_column(
    Extension(column): Extension<KanbanColumn>,
) -> Result<ResponseJson<ApiResponse<KanbanColumn>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(column)))
}

/// Update a column
pub async fn update_column(
    Extension(column): Extension<KanbanColumn>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateKanbanColumn>,
) -> Result<ResponseJson<ApiResponse<KanbanColumn>>, ApiError> {
    let updated = KanbanColumn::update(&deployment.db().pool, column.id, &payload).await?;

    deployment
        .track_if_analytics_allowed(
            "kanban_column_updated",
            serde_json::json!({
                "column_id": column.id.to_string(),
                "column_name": updated.name,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(updated)))
}

/// Delete a column
pub async fn delete_column(
    Extension(column): Extension<KanbanColumn>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    // Check if any tasks are in this column
    let tasks_in_column = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!: i64" FROM tasks WHERE column_id = $1"#,
        column.id
    )
    .fetch_one(&deployment.db().pool)
    .await?;

    if tasks_in_column > 0 {
        return Err(ApiError::BadRequest(format!(
            "Cannot delete column '{}': {} task(s) are still in this column. Move them first.",
            column.name, tasks_in_column
        )));
    }

    // Delete associated transitions first
    sqlx::query!(
        "DELETE FROM state_transitions WHERE from_column_id = $1 OR to_column_id = $1",
        column.id
    )
    .execute(&deployment.db().pool)
    .await?;

    let rows_affected = KanbanColumn::delete(&deployment.db().pool, column.id).await?;
    if rows_affected == 0 {
        Err(ApiError::Database(sqlx::Error::RowNotFound))
    } else {
        deployment
            .track_if_analytics_allowed(
                "kanban_column_deleted",
                serde_json::json!({
                    "column_id": column.id.to_string(),
                }),
            )
            .await;

        Ok(ResponseJson(ApiResponse::success(())))
    }
}

/// Reorder columns
pub async fn reorder_columns(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<ReorderColumnsRequest>,
) -> Result<ResponseJson<ApiResponse<Vec<KanbanColumn>>>, ApiError> {
    KanbanColumn::reorder(&deployment.db().pool, project.id, &payload.column_ids).await?;
    let columns = KanbanColumn::find_by_project(&deployment.db().pool, project.id).await?;

    deployment
        .track_if_analytics_allowed(
            "kanban_columns_reordered",
            serde_json::json!({
                "project_id": project.id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(columns)))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Routes for a specific column (requires column_id)
    let column_router = Router::new()
        .route("/", get(get_column).put(update_column).delete(delete_column))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_kanban_column_middleware,
        ));

    // Routes under /projects/:project_id/columns
    let project_columns_router = Router::new()
        .route("/", get(get_project_columns).post(create_column))
        .route("/reorder", post(reorder_columns))
        .nest("/{column_id}", column_router)
        .layer(from_fn_with_state(
            deployment.clone(),
            load_project_middleware,
        ));

    Router::new().nest("/projects/{project_id}/columns", project_columns_router)
}

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::board::{Board, CreateBoard, UpdateBoard};
use db::models::kanban_column::{CreateKanbanColumn, KanbanColumn, UpdateKanbanColumn};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::load_board_middleware};

/// Get all boards
pub async fn list_boards(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<Board>>>, ApiError> {
    let boards = Board::find_all(&deployment.db().pool).await?;
    Ok(ResponseJson(ApiResponse::success(boards)))
}

/// Create a new board
pub async fn create_board(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateBoard>,
) -> Result<ResponseJson<ApiResponse<Board>>, ApiError> {
    let board = Board::create(&deployment.db().pool, &payload).await?;

    deployment
        .track_if_analytics_allowed(
            "board_created",
            serde_json::json!({
                "board_id": board.id.to_string(),
                "board_name": board.name,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(board)))
}

/// Get a single board
pub async fn get_board(
    Extension(board): Extension<Board>,
) -> Result<ResponseJson<ApiResponse<Board>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(board)))
}

/// Update a board
pub async fn update_board(
    Extension(board): Extension<Board>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateBoard>,
) -> Result<ResponseJson<ApiResponse<Board>>, ApiError> {
    let updated = Board::update(&deployment.db().pool, board.id, &payload).await?;

    deployment
        .track_if_analytics_allowed(
            "board_updated",
            serde_json::json!({
                "board_id": board.id.to_string(),
                "board_name": updated.name,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(updated)))
}

/// Delete a board
pub async fn delete_board(
    Extension(board): Extension<Board>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    // Columns and transitions cascade via ON DELETE CASCADE in the DB schema
    let rows_affected = Board::delete(&deployment.db().pool, board.id).await?;
    if rows_affected == 0 {
        Err(ApiError::Database(sqlx::Error::RowNotFound))
    } else {
        deployment
            .track_if_analytics_allowed(
                "board_deleted",
                serde_json::json!({
                    "board_id": board.id.to_string(),
                }),
            )
            .await;

        Ok(ResponseJson(ApiResponse::success(())))
    }
}

// ============================================================================
// Column management for boards
// ============================================================================

/// List all columns for a board
pub async fn list_board_columns(
    Extension(board): Extension<Board>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<KanbanColumn>>>, ApiError> {
    let columns = KanbanColumn::find_by_board(&deployment.db().pool, board.id).await?;
    Ok(ResponseJson(ApiResponse::success(columns)))
}

/// Create a column for a board
pub async fn create_board_column(
    Extension(board): Extension<Board>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateKanbanColumn>,
) -> Result<ResponseJson<ApiResponse<KanbanColumn>>, ApiError> {
    let column = KanbanColumn::create_for_board(
        &deployment.db().pool,
        board.id,
        &payload,
    )
    .await?;

    deployment
        .track_if_analytics_allowed(
            "board_column_created",
            serde_json::json!({
                "board_id": board.id.to_string(),
                "column_id": column.id.to_string(),
                "column_name": column.name,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(column)))
}

#[derive(Deserialize)]
pub struct ColumnPath {
    pub board_id: Uuid,
    pub column_id: Uuid,
}

/// Update a board column
pub async fn update_board_column(
    Extension(board): Extension<Board>,
    State(deployment): State<DeploymentImpl>,
    Path(path): Path<ColumnPath>,
    Json(payload): Json<UpdateKanbanColumn>,
) -> Result<ResponseJson<ApiResponse<KanbanColumn>>, ApiError> {
    // Verify column belongs to this board
    let existing = KanbanColumn::find_by_id(&deployment.db().pool, path.column_id)
        .await?
        .ok_or(ApiError::BadRequest("Column not found".to_string()))?;

    if existing.board_id != board.id {
        return Err(ApiError::BadRequest("Column not found in this board".to_string()));
    }

    let column = KanbanColumn::update(&deployment.db().pool, path.column_id, &payload).await?;

    deployment
        .track_if_analytics_allowed(
            "board_column_updated",
            serde_json::json!({
                "board_id": board.id.to_string(),
                "column_id": column.id.to_string(),
                "column_name": column.name,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(column)))
}

/// Delete a board column
pub async fn delete_board_column(
    Extension(board): Extension<Board>,
    State(deployment): State<DeploymentImpl>,
    Path(path): Path<ColumnPath>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    // Verify column belongs to this board
    let existing = KanbanColumn::find_by_id(&deployment.db().pool, path.column_id)
        .await?
        .ok_or(ApiError::BadRequest("Column not found".to_string()))?;

    if existing.board_id != board.id {
        return Err(ApiError::BadRequest("Column not found in this board".to_string()));
    }

    let rows_affected = KanbanColumn::delete(&deployment.db().pool, path.column_id).await?;
    if rows_affected == 0 {
        Err(ApiError::Database(sqlx::Error::RowNotFound))
    } else {
        deployment
            .track_if_analytics_allowed(
                "board_column_deleted",
                serde_json::json!({
                    "board_id": board.id.to_string(),
                    "column_id": path.column_id.to_string(),
                }),
            )
            .await;

        Ok(ResponseJson(ApiResponse::success(())))
    }
}

#[derive(Deserialize)]
pub struct ReorderColumnsPayload {
    pub column_ids: Vec<Uuid>,
}

/// Reorder columns within a board
pub async fn reorder_board_columns(
    Extension(board): Extension<Board>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<ReorderColumnsPayload>,
) -> Result<ResponseJson<ApiResponse<Vec<KanbanColumn>>>, ApiError> {
    KanbanColumn::reorder_board(&deployment.db().pool, board.id, &payload.column_ids).await?;

    // Fetch updated columns
    let columns = KanbanColumn::find_by_board(&deployment.db().pool, board.id).await?;

    deployment
        .track_if_analytics_allowed(
            "board_columns_reordered",
            serde_json::json!({
                "board_id": board.id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(columns)))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Routes for a specific board (requires board_id)
    let board_router = Router::new()
        .route("/", get(get_board).put(update_board).delete(delete_board))
        // Column management routes
        .route(
            "/columns",
            get(list_board_columns).post(create_board_column),
        )
        .route("/columns/reorder", axum::routing::post(reorder_board_columns))
        .route(
            "/columns/{column_id}",
            axum::routing::put(update_board_column).delete(delete_board_column),
        )
        .route(
            "/save-as-template",
            axum::routing::post(super::workflow_templates::save_as_template),
        )
        .layer(from_fn_with_state(
            deployment.clone(),
            load_board_middleware,
        ));

    Router::new()
        .route("/boards", get(list_boards).post(create_board))
        .nest("/boards/{board_id}", board_router)
}

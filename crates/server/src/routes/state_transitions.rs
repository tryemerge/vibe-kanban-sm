use axum::{
    Extension, Json, Router,
    extract::State,
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::{
    board::Board,
    project::Project,
    state_transition::{CreateStateTransition, StateTransition, StateTransitionWithColumns, UpdateStateTransition},
};
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{
    DeploymentImpl, error::ApiError,
    middleware::{load_board_middleware, load_project_middleware, load_state_transition_middleware},
};

// ============================================================================
// Board-level transitions (default workflow for all projects using this board)
// ============================================================================

/// Get all transitions for a board (board-level defaults)
pub async fn get_board_transitions(
    Extension(board): Extension<Board>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<StateTransitionWithColumns>>>, ApiError> {
    let transitions = StateTransition::find_by_board_with_columns(&deployment.db().pool, board.id).await?;
    Ok(ResponseJson(ApiResponse::success(transitions)))
}

/// Create a board-level state transition
pub async fn create_board_transition(
    Extension(board): Extension<Board>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateStateTransition>,
) -> Result<ResponseJson<ApiResponse<StateTransition>>, ApiError> {
    let transition = StateTransition::create_for_board(&deployment.db().pool, board.id, &payload).await?;

    deployment
        .track_if_analytics_allowed(
            "state_transition_created",
            serde_json::json!({
                "scope": "board",
                "board_id": board.id.to_string(),
                "transition_id": transition.id.to_string(),
                "from_column_id": transition.from_column_id.to_string(),
                "to_column_id": transition.to_column_id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(transition)))
}

// ============================================================================
// Project-level transitions (override board defaults for specific project)
// ============================================================================

/// Get all transitions for a project (project-level overrides only)
pub async fn get_project_transitions(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<StateTransitionWithColumns>>>, ApiError> {
    let transitions = StateTransition::find_by_project_with_columns(&deployment.db().pool, project.id).await?;
    Ok(ResponseJson(ApiResponse::success(transitions)))
}

/// Create a project-level state transition
pub async fn create_project_transition(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateStateTransition>,
) -> Result<ResponseJson<ApiResponse<StateTransition>>, ApiError> {
    let transition = StateTransition::create_for_project(&deployment.db().pool, project.id, &payload).await?;

    deployment
        .track_if_analytics_allowed(
            "state_transition_created",
            serde_json::json!({
                "scope": "project",
                "project_id": project.id.to_string(),
                "transition_id": transition.id.to_string(),
                "from_column_id": transition.from_column_id.to_string(),
                "to_column_id": transition.to_column_id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(transition)))
}

// ============================================================================
// Single transition operations (scope-agnostic, identified by ID)
// ============================================================================

/// Get a single transition
pub async fn get_transition(
    Extension(transition): Extension<StateTransition>,
) -> Result<ResponseJson<ApiResponse<StateTransition>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(transition)))
}

/// Update a transition
pub async fn update_transition(
    Extension(transition): Extension<StateTransition>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateStateTransition>,
) -> Result<ResponseJson<ApiResponse<StateTransition>>, ApiError> {
    let updated = StateTransition::update(&deployment.db().pool, transition.id, &payload).await?;

    deployment
        .track_if_analytics_allowed(
            "state_transition_updated",
            serde_json::json!({
                "transition_id": transition.id.to_string(),
                "scope": match (transition.task_id, transition.project_id, transition.board_id) {
                    (Some(_), _, _) => "task",
                    (_, Some(_), _) => "project",
                    _ => "board",
                },
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(updated)))
}

/// Delete a transition
pub async fn delete_transition(
    Extension(transition): Extension<StateTransition>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let rows_affected = StateTransition::delete(&deployment.db().pool, transition.id).await?;
    if rows_affected == 0 {
        Err(ApiError::Database(sqlx::Error::RowNotFound))
    } else {
        deployment
            .track_if_analytics_allowed(
                "state_transition_deleted",
                serde_json::json!({
                    "transition_id": transition.id.to_string(),
                    "scope": match (transition.task_id, transition.project_id, transition.board_id) {
                        (Some(_), _, _) => "task",
                        (_, Some(_), _) => "project",
                        _ => "board",
                    },
                }),
            )
            .await;

        Ok(ResponseJson(ApiResponse::success(())))
    }
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Routes for a specific transition (requires transition_id)
    let transition_router = Router::new()
        .route("/", get(get_transition).put(update_transition).delete(delete_transition))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_state_transition_middleware,
        ));

    // Routes under /boards/:board_id/transitions (board-level defaults)
    let board_transitions_router = Router::new()
        .route("/", get(get_board_transitions).post(create_board_transition))
        .nest("/{transition_id}", transition_router.clone())
        .layer(from_fn_with_state(
            deployment.clone(),
            load_board_middleware,
        ));

    // Routes under /projects/:project_id/transitions (project-level overrides)
    let project_transitions_router = Router::new()
        .route("/", get(get_project_transitions).post(create_project_transition))
        .nest("/{transition_id}", transition_router)
        .layer(from_fn_with_state(
            deployment.clone(),
            load_project_middleware,
        ));

    Router::new()
        .nest("/boards/{board_id}/transitions", board_transitions_router)
        .nest("/projects/{project_id}/transitions", project_transitions_router)
}

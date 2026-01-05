use axum::{
    Extension, Json, Router,
    extract::State,
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::{
    project::Project,
    state_transition::{CreateStateTransition, StateTransition, StateTransitionWithColumns},
};
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{
    DeploymentImpl, error::ApiError,
    middleware::{load_project_middleware, load_state_transition_middleware},
};

/// Get all transitions for a project (with column names)
pub async fn get_project_transitions(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<StateTransitionWithColumns>>>, ApiError> {
    let transitions = StateTransition::find_by_project_with_columns(&deployment.db().pool, project.id).await?;
    Ok(ResponseJson(ApiResponse::success(transitions)))
}

/// Create a new state transition
pub async fn create_transition(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateStateTransition>,
) -> Result<ResponseJson<ApiResponse<StateTransition>>, ApiError> {
    let transition = StateTransition::create(&deployment.db().pool, project.id, &payload).await?;

    deployment
        .track_if_analytics_allowed(
            "state_transition_created",
            serde_json::json!({
                "project_id": project.id.to_string(),
                "transition_id": transition.id.to_string(),
                "from_column_id": transition.from_column_id.to_string(),
                "to_column_id": transition.to_column_id.to_string(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(transition)))
}

/// Get a single transition
pub async fn get_transition(
    Extension(transition): Extension<StateTransition>,
) -> Result<ResponseJson<ApiResponse<StateTransition>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(transition)))
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
                }),
            )
            .await;

        Ok(ResponseJson(ApiResponse::success(())))
    }
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Routes for a specific transition (requires transition_id)
    let transition_router = Router::new()
        .route("/", get(get_transition).delete(delete_transition))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_state_transition_middleware,
        ));

    // Routes under /projects/:project_id/transitions
    let project_transitions_router = Router::new()
        .route("/", get(get_project_transitions).post(create_transition))
        .nest("/{transition_id}", transition_router)
        .layer(from_fn_with_state(
            deployment.clone(),
            load_project_middleware,
        ));

    Router::new().nest("/projects/{project_id}/transitions", project_transitions_router)
}

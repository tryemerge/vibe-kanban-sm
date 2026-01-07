use axum::{
    Extension, Json, Router,
    extract::State,
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::{
    task::Task,
    task_event::{CreateTaskEvent, TaskEvent, TaskEventWithNames},
};
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError, middleware::load_task_middleware};

/// Get all events for a task (with column names resolved)
pub async fn get_task_events(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskEventWithNames>>>, ApiError> {
    let events = TaskEvent::find_by_task_id_with_names(&deployment.db().pool, task.id).await?;
    Ok(ResponseJson(ApiResponse::success(events)))
}

/// Create a new event for a task
pub async fn create_task_event(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
    Json(mut data): Json<CreateTaskEvent>,
) -> Result<ResponseJson<ApiResponse<TaskEvent>>, ApiError> {
    // Ensure the task_id matches the URL parameter
    data.task_id = task.id;

    let event = TaskEvent::create(&deployment.db().pool, &data).await?;
    Ok(ResponseJson(ApiResponse::success(event)))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let events_router = Router::new()
        .route("/", get(get_task_events).post(create_task_event))
        .layer(from_fn_with_state(deployment.clone(), load_task_middleware));

    Router::new().nest("/tasks/{task_id}/events", events_router)
}

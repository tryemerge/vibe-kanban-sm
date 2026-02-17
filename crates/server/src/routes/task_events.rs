use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::{
    task::Task,
    task_event::{CreateTaskEvent, TaskEvent, TaskEventWithNames},
};
use deployment::Deployment;
use uuid::Uuid;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError, middleware::load_task_middleware};

#[derive(serde::Deserialize)]
pub struct EventsQuery {
    pub workspace_id: Option<Uuid>,
}

/// Get all events for a task (with column names resolved)
/// If workspace_id query param is provided, returns only events for that workspace
pub async fn get_task_events(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<EventsQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskEventWithNames>>>, ApiError> {
    let events = if let Some(workspace_id) = query.workspace_id {
        TaskEvent::find_by_workspace_id_with_names(&deployment.db().pool, workspace_id).await?
    } else {
        TaskEvent::find_by_task_id_with_names(&deployment.db().pool, task.id).await?
    };
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

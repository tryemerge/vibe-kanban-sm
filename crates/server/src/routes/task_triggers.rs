use axum::{
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{delete, get, post},
    Json, Router,
};
use sqlx::Error as SqlxError;
use uuid::Uuid;

use db::models::task_trigger::{CreateTaskTrigger, TaskTrigger};
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{error::ApiError, DeploymentImpl};

/// Router for task trigger endpoints
pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let _ = deployment; // Used to match pattern of other routers
    Router::new()
        .route("/tasks/{task_id}/triggers", get(list_task_triggers))
        .route("/tasks/{task_id}/triggers", post(create_task_trigger))
        .route(
            "/tasks/{task_id}/triggers/{trigger_id}",
            delete(delete_task_trigger),
        )
}

/// List all triggers for a task (what this task is waiting for)
async fn list_task_triggers(
    Path(task_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskTrigger>>>, ApiError> {
    let pool = &deployment.db().pool;
    let triggers = TaskTrigger::find_by_task(pool, task_id).await?;
    Ok(ResponseJson(ApiResponse::success(triggers)))
}

/// Create a new trigger for a task
async fn create_task_trigger(
    Path(task_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(mut payload): Json<CreateTaskTrigger>,
) -> Result<ResponseJson<ApiResponse<TaskTrigger>>, ApiError> {
    let pool = &deployment.db().pool;

    // Ensure the task_id in the payload matches the path
    payload.task_id = task_id;

    // Validate that trigger_task_id exists
    let trigger_task = db::models::task::Task::find_by_id(pool, payload.trigger_task_id).await?;
    if trigger_task.is_none() {
        return Err(ApiError::BadRequest(
            "Trigger task not found".to_string(),
        ));
    }

    // Prevent self-referential triggers
    if payload.task_id == payload.trigger_task_id {
        return Err(ApiError::BadRequest(
            "A task cannot trigger itself".to_string(),
        ));
    }

    let trigger = TaskTrigger::create(pool, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(trigger)))
}

/// Delete a trigger
async fn delete_task_trigger(
    Path((task_id, trigger_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify the trigger exists and belongs to this task
    let trigger = TaskTrigger::find_by_id(pool, trigger_id).await?;
    match trigger {
        Some(t) if t.task_id == task_id => {
            TaskTrigger::delete(pool, trigger_id).await?;
            Ok(ResponseJson(ApiResponse::success(())))
        }
        Some(_) => Err(ApiError::BadRequest(
            "Trigger does not belong to this task".to_string(),
        )),
        None => Err(ApiError::Database(SqlxError::RowNotFound)),
    }
}

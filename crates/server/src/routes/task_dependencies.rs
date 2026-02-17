use axum::{
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{delete, get, post},
    Json, Router,
};
use sqlx::Error as SqlxError;
use uuid::Uuid;

use db::models::task::TaskStatus;
use db::models::task_dependency::{CreateTaskDependency, TaskDependency};
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{error::ApiError, DeploymentImpl};

/// Router for task dependency endpoints
pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let _ = deployment;
    Router::new()
        .route("/tasks/{task_id}/dependencies", get(list_task_dependencies))
        .route(
            "/tasks/{task_id}/dependencies",
            post(create_task_dependency),
        )
        .route(
            "/tasks/{task_id}/dependencies/{dependency_id}",
            delete(delete_task_dependency),
        )
}

/// List all dependencies for a task (what this task is waiting for)
async fn list_task_dependencies(
    Path(task_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskDependency>>>, ApiError> {
    let pool = &deployment.db().pool;
    let deps = TaskDependency::find_by_task(pool, task_id).await?;
    Ok(ResponseJson(ApiResponse::success(deps)))
}

/// Create a new dependency for a task
async fn create_task_dependency(
    Path(task_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(mut payload): Json<CreateTaskDependency>,
) -> Result<ResponseJson<ApiResponse<TaskDependency>>, ApiError> {
    let pool = &deployment.db().pool;

    // Ensure the task_id in the payload matches the path
    payload.task_id = task_id;

    // Validate that depends_on_task_id exists
    let prereq_task =
        db::models::task::Task::find_by_id(pool, payload.depends_on_task_id).await?;
    if prereq_task.is_none() {
        return Err(ApiError::BadRequest(
            "Prerequisite task not found".to_string(),
        ));
    }

    // Prevent self-referential dependencies
    if payload.task_id == payload.depends_on_task_id {
        return Err(ApiError::BadRequest(
            "A task cannot depend on itself".to_string(),
        ));
    }

    let dep = TaskDependency::create(pool, &payload).await?;

    // Auto-satisfy if the prerequisite is already in a done/terminal column
    let prereq = prereq_task.unwrap();
    if let Some(column_id) = prereq.column_id {
        let column =
            db::models::kanban_column::KanbanColumn::find_by_id(pool, column_id).await?;
        if let Some(col) = column {
            if col.is_terminal && col.status == TaskStatus::Done {
                TaskDependency::satisfy_by_prerequisite(pool, prereq.id).await?;
                // Re-fetch to return the updated dependency
                if let Some(updated) = TaskDependency::find_by_id(pool, dep.id).await? {
                    return Ok(ResponseJson(ApiResponse::success(updated)));
                }
            }
        }
    }

    Ok(ResponseJson(ApiResponse::success(dep)))
}

/// Delete a dependency
async fn delete_task_dependency(
    Path((task_id, dependency_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify the dependency exists and belongs to this task
    let dep = TaskDependency::find_by_id(pool, dependency_id).await?;
    match dep {
        Some(d) if d.task_id == task_id => {
            TaskDependency::delete(pool, dependency_id).await?;
            Ok(ResponseJson(ApiResponse::success(())))
        }
        Some(_) => Err(ApiError::BadRequest(
            "Dependency does not belong to this task".to_string(),
        )),
        None => Err(ApiError::Database(SqlxError::RowNotFound)),
    }
}

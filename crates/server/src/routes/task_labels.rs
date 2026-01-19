use axum::{
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use sqlx::Error as SqlxError;
use uuid::Uuid;

use db::models::task_label::{CreateTaskLabel, TaskLabel, UpdateTaskLabel};
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{error::ApiError, DeploymentImpl};

/// Router for task label endpoints
pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let _ = deployment; // Used to match pattern of other routers
    Router::new()
        // Project-level label management
        .route("/projects/{project_id}/labels", get(list_project_labels))
        .route("/projects/{project_id}/labels", post(create_label))
        .route("/projects/{project_id}/labels/reorder", post(reorder_labels))
        .route(
            "/projects/{project_id}/labels/assignments",
            get(list_project_label_assignments),
        )
        .route(
            "/projects/{project_id}/labels/{label_id}",
            put(update_label),
        )
        .route(
            "/projects/{project_id}/labels/{label_id}",
            delete(delete_label),
        )
        // Task-level label assignment
        .route("/tasks/{task_id}/labels", get(list_task_labels))
        .route(
            "/tasks/{task_id}/labels/{label_id}",
            post(assign_label_to_task),
        )
        .route(
            "/tasks/{task_id}/labels/{label_id}",
            delete(remove_label_from_task),
        )
}

/// List all labels for a project
async fn list_project_labels(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskLabel>>>, ApiError> {
    let pool = &deployment.db().pool;
    let labels = TaskLabel::find_by_project(pool, project_id).await?;
    Ok(ResponseJson(ApiResponse::success(labels)))
}

/// Response type for task-label assignments
#[derive(Debug, serde::Serialize)]
struct TaskLabelAssignmentResponse {
    task_id: Uuid,
    label: TaskLabel,
}

/// List all task-label assignments for a project
/// Returns all (task_id, label) pairs for efficient bulk loading
async fn list_project_label_assignments(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskLabelAssignmentResponse>>>, ApiError> {
    let pool = &deployment.db().pool;
    let assignments = TaskLabel::find_all_assignments_by_project(pool, project_id).await?;
    let response: Vec<TaskLabelAssignmentResponse> = assignments
        .into_iter()
        .map(|(task_id, label)| TaskLabelAssignmentResponse { task_id, label })
        .collect();
    Ok(ResponseJson(ApiResponse::success(response)))
}

/// Create a new label for a project
async fn create_label(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(mut payload): Json<CreateTaskLabel>,
) -> Result<ResponseJson<ApiResponse<TaskLabel>>, ApiError> {
    let pool = &deployment.db().pool;

    // Ensure project_id matches path
    payload.project_id = project_id;

    // Validate project exists
    let project = db::models::project::Project::find_by_id(pool, project_id).await?;
    if project.is_none() {
        return Err(ApiError::BadRequest("Project not found".to_string()));
    }

    let label = TaskLabel::create(pool, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(label)))
}

/// Update a label
async fn update_label(
    Path((project_id, label_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateTaskLabel>,
) -> Result<ResponseJson<ApiResponse<TaskLabel>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify the label exists and belongs to this project
    let existing = TaskLabel::find_by_id(pool, label_id).await?;
    match existing {
        Some(label) if label.project_id == project_id => {
            let updated = TaskLabel::update(pool, label_id, &payload).await?;
            Ok(ResponseJson(ApiResponse::success(updated)))
        }
        Some(_) => Err(ApiError::BadRequest(
            "Label does not belong to this project".to_string(),
        )),
        None => Err(ApiError::Database(SqlxError::RowNotFound)),
    }
}

/// Delete a label
async fn delete_label(
    Path((project_id, label_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify the label exists and belongs to this project
    let existing = TaskLabel::find_by_id(pool, label_id).await?;
    match existing {
        Some(label) if label.project_id == project_id => {
            TaskLabel::delete(pool, label_id).await?;
            Ok(ResponseJson(ApiResponse::success(())))
        }
        Some(_) => Err(ApiError::BadRequest(
            "Label does not belong to this project".to_string(),
        )),
        None => Err(ApiError::Database(SqlxError::RowNotFound)),
    }
}

/// Reorder labels within a project
#[derive(Debug, Deserialize)]
struct ReorderLabelsPayload {
    label_ids: Vec<Uuid>,
}

async fn reorder_labels(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<ReorderLabelsPayload>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    TaskLabel::reorder(pool, project_id, &payload.label_ids).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

/// List all labels for a task
async fn list_task_labels(
    Path(task_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskLabel>>>, ApiError> {
    let pool = &deployment.db().pool;
    let labels = TaskLabel::find_by_task(pool, task_id).await?;
    Ok(ResponseJson(ApiResponse::success(labels)))
}

/// Assign a label to a task
async fn assign_label_to_task(
    Path((task_id, label_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify the task exists
    let task = db::models::task::Task::find_by_id(pool, task_id).await?;
    if task.is_none() {
        return Err(ApiError::BadRequest("Task not found".to_string()));
    }

    // Verify the label exists
    let label = TaskLabel::find_by_id(pool, label_id).await?;
    if label.is_none() {
        return Err(ApiError::BadRequest("Label not found".to_string()));
    }

    // Verify label belongs to same project as task
    let task = task.unwrap();
    let label = label.unwrap();
    if task.project_id != label.project_id {
        return Err(ApiError::BadRequest(
            "Label does not belong to the same project as the task".to_string(),
        ));
    }

    TaskLabel::assign_to_task(pool, task_id, label_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

/// Remove a label from a task
async fn remove_label_from_task(
    Path((task_id, label_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    TaskLabel::remove_from_task(pool, task_id, label_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

use axum::{
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use sqlx::Error as SqlxError;
use uuid::Uuid;

use db::models::task::Task;
use db::models::task_dependency::{CreateTaskDependency, TaskDependency};
use db::models::task_group::{CreateTaskGroup, TaskGroup, UpdateTaskGroup};
use db::models::task_group_dependency::TaskGroupDependency;
use db::models::group_event::{CreateGroupEvent, GroupEvent};
use deployment::Deployment;
use services::services::analytics::AnalyticsContext;
use services::services::task_grouper::TaskGrouperService;
use utils::response::ApiResponse;

use crate::{error::ApiError, DeploymentImpl};

/// Router for task group endpoints
pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let _ = deployment;
    Router::new()
        // Task group CRUD
        .route(
            "/projects/{project_id}/task-groups",
            get(list_task_groups),
        )
        .route(
            "/projects/{project_id}/task-groups",
            post(create_task_group),
        )
        .route(
            "/projects/{project_id}/task-groups/reorder",
            post(reorder_task_groups),
        )
        .route(
            "/projects/{project_id}/task-groups/{group_id}",
            put(update_task_group),
        )
        .route(
            "/projects/{project_id}/task-groups/{group_id}",
            delete(delete_task_group),
        )
        // Task-to-group assignment
        .route(
            "/tasks/{task_id}/task-group/{group_id}",
            post(add_task_to_group),
        )
        .route(
            "/tasks/{task_id}/task-group",
            delete(remove_task_from_group),
        )
        // State transitions
        .route(
            "/task-groups/{group_id}/transition",
            post(transition_group_state),
        )
        // Inter-group dependencies
        .route(
            "/projects/{project_id}/task-group-dependencies",
            get(list_group_dependencies),
        )
        .route(
            "/projects/{project_id}/task-group-dependencies",
            post(add_group_dependency),
        )
        .route(
            "/projects/{project_id}/task-group-dependencies/{dep_id}",
            delete(remove_group_dependency),
        )
        // Manual grouping analysis
        .route(
            "/projects/{project_id}/analyze-backlog",
            post(analyze_backlog),
        )
}

// ─── Task Group CRUD ───────────────────────────────────────────────

async fn list_task_groups(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskGroup>>>, ApiError> {
    let pool = &deployment.db().pool;
    let groups = TaskGroup::find_by_project(pool, project_id).await?;
    Ok(ResponseJson(ApiResponse::success(groups)))
}

async fn create_task_group(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(mut payload): Json<CreateTaskGroup>,
) -> Result<ResponseJson<ApiResponse<TaskGroup>>, ApiError> {
    let pool = &deployment.db().pool;
    payload.project_id = project_id;

    let project = db::models::project::Project::find_by_id(pool, project_id).await?;
    if project.is_none() {
        return Err(ApiError::BadRequest("Project not found".to_string()));
    }

    let group = TaskGroup::create(pool, &payload).await?;

    // Emit group creation event
    let event = CreateGroupEvent {
        task_group_id: group.id,
        task_id: None,
        event_type: "group_state_change".to_string(),
        actor_type: "user".to_string(),
        summary: format!("Task group '{}' created in draft state", group.name),
        payload: Some(format!(r#"{{"from": null, "to": "draft"}}"#)),
    };
    GroupEvent::create(pool, &event).await?;

    Ok(ResponseJson(ApiResponse::success(group)))
}

async fn update_task_group(
    Path((project_id, group_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateTaskGroup>,
) -> Result<ResponseJson<ApiResponse<TaskGroup>>, ApiError> {
    let pool = &deployment.db().pool;

    let existing = TaskGroup::find_by_id(pool, group_id).await?;
    match existing {
        Some(group) if group.project_id == project_id => {
            if group.state != "draft" {
                return Err(ApiError::Conflict(
                    "Cannot modify a task group that is not in draft state".to_string(),
                ));
            }
            let updated = TaskGroup::update(pool, group_id, &payload)
                .await?
                .ok_or(ApiError::Database(SqlxError::RowNotFound))?;
            Ok(ResponseJson(ApiResponse::success(updated)))
        }
        Some(_) => Err(ApiError::BadRequest(
            "Task group does not belong to this project".to_string(),
        )),
        None => Err(ApiError::Database(SqlxError::RowNotFound)),
    }
}

async fn delete_task_group(
    Path((project_id, group_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    let existing = TaskGroup::find_by_id(pool, group_id).await?;
    match existing {
        Some(group) if group.project_id == project_id => {
            if group.state != "draft" {
                return Err(ApiError::Conflict(
                    "Cannot delete a task group that is not in draft state".to_string(),
                ));
            }
            TaskGroup::delete(pool, group_id).await?;
            Ok(ResponseJson(ApiResponse::success(())))
        }
        Some(_) => Err(ApiError::BadRequest(
            "Task group does not belong to this project".to_string(),
        )),
        None => Err(ApiError::Database(SqlxError::RowNotFound)),
    }
}

#[derive(Debug, Deserialize)]
struct ReorderGroupsPayload {
    group_ids: Vec<Uuid>,
}

async fn reorder_task_groups(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<ReorderGroupsPayload>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    TaskGroup::reorder(pool, project_id, payload.group_ids).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

// ─── Task-to-Group Assignment ──────────────────────────────────────

/// Add a task to a group, creating auto-dependencies on the previous task in the group.
async fn add_task_to_group(
    Path((task_id, group_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify group exists
    let group = TaskGroup::find_by_id(pool, group_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Task group not found".to_string()))?;

    // Reject if group is not in draft state
    if group.state != "draft" {
        return Err(ApiError::Conflict(
            "Cannot add tasks to a task group that is not in draft state".to_string(),
        ));
    }

    // Verify task exists
    let task = Task::find_by_id(pool, task_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Task not found".to_string()))?;

    // Verify task belongs to same project as group
    if task.project_id != group.project_id {
        return Err(ApiError::BadRequest(
            "Task does not belong to the same project as the group".to_string(),
        ));
    }

    // Remove existing auto-group dependencies if task was in another group
    if task.task_group_id.is_some() {
        TaskDependency::delete_auto_group_by_task(pool, task_id).await?;
    }

    // Find the last task in the group (by created_at order) to create the dependency chain
    let group_tasks = Task::find_by_group(pool, group_id).await?;
    let last_task = group_tasks.iter().filter(|t| t.id != task_id).last();

    if let Some(prev_task) = last_task {
        let dep = CreateTaskDependency {
            task_id,
            depends_on_task_id: prev_task.id,
        };
        TaskDependency::create_with_auto_group(pool, &dep, true).await?;
    }

    // Assign task to group
    Task::update_task_group(pool, task_id, Some(group_id)).await?;

    // Re-fetch the updated task
    let updated = Task::find_by_id(pool, task_id)
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    // Emit task addition event
    let event = CreateGroupEvent {
        task_group_id: group_id,
        task_id: Some(task_id),
        event_type: "dag_task_added".to_string(),
        actor_type: "user".to_string(),
        summary: format!("Task '{}' added to group '{}'", task.title, group.name),
        payload: None,
    };
    GroupEvent::create(pool, &event).await?;

    Ok(ResponseJson(ApiResponse::success(updated)))
}

/// Remove a task from its group, cleaning up auto-dependencies and re-linking the chain.
async fn remove_task_from_group(
    Path(task_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    let pool = &deployment.db().pool;

    let task = Task::find_by_id(pool, task_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Task not found".to_string()))?;

    let group_id = match task.task_group_id {
        Some(gid) => gid,
        None => {
            return Err(ApiError::BadRequest(
                "Task is not in a group".to_string(),
            ));
        }
    };

    // Verify group is in draft state
    let group = TaskGroup::find_by_id(pool, group_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Task group not found".to_string()))?;

    if group.state != "draft" {
        return Err(ApiError::Conflict(
            "Cannot remove tasks from a task group that is not in draft state".to_string(),
        ));
    }

    // Re-link: find the predecessor and successor in the auto-dependency chain
    // Predecessor: who does this task depend on (auto-group)?
    let deps_on_me = TaskDependency::find_by_task(pool, task_id).await?;
    let predecessor = deps_on_me.iter().find(|d| d.is_auto_group);

    // Successor: who depends on this task (auto-group)?
    let blocked_by_me = TaskDependency::find_by_prerequisite(pool, task_id).await?;
    let successor = blocked_by_me.iter().find(|d| d.is_auto_group);

    // If both exist, create a direct link from predecessor's prerequisite to successor's task
    if let (Some(pred), Some(succ)) = (predecessor, successor) {
        let relink = CreateTaskDependency {
            task_id: succ.task_id,
            depends_on_task_id: pred.depends_on_task_id,
        };
        TaskDependency::create_with_auto_group(pool, &relink, true).await?;
    }

    // Delete all auto-group dependencies involving this task
    TaskDependency::delete_auto_group_by_task(pool, task_id).await?;

    // Clear task's group assignment
    Task::update_task_group(pool, task_id, None).await?;

    let updated = Task::find_by_id(pool, task_id)
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    Ok(ResponseJson(ApiResponse::success(updated)))
}

// ─── Inter-Group Dependencies ──────────────────────────────────────

async fn list_group_dependencies(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskGroupDependency>>>, ApiError> {
    let pool = &deployment.db().pool;

    // Get all groups for this project, then collect their dependencies
    let groups = TaskGroup::find_by_project(pool, project_id).await?;
    let mut all_deps = Vec::new();
    for group in &groups {
        let deps = TaskGroupDependency::find_by_group(pool, group.id).await?;
        all_deps.extend(deps);
    }
    Ok(ResponseJson(ApiResponse::success(all_deps)))
}

#[derive(Debug, Deserialize)]
struct AddGroupDependencyPayload {
    task_group_id: Uuid,
    depends_on_group_id: Uuid,
}

async fn add_group_dependency(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<AddGroupDependencyPayload>,
) -> Result<ResponseJson<ApiResponse<TaskGroupDependency>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify both groups exist and belong to this project
    let group = TaskGroup::find_by_id(pool, payload.task_group_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Task group not found".to_string()))?;
    let prereq = TaskGroup::find_by_id(pool, payload.depends_on_group_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Prerequisite group not found".to_string()))?;

    if group.project_id != project_id || prereq.project_id != project_id {
        return Err(ApiError::BadRequest(
            "Both groups must belong to the specified project".to_string(),
        ));
    }

    // Prevent self-referential
    if payload.task_group_id == payload.depends_on_group_id {
        return Err(ApiError::BadRequest(
            "A group cannot depend on itself".to_string(),
        ));
    }

    let dep = TaskGroupDependency::create(
        pool,
        payload.task_group_id,
        payload.depends_on_group_id,
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(dep)))
}

async fn remove_group_dependency(
    Path((project_id, dep_id)): Path<(Uuid, Uuid)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    let dep = TaskGroupDependency::find_by_id(pool, dep_id)
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    // Verify the dependency's group belongs to this project
    let group = TaskGroup::find_by_id(pool, dep.task_group_id)
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    if group.project_id != project_id {
        return Err(ApiError::BadRequest(
            "Dependency does not belong to this project".to_string(),
        ));
    }

    TaskGroupDependency::delete(pool, dep_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

// ─── State Transitions ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TransitionPayload {
    from: String,
    to: String,
}

async fn transition_group_state(
    Path(group_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<TransitionPayload>,
) -> Result<ResponseJson<ApiResponse<TaskGroup>>, ApiError> {
    let pool = &deployment.db().pool;

    // Perform the state transition
    let group = TaskGroup::transition_state(pool, group_id, &payload.from, &payload.to)
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    // Log the transition event
    let event = CreateGroupEvent {
        task_group_id: group_id,
        task_id: None,
        event_type: "group_state_change".to_string(),
        actor_type: "user".to_string(),
        summary: format!(
            "Group '{}' transitioned from {} to {}",
            group.name, payload.from, payload.to
        ),
        payload: Some(serde_json::json!({
            "from": payload.from,
            "to": payload.to,
        }).to_string()),
    };
    GroupEvent::create(pool, &event).await?;

    // If transitioning to 'analyzing', create the analysis task
    if payload.to == "analyzing" {
        use services::services::group_analyzer::GroupAnalyzer;

        let analyzer = GroupAnalyzer::new(deployment.db().clone());
        match analyzer.create_analysis_task(group_id).await {
            Ok(task) => {
                tracing::info!(
                    "Created analysis task {} for group {} ({})",
                    task.id,
                    group.name,
                    group_id
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to create analysis task for group {}: {}",
                    group_id,
                    e
                );
            }
        }
    }

    Ok(ResponseJson(ApiResponse::success(group)))
}

// ─── Manual Grouping Analysis ─────────────────────────────────────

/// Manually trigger the Task Grouper agent to analyze ungrouped tasks in a project's backlog.
/// The Task Grouper runs automatically every 5 minutes, but this endpoint allows on-demand triggering.
async fn analyze_backlog(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify project exists
    let project = db::models::project::Project::find_by_id(pool, project_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Project not found".to_string()))?;

    // Count ungrouped tasks
    let ungrouped = Task::find_ungrouped_by_project(pool, project_id).await?;
    let ungrouped_count = ungrouped.len();

    if ungrouped_count == 0 {
        return Ok(ResponseJson(ApiResponse::success(serde_json::json!({
            "message": "No ungrouped tasks found in backlog",
            "ungrouped_count": 0,
        }))));
    }

    if ungrouped_count < 2 {
        return Ok(ResponseJson(ApiResponse::success(serde_json::json!({
            "message": "Only 1 ungrouped task found, need at least 2 to group",
            "ungrouped_count": 1,
        }))));
    }

    // Create a service instance for manual analysis (without background timer)
    // Note: This doesn't spawn a background task, just uses the analysis logic
    let analytics_ctx = deployment.analytics().as_ref().map(|s| AnalyticsContext {
        user_id: deployment.user_id().to_string(),
        analytics_service: s.clone(),
    });
    let service = TaskGrouperService::new(deployment.db().clone(), analytics_ctx);

    // Run analysis for this project
    service.analyze_project_by_id(project_id).await
        .map_err(|e| {
            tracing::error!("Failed to analyze backlog for project {}: {}", project_id, e);
            ApiError::BadRequest(format!("Grouping analysis failed: {}", e))
        })?;

    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "message": format!("Grouping analysis requested for {} ungrouped tasks in project '{}'", ungrouped_count, project.name),
        "ungrouped_count": ungrouped_count,
        "project_id": project_id.to_string(),
    }))))
}

use axum::{
    extract::{
        Path, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::{IntoResponse, Json as ResponseJson},
    routing::{delete, get, post, put},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use serde::Deserialize;
use sqlx::Error as SqlxError;
use uuid::Uuid;

use std::str::FromStr;

use db::models::agent::Agent;
use db::models::context_artifact::ContextArtifact;
use db::models::kanban_column::KanbanColumn;
use db::models::project::Project;
use db::models::project_repo::ProjectRepo;
use db::models::task::{CreateTask, Task, TaskWithAttemptStatus};
use services::services::events::task_patch;
use db::models::task_dependency::{CreateTaskDependency, TaskDependency};
use db::models::task_group::{CreateTaskGroup, TaskGroup, UpdateTaskGroup};
use db::models::task_group_dependency::TaskGroupDependency;
use db::models::group_event::{CreateGroupEvent, GroupEvent};
use db::models::workspace_repo::{CreateWorkspaceRepo, WorkspaceRepo};
use deployment::Deployment;
use executors::executors::BaseCodingAgent;
use executors::profile::ExecutorProfileId;
use services::services::container::{AgentContext, ContainerService};
use services::services::events::{group_event_patch, group_patch};
use services::services::git::GitService;
use services::services::group_evaluator::{self, GROUP_EVALUATOR_AGENT_ID};
use services::services::task_grouper::{self, TASK_GROUPER_AGENT_ID};
use utils::response::ApiResponse;

use crate::{error::ApiError, routes::tasks::spawn_agent_execution, DeploymentImpl};

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
        // Single group detail (group + tasks + dependencies)
        .route(
            "/task-groups/{group_id}",
            get(get_task_group_detail),
        )
        // Group workspaces (for column-level agent viewing)
        .route(
            "/task-groups/{group_id}/workspaces",
            get(list_group_workspaces),
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
        // Execution DAG
        .route(
            "/task-groups/{group_id}/set-execution-dag",
            post(set_execution_dag),
        )
        // Manual grouping analysis
        .route(
            "/projects/{project_id}/analyze-backlog",
            post(analyze_backlog),
        )
        // WebSocket streams
        .route(
            "/projects/{project_id}/task-groups/stream/ws",
            get(stream_task_groups_ws),
        )
        .route(
            "/projects/{project_id}/group-events/stream/ws",
            get(stream_group_events_ws),
        )
}

// ─── WebSocket Streams ────────────────────────────────────────────

pub async fn stream_task_groups_ws(
    ws: WebSocketUpgrade,
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_task_groups_ws(socket, deployment, project_id).await {
            tracing::warn!("task_groups WS closed: {}", e);
        }
    })
}

async fn handle_task_groups_ws(
    socket: WebSocket,
    deployment: DeploymentImpl,
    project_id: Uuid,
) -> anyhow::Result<()> {
    let mut stream = deployment
        .events()
        .stream_task_groups_raw(project_id)
        .await?
        .map_ok(|msg| msg.to_ws_message_unchecked());

    let (mut sender, mut receiver) = socket.split();
    tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

    while let Some(item) = stream.next().await {
        match item {
            Ok(msg) => {
                if sender.send(msg).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                tracing::error!("task_groups stream error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

pub async fn stream_group_events_ws(
    ws: WebSocketUpgrade,
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_group_events_ws(socket, deployment, project_id).await {
            tracing::warn!("group_events WS closed: {}", e);
        }
    })
}

async fn handle_group_events_ws(
    socket: WebSocket,
    deployment: DeploymentImpl,
    project_id: Uuid,
) -> anyhow::Result<()> {
    let mut stream = deployment
        .events()
        .stream_group_events_raw(project_id)
        .await?
        .map_ok(|msg| msg.to_ws_message_unchecked());

    let (mut sender, mut receiver) = socket.split();
    tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

    while let Some(item) = stream.next().await {
        match item {
            Ok(msg) => {
                if sender.send(msg).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                tracing::error!("group_events stream error: {}", e);
                break;
            }
        }
    }
    Ok(())
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

#[derive(serde::Serialize)]
struct TaskGroupDetail {
    group: TaskGroup,
    tasks: Vec<db::models::task::Task>,
    dependencies: Vec<TaskGroupDependency>,
    blocked_by: Vec<TaskGroupDependency>,
}

async fn get_task_group_detail(
    Path(group_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TaskGroupDetail>>, ApiError> {
    let pool = &deployment.db().pool;
    let group = TaskGroup::find_by_id(pool, group_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Task group not found".to_string()))?;
    let tasks = Task::find_by_group(pool, group_id).await?;
    let dependencies = TaskGroupDependency::find_by_group(pool, group_id).await?;
    let blocked_by = TaskGroupDependency::find_by_prerequisite(pool, group_id).await?;
    Ok(ResponseJson(ApiResponse::success(TaskGroupDetail { group, tasks, dependencies, blocked_by })))
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

    // Enforce Plan-first workflow: non-backlog groups must link to an iplan artifact
    if !payload.is_backlog.unwrap_or(false) {
        match payload.artifact_id {
            None => {
                return Err(ApiError::BadRequest(
                    "artifact_id is required — create a Plan first via the Project Agent, then use the Plans panel to create the group".to_string(),
                ));
            }
            Some(artifact_id) => {
                let artifact = ContextArtifact::find_by_id(pool, artifact_id).await?;
                match artifact {
                    None => {
                        return Err(ApiError::BadRequest(
                            "artifact not found".to_string(),
                        ));
                    }
                    Some(a) if a.artifact_type != "iplan" => {
                        return Err(ApiError::BadRequest(
                            "artifact_id must reference an iplan artifact".to_string(),
                        ));
                    }
                    _ => {}
                }
            }
        }
    }

    let group = TaskGroup::create(pool, &payload).await?;

    // Lock the project when a new non-backlog group is created — signals that new work
    // has entered the pipeline and the project needs re-stabilization.
    if !group.is_backlog {
        if let Err(e) = db::models::project::Project::set_ready_locked(pool, project_id, true).await {
            tracing::warn!("Failed to set ready_locked on group creation: {}", e);
        }
    }

    // Broadcast via WebSocket
    deployment
        .events()
        .msg_store()
        .push_patch(group_patch::add(&group));

    // Emit group creation event
    let event = CreateGroupEvent {
        task_group_id: group.id,
        task_id: None,
        event_type: "group_state_change".to_string(),
        actor_type: "user".to_string(),
        summary: format!("Task group '{}' created in draft state", group.name),
        payload: Some(format!(r#"{{"from": null, "to": "draft"}}"#)),
    };
    let event = GroupEvent::create(pool, &event).await?;
    deployment
        .events()
        .msg_store()
        .push_patch(group_event_patch::add(&event));

    // Auto-trigger Task Builder when a Plan-backed group is created
    if group.artifact_id.is_some() && !group.is_backlog {
        let pool_clone = pool.clone();
        let deployment_clone = deployment.clone();
        let project = project.unwrap(); // already verified above
        tokio::spawn(async move {
            if let Err(e) = launch_task_builder_agent(&pool_clone, &deployment_clone, &project).await {
                tracing::error!("Auto-trigger Task Builder failed: {}", e);
            }
        });
    }

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
            deployment
                .events()
                .msg_store()
                .push_patch(group_patch::replace(&updated));
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
            deployment
                .events()
                .msg_store()
                .push_patch(group_patch::remove(group_id));
            Ok(ResponseJson(ApiResponse::success(())))
        }
        Some(_) => Err(ApiError::BadRequest(
            "Task group does not belong to this project".to_string(),
        )),
        None => Err(ApiError::Database(SqlxError::RowNotFound)),
    }
}

/// List workspaces for a task group (for column-level agent viewing)
async fn list_group_workspaces(
    Path(group_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<db::models::workspace::Workspace>>>, ApiError> {
    let pool = &deployment.db().pool;
    let workspaces = db::models::workspace::Workspace::find_by_task_group(pool, group_id).await?;
    Ok(ResponseJson(ApiResponse::success(workspaces)))
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

    // Broadcast updated positions
    let groups = TaskGroup::find_by_project(pool, project_id).await?;
    for group in &groups {
        deployment
            .events()
            .msg_store()
            .push_patch(group_patch::replace(group));
    }

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

    // Broadcast task update so frontend sees the task_group_id change live
    emit_task_patch(&deployment, updated.clone());

    // Broadcast group update (task count changed)
    if let Some(updated_group) = TaskGroup::find_by_id(pool, group_id).await? {
        deployment
            .events()
            .msg_store()
            .push_patch(group_patch::replace(&updated_group));
    }

    // Emit task addition event
    let event = CreateGroupEvent {
        task_group_id: group_id,
        task_id: Some(task_id),
        event_type: "dag_task_added".to_string(),
        actor_type: "user".to_string(),
        summary: format!("Task '{}' added to group '{}'", task.title, group.name),
        payload: None,
    };
    let event = GroupEvent::create(pool, &event).await?;
    deployment
        .events()
        .msg_store()
        .push_patch(group_event_patch::add(&event));

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

    // Broadcast task update so frontend sees the task_group_id change live
    emit_task_patch(&deployment, updated.clone());

    // Broadcast group update (task removed)
    if let Some(updated_group) = TaskGroup::find_by_id(pool, group_id).await? {
        deployment
            .events()
            .msg_store()
            .push_patch(group_patch::replace(&updated_group));
    }

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

    // Broadcast group state change
    deployment
        .events()
        .msg_store()
        .push_patch(group_patch::replace(&group));

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
    let event = GroupEvent::create(pool, &event).await?;
    deployment
        .events()
        .msg_store()
        .push_patch(group_event_patch::add(&event));

    // If transitioning to 'analyzing', launch Group Evaluator via execution pipeline
    // The agent will build the execution DAG and call set_execution_dag when done,
    // which advances analyzing → ready and triggers the project DAG builder
    if payload.to == "analyzing" {
        if let Err(e) = launch_group_evaluator(pool, &deployment, group.project_id).await {
            tracing::error!("Failed to launch Group Evaluator for group {}: {}", group_id, e);
        }
    }

    // If transitioning to 'ready' (from prereq_eval — either prereq satisfied or new dep added):
    // 1. Auto-advance THIS group to executing if its inter-group deps are satisfied
    // 2. Advance the project DAG builder to pick up the NEXT unblocked ready group
    if payload.to == "ready" {
        if let Err(e) = try_auto_advance_to_executing(pool, &deployment, group_id).await {
            tracing::error!("Failed to auto-advance group {} to executing: {}", group_id, e);
        }
        if let Err(e) = deployment.container().advance_project_dag(group.project_id).await {
            tracing::error!("Failed to advance project DAG after group {} → ready: {}", group_id, e);
        }
    }

    // If transitioning to 'done', unblock other groups that were waiting on this one
    if payload.to == "done" {
        if let Err(e) = deployment.container().advance_project_dag(group.project_id).await {
            tracing::error!("Failed to advance project DAG after group {} → done: {}", group_id, e);
        }
    }

    // Re-fetch group to get latest state (may have auto-advanced)
    let final_group = TaskGroup::find_by_id(pool, group_id)
        .await?
        .unwrap_or(group);

    Ok(ResponseJson(ApiResponse::success(final_group)))
}

// ─── Execution DAG ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SetExecutionDagPayload {
    parallel_sets: Vec<Vec<String>>,
    group_name: Option<String>,
}

/// Store an execution DAG for a task group, advance analyzing → ready,
/// then let the project DAG builder decide when to advance further.
async fn set_execution_dag(
    Path(group_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<SetExecutionDagPayload>,
) -> Result<ResponseJson<ApiResponse<TaskGroup>>, ApiError> {
    let pool = &deployment.db().pool;

    // 1. Verify group exists and is in 'analyzing' state
    let group = TaskGroup::find_by_id(pool, group_id)
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    if group.state != "analyzing" {
        return Err(ApiError::Conflict(format!(
            "Group must be in 'analyzing' state, currently: {}",
            group.state
        )));
    }

    // 2. Validate all task IDs in the DAG belong to this group
    let group_tasks = TaskGroup::get_tasks(pool, group_id).await?;
    let group_task_ids: std::collections::HashSet<String> = group_tasks
        .iter()
        .map(|t| t.id.to_string())
        .collect();

    for set in &payload.parallel_sets {
        for task_id in set {
            if !group_task_ids.contains(task_id) {
                return Err(ApiError::BadRequest(format!(
                    "Task {} is not in this group",
                    task_id
                )));
            }
        }
    }

    // 3. Store the DAG
    let dag_json = serde_json::json!({ "parallel_sets": payload.parallel_sets }).to_string();
    TaskGroup::update_execution_dag(pool, group_id, &dag_json).await?;

    // 3b. Create inter-task dependencies from DAG parallel sets
    // Tasks in set[i+1] depend on ALL tasks in set[i]
    // First, clear any existing auto-group deps for tasks in this group
    for set in &payload.parallel_sets {
        for task_id_str in set {
            if let Ok(tid) = Uuid::parse_str(task_id_str) {
                let _ = TaskDependency::delete_auto_group_by_task(pool, tid).await;
            }
        }
    }
    // Now create sequential dependencies between adjacent sets
    let mut deps_created = 0u32;
    for i in 0..payload.parallel_sets.len().saturating_sub(1) {
        let prev_set = &payload.parallel_sets[i];
        let next_set = &payload.parallel_sets[i + 1];
        for next_task_str in next_set {
            let Ok(next_tid) = Uuid::parse_str(next_task_str) else {
                continue;
            };
            for prev_task_str in prev_set {
                let Ok(prev_tid) = Uuid::parse_str(prev_task_str) else {
                    continue;
                };
                let dep = CreateTaskDependency {
                    task_id: next_tid,
                    depends_on_task_id: prev_tid,
                };
                match TaskDependency::create_with_auto_group(pool, &dep, true).await {
                    Ok(_) => deps_created += 1,
                    Err(e) => tracing::warn!(
                        "Failed to create auto-group dep {} → {}: {}",
                        next_tid, prev_tid, e
                    ),
                }
            }
        }
    }
    if deps_created > 0 {
        tracing::info!(
            "Created {} auto-group dependencies for group {} from {} parallel sets",
            deps_created, group_id, payload.parallel_sets.len()
        );
    }

    // 4. Optionally rename the group
    if let Some(ref name) = payload.group_name {
        sqlx::query("UPDATE task_groups SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(group_id)
            .execute(pool)
            .await?;
    }

    // 5. Log the DAG storage event
    let dag_event = CreateGroupEvent {
        task_group_id: group_id,
        task_id: None,
        event_type: "dag_stored".to_string(),
        actor_type: "agent".to_string(),
        summary: format!(
            "Execution DAG stored for '{}' ({} parallel sets, {} tasks)",
            payload.group_name.as_deref().unwrap_or(&group.name),
            payload.parallel_sets.len(),
            payload.parallel_sets.iter().map(|s| s.len()).sum::<usize>()
        ),
        payload: Some(dag_json.clone()),
    };
    let dag_event = GroupEvent::create(pool, &dag_event).await?;
    deployment
        .events()
        .msg_store()
        .push_patch(group_event_patch::add(&dag_event));

    // 6. Transition analyzing → ready (joins the ready pool; project DAG builder picks up from here)
    let ready_group = TaskGroup::transition_state(pool, group_id, "analyzing", "ready")
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    let transition_event = CreateGroupEvent {
        task_group_id: group_id,
        task_id: None,
        event_type: "group_state_change".to_string(),
        actor_type: "system".to_string(),
        summary: format!("Group '{}' advanced analyzing → ready", ready_group.name),
        payload: Some(
            serde_json::json!({
                "from": "analyzing",
                "to": "ready",
            })
            .to_string(),
        ),
    };
    let transition_event = GroupEvent::create(pool, &transition_event).await?;
    deployment
        .events()
        .msg_store()
        .push_patch(group_event_patch::add(&transition_event));

    // Broadcast group in ready state
    deployment
        .events()
        .msg_store()
        .push_patch(group_patch::replace(&ready_group));

    // 6b. Try to advance the project-level DAG: send first unblocked ready group to prereq_eval
    if let Err(e) = deployment.container().advance_project_dag(ready_group.project_id).await {
        tracing::error!("Failed to advance project DAG after group {} reached ready: {}", group_id, e);
    }

    Ok(ResponseJson(ApiResponse::success(ready_group)))
}

// ─── Manual Grouping Analysis ─────────────────────────────────────

/// Manually trigger the Task Builder agent to process empty draft groups in the backlog.
/// Uses the standard execution pipeline (workspace → session → execution process) so the
/// agent's logs stream to the frontend via the same TaskAttemptPanel used for task-level agents.
#[derive(Debug, Deserialize)]
struct AnalyzeBacklogPayload {
    prompt: Option<String>,
}

async fn analyze_backlog(
    Path(project_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    body: Option<axum::Json<AnalyzeBacklogPayload>>,
) -> Result<ResponseJson<ApiResponse<serde_json::Value>>, ApiError> {
    let _user_prompt = body.and_then(|b| b.0.prompt);
    let pool = &deployment.db().pool;

    // Verify project exists
    let project = Project::find_by_id(pool, project_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Project not found".to_string()))?;

    let workspace_id = launch_task_builder_agent(pool, &deployment, &project)
        .await
        .map_err(|e| {
            tracing::error!("Failed to launch Task Builder: {}", e);
            ApiError::BadRequest(format!("Failed to launch Task Builder: {}", e))
        })?;

    Ok(ResponseJson(ApiResponse::success(serde_json::json!({
        "message": "Task Builder agent launched",
        "workspace_id": workspace_id.to_string(),
    }))))
}

/// Core helper: get/create the persistent Task Builder workspace and launch a new session.
/// Returns the workspace ID.
async fn launch_task_builder_agent(
    pool: &sqlx::PgPool,
    deployment: &DeploymentImpl,
    project: &Project,
) -> Result<Uuid, anyhow::Error> {
    // Get the Task Builder (Task Grouper) agent
    let agent = Agent::find_by_id(pool, TASK_GROUPER_AGENT_ID)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Task Builder agent not found"))?;

    // Build the Task Builder prompt
    let prompt = task_grouper::build_grouper_prompt(project, &[], None);

    // Get or create the project's persistent Task Builder workspace
    let (workspace, newly_created) = get_or_create_column_agent_workspace(
        pool,
        project,
        project.grouper_workspace_id,
        &agent.executor,
        &format!("Task Builder — {}", project.name),
        "grouper",
    )
    .await?;

    // Store workspace on project if newly created, and broadcast update
    if newly_created {
        if let Err(e) = Project::set_grouper_workspace_id(pool, project.id, workspace.id).await {
            tracing::warn!("Failed to store grouper_workspace_id on project: {}", e);
        }
        if let Ok(Some(updated_project)) = Project::find_by_id(pool, project.id).await {
            deployment
                .events()
                .msg_store()
                .push_patch(services::services::events::project_patch::replace(&updated_project));
        }
    }

    // Parse executor from agent config
    let base_agent = BaseCodingAgent::from_str(&agent.executor)?;
    let executor_profile_id = ExecutorProfileId::new(base_agent);

    // Build agent context
    let agent_context = AgentContext {
        system_prompt: Some(agent.system_prompt.clone()),
        workflow_history: None,
        start_command: Some(prompt),
        deliverable: None,
        name: agent.name.clone(),
        color: agent.color.clone(),
        column_name: "Task Builder".to_string(),
        project_context: None,
        task_id_override: None,
    };

    // Launch agent via the standard execution pipeline (new session in persistent workspace)
    deployment
        .container()
        .launch_agent_in_workspace(&workspace, executor_profile_id, agent_context)
        .await?;

    tracing::info!(
        "Task Builder launched for project={}, workspace={}",
        project.id, workspace.id
    );

    Ok(workspace.id)
}

// ─── Auto-Start Unblocked Tasks ──────────────────────────────────

/// Emit a task_patch::replace for a Task so the frontend WS stream updates in real-time
fn emit_task_patch(deployment: &DeploymentImpl, task: Task) {
    let task_status = TaskWithAttemptStatus {
        task,
        has_in_progress_attempt: false,
        last_attempt_failed: false,
        executor: String::new(),
        latest_attempt_id: None,
    };
    deployment
        .events()
        .msg_store()
        .push_patch(task_patch::replace(&task_status));
}

/// Auto-advance a group from ready → executing if no inter-group dependencies block it.
/// Creates the group workspace, starts unblocked tasks in the first parallel set.
async fn try_auto_advance_to_executing(
    pool: &sqlx::PgPool,
    deployment: &DeploymentImpl,
    group_id: Uuid,
) -> Result<(), anyhow::Error> {
    let has_blockers = TaskGroupDependency::has_unsatisfied(pool, group_id).await?;
    if has_blockers {
        tracing::info!(
            "Group {} has unsatisfied dependencies, staying in 'ready'",
            group_id
        );
        return Ok(());
    }

    // Check ready_lock — if the project is locked (briefs or plans pending review),
    // don't auto-advance. The Group Evaluator will unlock once the project is stable.
    if let Some(group) = TaskGroup::find_by_id(pool, group_id).await? {
        if let Some(project) = Project::find_by_id(pool, group.project_id).await? {
            if project.ready_locked {
                tracing::info!(
                    "Group {} staying in 'ready' — project is ready_locked (pending briefs/plans)",
                    group_id
                );
                return Ok(());
            }
        }
    }

    let group = match TaskGroup::transition_state(pool, group_id, "ready", "executing").await? {
        Some(g) => g,
        None => {
            tracing::warn!(
                "Failed to auto-advance group {} from ready → executing",
                group_id
            );
            return Ok(());
        }
    };

    tracing::info!(
        "Auto-advanced group '{}' ({}) ready → executing (no blockers)",
        group.name,
        group_id
    );

    let exec_event = CreateGroupEvent {
        task_group_id: group_id,
        task_id: None,
        event_type: "group_state_change".to_string(),
        actor_type: "system".to_string(),
        summary: format!(
            "Group '{}' auto-advanced ready → executing (no blockers)",
            group.name
        ),
        payload: Some(
            serde_json::json!({
                "from": "ready",
                "to": "executing",
                "auto_advanced": true,
            })
            .to_string(),
        ),
    };
    let exec_event = GroupEvent::create(pool, &exec_event).await?;
    deployment
        .events()
        .msg_store()
        .push_patch(group_event_patch::add(&exec_event));

    // Emit group patch immediately so frontend knows group is executing
    deployment
        .events()
        .msg_store()
        .push_patch(group_patch::replace(&group));

    // Create group-level workspace (ADR-015: shared worktree)
    match create_group_workspace(pool, deployment.clone(), &group, &group.name).await {
        Ok(ws) => {
            tracing::info!(
                "Created group workspace {} for group '{}' ({})",
                ws.id, group.name, group_id
            );
            let ws_event = CreateGroupEvent {
                task_group_id: group_id,
                task_id: None,
                event_type: "workspace_created".to_string(),
                actor_type: "system".to_string(),
                summary: format!(
                    "Shared workspace created for group '{}' (branch: {})",
                    group.name, ws.branch
                ),
                payload: Some(serde_json::json!({
                    "workspace_id": ws.id,
                    "branch": ws.branch,
                }).to_string()),
            };
            if let Ok(evt) = GroupEvent::create(pool, &ws_event).await {
                deployment
                    .events()
                    .msg_store()
                    .push_patch(group_event_patch::add(&evt));
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to create group workspace for group {}: {} — tasks will fall back to per-task workspaces",
                group_id, e
            );
        }
    }

    // Auto-start unblocked tasks in the first parallel set
    if let Some(dag_json) = &group.execution_dag {
        if let Ok(dag) = serde_json::from_str::<serde_json::Value>(dag_json) {
            if let Some(parallel_sets) = dag.get("parallel_sets").and_then(|v| v.as_array()) {
                let sets: Vec<Vec<String>> = parallel_sets
                    .iter()
                    .filter_map(|s| {
                        s.as_array().map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                    })
                    .collect();

                if let Err(e) = start_unblocked_group_tasks(
                    pool,
                    deployment.clone(),
                    group_id,
                    &sets,
                ).await {
                    tracing::error!(
                        "Failed to auto-start tasks for group {}: {}",
                        group_id, e
                    );
                    // Surface the error as a group event so it's visible in the UI
                    let err_event = CreateGroupEvent {
                        task_group_id: group_id,
                        task_id: None,
                        event_type: "execution_error".to_string(),
                        actor_type: "system".to_string(),
                        summary: format!("Failed to start tasks: {}", e),
                        payload: Some(serde_json::json!({ "error": e.to_string() }).to_string()),
                    };
                    if let Ok(evt) = GroupEvent::create(pool, &err_event).await {
                        deployment
                            .events()
                            .msg_store()
                            .push_patch(group_event_patch::add(&evt));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Start unblocked tasks in an executing group.
/// Called when a group first enters "executing" (with the DAG's parallel sets)
/// and when a task completes (satisfying dependencies for the next set).
async fn start_unblocked_group_tasks(
    pool: &sqlx::PgPool,
    deployment: DeploymentImpl,
    group_id: Uuid,
    parallel_sets: &[Vec<String>],
) -> Result<(), anyhow::Error> {
    // Collect all task IDs from the DAG
    let all_task_ids: Vec<Uuid> = parallel_sets
        .iter()
        .flatten()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();

    if all_task_ids.is_empty() {
        return Ok(());
    }

    // Find project → board → workflow start column
    let first_task = Task::find_by_id(pool, all_task_ids[0])
        .await?
        .ok_or_else(|| anyhow::anyhow!("Task {} not found", all_task_ids[0]))?;

    let project = Project::find_by_id(pool, first_task.project_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Project not found"))?;

    let board_id = project
        .board_id
        .ok_or_else(|| anyhow::anyhow!("Project has no board configured"))?;

    let start_column = KanbanColumn::find_workflow_start(pool, board_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No workflow start column found for board"))?;

    let agent_id = start_column.agent_id.ok_or_else(|| {
        anyhow::anyhow!(
            "No agent configured on workflow start column '{}' — assign an agent to this column in board settings",
            start_column.name
        )
    })?;
    let agent = Agent::find_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Agent {} not found for start column '{}'", agent_id, start_column.name))?;

    let mut started = 0u32;
    for &task_id in &all_task_ids {
        // Check if this task has unsatisfied dependencies
        if TaskDependency::has_unsatisfied(pool, task_id).await.unwrap_or(true) {
            continue; // Still blocked by earlier set
        }

        let Some(task) = Task::find_by_id(pool, task_id).await? else {
            continue;
        };

        // Skip tasks that are already done or cancelled
        if task.status.to_string() == "done" || task.status.to_string() == "cancelled" {
            continue;
        }

        // Skip tasks that already have a running execution
        if Task::has_active_attempt(pool, task_id).await.unwrap_or(false) {
            continue;
        }

        // Move task to the workflow start column if not already there
        if task.column_id != Some(start_column.id) {
            let status_str = start_column.status.to_string();
            sqlx::query("UPDATE tasks SET column_id = $1, status = $2 WHERE id = $3")
                .bind(start_column.id)
                .bind(&status_str)
                .bind(task_id)
                .execute(pool)
                .await?;
        }

        // Re-fetch the task after column update and broadcast patch
        let task = Task::find_by_id(pool, task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task disappeared after update"))?;
        emit_task_patch(&deployment, task.clone());

        // Spawn agent execution
        match spawn_agent_execution(
            deployment.clone(),
            task,
            agent.clone(),
            &start_column,
        ).await {
            Ok(()) => {
                started += 1;
                tracing::info!(
                    "Auto-started task {} in group {} (column '{}')",
                    task_id, group_id, start_column.name
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to auto-start task {} in group {}: {}",
                    task_id, group_id, e
                );
                return Err(anyhow::anyhow!(
                    "Failed to start task '{}': {}",
                    task_id, e
                ));
            }
        }
    }

    if started > 0 {
        // Log event for the auto-start
        let event = CreateGroupEvent {
            task_group_id: group_id,
            task_id: None,
            event_type: "tasks_auto_started".to_string(),
            actor_type: "system".to_string(),
            summary: format!("{} task{} auto-started in group", started, if started == 1 { "" } else { "s" }),
            payload: Some(serde_json::json!({ "started_count": started }).to_string()),
        };
        if let Ok(evt) = GroupEvent::create(pool, &event).await {
            deployment
                .events()
                .msg_store()
                .push_patch(group_event_patch::add(&evt));
        }
    }

    Ok(())
}

/// Create a full workspace with git worktree for task execution.
/// This sets `task_groups.workspace_id` and creates WorkspaceRepo entries.
/// Used only when groups enter the `executing` state.
async fn create_group_workspace(
    pool: &sqlx::PgPool,
    _deployment: DeploymentImpl,
    group: &TaskGroup,
    group_name: &str,
) -> Result<db::models::workspace::Workspace, anyhow::Error> {
    // Generate a branch name for the group workspace
    let workspace_id = uuid::Uuid::new_v4();
    let short_id = utils::text::short_uuid(&workspace_id);
    let name_slug = utils::text::git_branch_id(group_name);
    let branch_name = format!("group/{}-{}", short_id, name_slug);

    // Create the workspace via TaskGroup (links workspace_id on the group)
    let workspace = TaskGroup::create_workspace(pool, group.id, &branch_name)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create group workspace: {}", e))?;

    // Create WorkspaceRepo entries using the group's project_id directly
    let repos = ProjectRepo::find_repos_for_project(pool, group.project_id).await?;
    if !repos.is_empty() {
        let git_service = GitService {};
        let workspace_repos: Vec<CreateWorkspaceRepo> = repos
            .iter()
            .map(|repo| {
                let target_branch = git_service
                    .get_current_branch(&repo.path)
                    .unwrap_or_else(|_| "main".to_string());
                CreateWorkspaceRepo {
                    repo_id: repo.id,
                    target_branch,
                }
            })
            .collect();

        WorkspaceRepo::create_many(pool, workspace.id, &workspace_repos).await?;
        tracing::info!(
            "Created {} workspace repo(s) for group workspace {}",
            workspace_repos.len(),
            workspace.id
        );
    }

    Ok(workspace)
}

/// Get or create the project's persistent lightweight workspace for a column-level agent.
/// Returns (workspace, was_newly_created).
async fn get_or_create_column_agent_workspace(
    pool: &sqlx::PgPool,
    project: &Project,
    existing_workspace_id: Option<Uuid>,
    agent_executor: &str,
    task_title: &str,
    branch_prefix: &str,
) -> Result<(db::models::workspace::Workspace, bool), anyhow::Error> {
    use db::models::session::{CreateSession, Session};
    use db::models::workspace::{CreateWorkspace, Workspace};

    // If workspace already exists and is valid, return it
    if let Some(ws_id) = existing_workspace_id {
        if let Some(ws) = Workspace::find_by_id(pool, ws_id).await? {
            return Ok((ws, false));
        }
    }

    // Get the project repo path for container_ref
    let repos = ProjectRepo::find_repos_for_project(pool, project.id).await?;
    let repo_path = repos
        .first()
        .map(|r| r.path.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    // Create a hidden system task to anchor the workspace
    let task = db::models::task::Task::create(
        pool,
        &CreateTask {
            project_id: project.id,
            title: task_title.to_string(),
            description: Some("System workspace for column-level agent".to_string()),
            status: None,
            column_id: None,
            parent_workspace_id: None,
            image_ids: None,
            shared_task_id: None,
            task_group_id: None,
        },
        Uuid::new_v4(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to create system task: {}", e))?;

    // Create lightweight workspace (no git worktree)
    let workspace_id = Uuid::new_v4();
    let short_id = utils::text::short_uuid(&workspace_id);
    let branch_name = format!("{}/{}", branch_prefix, short_id);
    let create_data = CreateWorkspace {
        branch: branch_name,
        agent_working_dir: None,
    };
    Workspace::create(pool, &create_data, workspace_id, task.id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create workspace: {}", e))?;

    // Pre-set container_ref so launch_agent_in_workspace doesn't create a worktree
    Workspace::update_container_ref(pool, workspace_id, &repo_path).await?;

    // Create a session so the panel can open immediately (even before first execution)
    Session::create(
        pool,
        &CreateSession {
            executor: Some(agent_executor.to_string()),
        },
        Uuid::new_v4(),
        workspace_id,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to create session: {}", e))?;

    let workspace = Workspace::find_by_id(pool, workspace_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Workspace not found after creation"))?;

    Ok((workspace, true))
}

/// Launch the Group Evaluator column worker.
/// The agent scans for ALL groups in "analyzing" state and processes them one-by-one.
/// Uses the project's persistent group_evaluator workspace (creating it on first use).
async fn launch_group_evaluator(
    pool: &sqlx::PgPool,
    deployment: &DeploymentImpl,
    project_id: Uuid,
) -> Result<(), anyhow::Error> {
    // Fetch the Group Evaluator agent
    let agent = Agent::find_by_id(pool, GROUP_EVALUATOR_AGENT_ID)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Group Evaluator agent not found"))?;

    // Get the project
    let project = Project::find_by_id(pool, project_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Project not found"))?;

    // Build the column worker prompt (scans for all analyzing groups)
    let prompt = group_evaluator::build_evaluator_prompt(&project);

    // Get or create the persistent workspace for the Group Evaluator
    let (workspace, newly_created) = get_or_create_column_agent_workspace(
        pool,
        &project,
        project.group_evaluator_workspace_id.map(|id| id),
        &agent.executor,
        &format!("Group Evaluator — {}", project.name),
        "group-evaluator",
    )
    .await?;

    // Store workspace on project if newly created
    if newly_created {
        if let Err(e) = Project::set_group_evaluator_workspace_id(pool, project.id, workspace.id).await {
            tracing::warn!("Failed to store group_evaluator_workspace_id on project: {}", e);
        }
        // Broadcast project update
        if let Ok(Some(updated_project)) = Project::find_by_id(pool, project.id).await {
            deployment
                .events()
                .msg_store()
                .push_patch(services::services::events::project_patch::replace(&updated_project));
        }
    }

    // Parse executor from agent config
    let base_agent = BaseCodingAgent::from_str(&agent.executor)
        .map_err(|e| anyhow::anyhow!("Failed to parse executor '{}': {}", agent.executor, e))?;
    let executor_profile_id = ExecutorProfileId::new(base_agent);

    // Build agent context — system prompt + group task as start_command
    let agent_context = AgentContext {
        system_prompt: Some(agent.system_prompt.clone()),
        workflow_history: None,
        start_command: Some(prompt),
        deliverable: None,
        name: agent.name.clone(),
        color: agent.color.clone(),
        column_name: "Analysis".to_string(),
        project_context: None,
        task_id_override: None,
    };

    // Launch via the standard execution pipeline (creates a new session in the persistent workspace)
    deployment
        .container()
        .launch_agent_in_workspace(&workspace, executor_profile_id, agent_context)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to launch evaluator agent: {}", e))?;

    tracing::info!(
        "Group Evaluator column worker launched for project {} in workspace {}",
        project_id,
        workspace.id
    );

    Ok(())
}

/// Called when a task completes — check if there are newly unblocked tasks in its group
pub async fn check_and_start_next_group_tasks(
    pool: &sqlx::PgPool,
    deployment: DeploymentImpl,
    completed_task_id: Uuid,
) -> Result<(), anyhow::Error> {
    // Find the task's group
    let task = Task::find_by_id(pool, completed_task_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

    let Some(group_id) = task.task_group_id else {
        return Ok(()); // Task is not in a group
    };

    let group = TaskGroup::find_by_id(pool, group_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Group not found"))?;

    // Only auto-start in executing groups
    if group.state != "executing" {
        return Ok(());
    }

    // Parse the DAG
    let Some(dag_str) = &group.execution_dag else {
        return Ok(()); // No DAG
    };

    #[derive(serde::Deserialize)]
    struct Dag {
        parallel_sets: Vec<Vec<String>>,
    }

    let dag: Dag = serde_json::from_str(dag_str)?;

    start_unblocked_group_tasks(
        pool,
        deployment,
        group_id,
        &dag.parallel_sets,
    ).await
}

use std::path::PathBuf;
use std::str::FromStr;

use anyhow;
use axum::{
    Extension, Json, Router,
    extract::{
        Query, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Json as ResponseJson},
    routing::{delete, get, post, put},
};
use db::models::{
    agent::Agent,
    automation_rule::{AutomationRule, TriggerType},
    image::TaskImage,
    kanban_column::KanbanColumn,
    project::{Project, ProjectError},
    project_repo::ProjectRepo,
    repo::Repo,
    tag::Tag,
    task::{CreateTask, Task, TaskWithAttemptStatus, UpdateTask},
    task_event::{ActorType, CreateTaskEvent, EventTriggerType, TaskEvent},
    workspace::{CreateWorkspace, Workspace},
    workspace_repo::{CreateWorkspaceRepo, WorkspaceRepo},
};
use deployment::Deployment;
use executors::executors::BaseCodingAgent;
use executors::profile::ExecutorProfileId;
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use services::services::{
    container::{AgentContext, ContainerService, build_decision_instructions, read_decision_file},
    git::GitService,
    share::ShareError,
    workspace_manager::WorkspaceManager,
};
use sqlx::Error as SqlxError;
use ts_rs::TS;
use utils::{api::oauth::LoginStatus, response::ApiResponse};
use uuid::Uuid;

use crate::{
    DeploymentImpl, error::ApiError, middleware::load_task_middleware,
    routes::task_attempts::WorkspaceRepoInput,
    routes::debug_events::{emit_debug_event, DebugEvent},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskQuery {
    pub project_id: Uuid,
}

pub async fn get_tasks(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<TaskQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<TaskWithAttemptStatus>>>, ApiError> {
    let tasks =
        Task::find_by_project_id_with_attempt_status(&deployment.db().pool, query.project_id)
            .await?;

    Ok(ResponseJson(ApiResponse::success(tasks)))
}

pub async fn stream_tasks_ws(
    ws: WebSocketUpgrade,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<TaskQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_tasks_ws(socket, deployment, query.project_id).await {
            tracing::warn!("tasks WS closed: {}", e);
        }
    })
}

async fn handle_tasks_ws(
    socket: WebSocket,
    deployment: DeploymentImpl,
    project_id: Uuid,
) -> anyhow::Result<()> {
    // Get the raw stream and convert LogMsg to WebSocket messages
    let mut stream = deployment
        .events()
        .stream_tasks_raw(project_id)
        .await?
        .map_ok(|msg| msg.to_ws_message_unchecked());

    // Split socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Drain (and ignore) any client->server messages so pings/pongs work
    tokio::spawn(async move { while let Some(Ok(_)) = receiver.next().await {} });

    // Forward server messages
    while let Some(item) = stream.next().await {
        match item {
            Ok(msg) => {
                if sender.send(msg).await.is_err() {
                    break; // client disconnected
                }
            }
            Err(e) => {
                tracing::error!("stream error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

pub async fn get_task(
    Extension(task): Extension<Task>,
    State(_deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(task)))
}

pub async fn create_task(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateTask>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    let id = Uuid::new_v4();

    tracing::debug!(
        "Creating task '{}' in project {}",
        payload.title,
        payload.project_id
    );

    let task = Task::create(&deployment.db().pool, &payload, id).await?;

    if let Some(image_ids) = &payload.image_ids {
        TaskImage::associate_many_dedup(&deployment.db().pool, task.id, image_ids).await?;
    }

    deployment
        .track_if_analytics_allowed(
            "task_created",
            serde_json::json!({
            "task_id": task.id.to_string(),
            "project_id": payload.project_id,
            "has_description": task.description.is_some(),
            "has_images": payload.image_ids.is_some(),
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(task)))
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateAndStartTaskRequest {
    pub task: CreateTask,
    pub executor_profile_id: ExecutorProfileId,
    pub repos: Vec<WorkspaceRepoInput>,
}

pub async fn create_task_and_start(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateAndStartTaskRequest>,
) -> Result<ResponseJson<ApiResponse<TaskWithAttemptStatus>>, ApiError> {
    if payload.repos.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one repository is required".to_string(),
        ));
    }

    let pool = &deployment.db().pool;

    let task_id = Uuid::new_v4();
    let task = Task::create(pool, &payload.task, task_id).await?;

    if let Some(image_ids) = &payload.task.image_ids {
        TaskImage::associate_many_dedup(pool, task.id, image_ids).await?;
    }

    deployment
        .track_if_analytics_allowed(
            "task_created",
            serde_json::json!({
                "task_id": task.id.to_string(),
                "project_id": task.project_id,
                "has_description": task.description.is_some(),
                "has_images": payload.task.image_ids.is_some(),
            }),
        )
        .await;

    let project = Project::find_by_id(pool, task.project_id)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;

    let attempt_id = Uuid::new_v4();
    let git_branch_name = deployment
        .container()
        .git_branch_from_workspace(&attempt_id, &task.title)
        .await;

    let agent_working_dir = project
        .default_agent_working_dir
        .as_ref()
        .filter(|dir: &&String| !dir.is_empty())
        .cloned();

    let workspace = Workspace::create(
        pool,
        &CreateWorkspace {
            branch: git_branch_name,
            agent_working_dir,
        },
        attempt_id,
        task.id,
    )
    .await?;

    let workspace_repos: Vec<CreateWorkspaceRepo> = payload
        .repos
        .iter()
        .map(|r| CreateWorkspaceRepo {
            repo_id: r.repo_id,
            target_branch: r.target_branch.clone(),
        })
        .collect();
    WorkspaceRepo::create_many(&deployment.db().pool, workspace.id, &workspace_repos).await?;

    let is_attempt_running = deployment
        .container()
        .start_workspace(&workspace, payload.executor_profile_id.clone())
        .await
        .inspect_err(|err| tracing::error!("Failed to start task attempt: {}", err))
        .is_ok();
    deployment
        .track_if_analytics_allowed(
            "task_attempt_started",
            serde_json::json!({
                "task_id": task.id.to_string(),
                "executor": &payload.executor_profile_id.executor,
                "variant": &payload.executor_profile_id.variant,
                "workspace_id": workspace.id.to_string(),
            }),
        )
        .await;

    let task = Task::find_by_id(pool, task.id)
        .await?
        .ok_or(ApiError::Database(SqlxError::RowNotFound))?;

    tracing::info!("Started attempt for task {}", task.id);
    Ok(ResponseJson(ApiResponse::success(TaskWithAttemptStatus {
        task,
        has_in_progress_attempt: is_attempt_running,
        last_attempt_failed: false,
        executor: payload.executor_profile_id.executor.to_string(),
        latest_attempt_id: Some(workspace.id),
    })))
}

pub async fn update_task(
    Extension(existing_task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,

    Json(payload): Json<UpdateTask>,
) -> Result<ResponseJson<ApiResponse<Task>>, ApiError> {
    ensure_shared_task_auth(&existing_task, &deployment).await?;

    let pool = &deployment.db().pool;

    // Use existing values if not provided in update
    let title = payload.title.unwrap_or(existing_task.title.clone());
    let description = match payload.description {
        Some(s) if s.trim().is_empty() => None, // Empty string = clear description
        Some(s) => Some(s),                     // Non-empty string = update description
        None => existing_task.description.clone(), // Field omitted = keep existing
    };
    let status = payload.status.unwrap_or(existing_task.status.clone());
    let column_id = payload.column_id.or(existing_task.column_id);
    let parent_workspace_id = payload
        .parent_workspace_id
        .or(existing_task.parent_workspace_id);

    // Check if moving to a new column
    let is_column_changing = payload.column_id.is_some() && payload.column_id != existing_task.column_id;

    if is_column_changing {
        if let Some(target_column_id) = payload.column_id {
            // Get the target column to check if it starts a workflow
            if let Some(target_column) = KanbanColumn::find_by_id(pool, target_column_id).await? {
                // Check if task is currently in a backlog (is_initial) column
                // Backlog tasks can only move to cancelled or starts_workflow columns
                if let Some(current_column_id) = existing_task.column_id {
                    if let Some(current_column) = KanbanColumn::find_by_id(pool, current_column_id).await? {
                        if current_column.is_initial {
                            // Task is in backlog - restrict where it can go
                            // Can only move to: terminal+cancelled status OR starts_workflow column
                            use db::models::task::TaskStatus;
                            let is_cancelled_column = target_column.is_terminal && target_column.status == TaskStatus::Cancelled;
                            let is_workflow_start = target_column.starts_workflow;

                            if !is_cancelled_column && !is_workflow_start {
                                return Err(ApiError::BadRequest(
                                    format!(
                                        "Tasks in '{}' can only move to 'Cancelled' or a workflow-starting column. \
                                         Use the Start or Cancel buttons.",
                                        current_column.name
                                    )
                                ));
                            }
                        }
                    }
                }

                // If moving TO a backlog column, ensure no active attempt
                // Backlog tasks should never have active executions
                if target_column.is_initial {
                    let has_active = Task::has_active_attempt(pool, existing_task.id).await?;
                    if has_active {
                        return Err(ApiError::Conflict(
                            "Cannot move task to backlog: an execution is still running. \
                             Stop the execution or wait for it to complete first.".to_string()
                        ));
                    }
                }

                // If moving to a workflow-starting column, check for active attempts
                // This prevents accidentally creating a new attempt while one is running
                if target_column.starts_workflow {
                    let has_active = Task::has_active_attempt(pool, existing_task.id).await?;
                    if has_active {
                        return Err(ApiError::Conflict(
                            "Cannot move task: an execution is already running. \
                             Wait for the current execution to complete or stop it first.".to_string()
                        ));
                    }
                }
            }
        }
    }

    let task = Task::update(
        pool,
        existing_task.id,
        existing_task.project_id,
        title,
        description,
        status,
        column_id,
        parent_workspace_id,
    )
    .await?;

    if let Some(image_ids) = &payload.image_ids {
        TaskImage::delete_by_task_id(pool, task.id).await?;
        TaskImage::associate_many_dedup(pool, task.id, image_ids).await?;
    }

    // Trigger automation rules on column change
    if is_column_changing {
        // Record column transition event
        if let Some(new_column_id) = payload.column_id {
            let event = CreateTaskEvent::column_transition(
                task.id,
                existing_task.column_id,
                new_column_id,
                EventTriggerType::DragDrop, // User-initiated column change
                ActorType::User,
                None,
            );
            if let Err(e) = TaskEvent::create(pool, &event).await {
                tracing::error!("Failed to record column transition event: {}", e);
            }
        }

        // Fire OnExit rules for the old column
        if let Some(old_column_id) = existing_task.column_id {
            let exit_rules = AutomationRule::find_triggered_rules(pool, old_column_id, TriggerType::OnExit).await?;
            for rule in exit_rules {
                tracing::info!(
                    "Automation triggered: OnExit rule '{}' (action: {}) for task {} leaving column {}",
                    rule.name.as_deref().unwrap_or("unnamed"),
                    rule.action_type,
                    task.id,
                    old_column_id
                );
                // TODO: Execute automation action
            }
        }

        // Fire OnEnter rules for the new column
        if let Some(new_column_id) = payload.column_id {
            let enter_rules = AutomationRule::find_triggered_rules(pool, new_column_id, TriggerType::OnEnter).await?;
            for rule in enter_rules {
                tracing::info!(
                    "Automation triggered: OnEnter rule '{}' (action: {}) for task {} entering column {}",
                    rule.name.as_deref().unwrap_or("unnamed"),
                    rule.action_type,
                    task.id,
                    new_column_id
                );
                // TODO: Execute automation action
            }

            // Auto-start agent execution if column has an assigned agent
            if let Some(new_column) = KanbanColumn::find_by_id(pool, new_column_id).await? {
                // Get old column name for debug event
                let old_column_name = if let Some(old_id) = existing_task.column_id {
                    KanbanColumn::find_by_id(pool, old_id).await.ok().flatten().map(|c| c.name)
                } else {
                    None
                };

                // Get agent name if assigned
                let agent_info = if let Some(aid) = new_column.agent_id {
                    Agent::find_by_id(pool, aid).await.ok().flatten().map(|a| a.name)
                } else {
                    None
                };

                // Emit debug event for column change
                emit_debug_event(DebugEvent::TaskColumnChanged {
                    task_id: task.id.to_string(),
                    task_title: task.title.clone(),
                    from_column: old_column_name,
                    to_column: new_column.name.clone(),
                    column_has_agent: new_column.agent_id.is_some(),
                    agent_name: agent_info.clone(),
                });

                tracing::info!(
                    "Task {} entered column '{}' (id: {}, starts_workflow: {}, agent_id: {:?})",
                    task.id,
                    new_column.name,
                    new_column.id,
                    new_column.starts_workflow,
                    new_column.agent_id
                );
                if let Some(agent_id) = new_column.agent_id {
                    // Check if there's already an ACTIVE (running) execution - don't start another
                    // Note: This allows starting a new execution after the previous one completes,
                    // which is the intended behavior for agent-to-agent handoff via column transitions.
                    // The workspace will be reused (not recreated) to maintain continuity.
                    let has_running = Task::has_active_attempt(pool, task.id).await.unwrap_or(false);
                    if has_running {
                        tracing::debug!(
                            "Skipping auto-start for task {} - execution already running",
                            task.id
                        );
                        // Don't start another execution while one is running
                    } else {
                        // Fetch the agent to get its context
                        match Agent::find_by_id(pool, agent_id).await {
                            Ok(Some(agent)) => {
                                if let Err(e) = spawn_agent_execution(
                                    deployment.clone(),
                                    task.clone(),
                                    agent,
                                    &new_column,
                                ).await {
                                    tracing::error!(
                                        "Failed to auto-start agent execution for task {} in column {}: {}",
                                        task.id,
                                        new_column_id,
                                        e
                                    );
                                }
                            }
                            Ok(None) => {
                                tracing::warn!("Agent {} not found for column {}", agent_id, new_column.name);
                            }
                            Err(e) => {
                                tracing::error!("Failed to fetch agent {}: {}", agent_id, e);
                            }
                        }
                    }
                } else {
                    tracing::debug!(
                        "No agent assigned to column '{}' - skipping auto-start",
                        new_column.name
                    );
                }
            }
        }
    }

    // If task has been shared, broadcast update
    if task.shared_task_id.is_some() {
        let Ok(publisher) = deployment.share_publisher() else {
            return Err(ShareError::MissingConfig("share publisher unavailable").into());
        };
        publisher.update_shared_task(&task).await?;
    }

    Ok(ResponseJson(ApiResponse::success(task)))
}

async fn ensure_shared_task_auth(
    existing_task: &Task,
    deployment: &local_deployment::LocalDeployment,
) -> Result<(), ApiError> {
    if existing_task.shared_task_id.is_some() {
        match deployment.get_login_status().await {
            LoginStatus::LoggedIn { .. } => return Ok(()),
            LoginStatus::LoggedOut => {
                return Err(ShareError::MissingAuth.into());
            }
        }
    }
    Ok(())
}

pub async fn delete_task(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<(StatusCode, ResponseJson<ApiResponse<()>>), ApiError> {
    ensure_shared_task_auth(&task, &deployment).await?;

    // Validate no running execution processes
    if deployment
        .container()
        .has_running_processes(task.id)
        .await?
    {
        return Err(ApiError::Conflict("Task has running execution processes. Please wait for them to complete or stop them first.".to_string()));
    }

    let pool = &deployment.db().pool;

    // Gather task attempts data needed for background cleanup
    let attempts = Workspace::fetch_all(pool, Some(task.id))
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch task attempts for task {}: {}", task.id, e);
            ApiError::Workspace(e)
        })?;

    let repositories = WorkspaceRepo::find_unique_repos_for_task(pool, task.id).await?;

    // Collect workspace directories that need cleanup
    let workspace_dirs: Vec<PathBuf> = attempts
        .iter()
        .filter_map(|attempt| attempt.container_ref.as_ref().map(PathBuf::from))
        .collect();

    if let Some(shared_task_id) = task.shared_task_id {
        let Ok(publisher) = deployment.share_publisher() else {
            return Err(ShareError::MissingConfig("share publisher unavailable").into());
        };
        publisher.delete_shared_task(shared_task_id).await?;
    }

    // Use a transaction to ensure atomicity: either all operations succeed or all are rolled back
    let mut tx = pool.begin().await?;

    // Nullify parent_workspace_id for all child tasks before deletion
    // This breaks parent-child relationships to avoid foreign key constraint violations
    let mut total_children_affected = 0u64;
    for attempt in &attempts {
        let children_affected =
            Task::nullify_children_by_workspace_id(&mut *tx, attempt.id).await?;
        total_children_affected += children_affected;
    }

    // Delete task from database (FK CASCADE will handle task_attempts)
    let rows_affected = Task::delete(&mut *tx, task.id).await?;

    if rows_affected == 0 {
        return Err(ApiError::Database(SqlxError::RowNotFound));
    }

    // Commit the transaction - if this fails, all changes are rolled back
    tx.commit().await?;

    if total_children_affected > 0 {
        tracing::info!(
            "Nullified {} child task references before deleting task {}",
            total_children_affected,
            task.id
        );
    }

    deployment
        .track_if_analytics_allowed(
            "task_deleted",
            serde_json::json!({
                "task_id": task.id.to_string(),
                "project_id": task.project_id.to_string(),
                "attempt_count": attempts.len(),
            }),
        )
        .await;

    let task_id = task.id;
    let pool = pool.clone();
    tokio::spawn(async move {
        tracing::info!(
            "Starting background cleanup for task {} ({} workspaces, {} repos)",
            task_id,
            workspace_dirs.len(),
            repositories.len()
        );

        for workspace_dir in &workspace_dirs {
            if let Err(e) = WorkspaceManager::cleanup_workspace(workspace_dir, &repositories).await
            {
                tracing::error!(
                    "Background workspace cleanup failed for task {} at {}: {}",
                    task_id,
                    workspace_dir.display(),
                    e
                );
            }
        }

        match Repo::delete_orphaned(&pool).await {
            Ok(count) if count > 0 => {
                tracing::info!("Deleted {} orphaned repo records", count);
            }
            Err(e) => {
                tracing::error!("Failed to delete orphaned repos: {}", e);
            }
            _ => {}
        }

        tracing::info!("Background cleanup completed for task {}", task_id);
    });

    // Return 202 Accepted to indicate deletion was scheduled
    Ok((StatusCode::ACCEPTED, ResponseJson(ApiResponse::success(()))))
}

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct ShareTaskResponse {
    pub shared_task_id: Uuid,
}

pub async fn share_task(
    Extension(task): Extension<Task>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<ShareTaskResponse>>, ApiError> {
    let Ok(publisher) = deployment.share_publisher() else {
        return Err(ShareError::MissingConfig("share publisher unavailable").into());
    };
    let profile = deployment
        .auth_context()
        .cached_profile()
        .await
        .ok_or(ShareError::MissingAuth)?;
    let shared_task_id = publisher.share_task(task.id, profile.user_id).await?;

    let props = serde_json::json!({
        "task_id": task.id,
        "shared_task_id": shared_task_id,
    });
    deployment
        .track_if_analytics_allowed("start_sharing_task", props)
        .await;

    Ok(ResponseJson(ApiResponse::success(ShareTaskResponse {
        shared_task_id,
    })))
}

/// Spawn agent execution for a task when entering a column with an assigned agent
async fn spawn_agent_execution(
    deployment: DeploymentImpl,
    task: Task,
    agent: Agent,
    column: &KanbanColumn,
) -> Result<(), anyhow::Error> {
    let column_id = column.id;
    let board_id = column.board_id;
    let column_name = column.name.clone();
    tracing::info!(
        "spawn_agent_execution called - task: {}, agent: {} ({}), column: '{}', \
         system_prompt length: {}, start_command: {:?}",
        task.id,
        agent.name,
        agent.id,
        column_name,
        agent.system_prompt.len(),
        agent.start_command.as_ref().map(|s| s.len())
    );
    let pool = &deployment.db().pool;

    // Expand @tagname references in agent start_command and column deliverable
    let expanded_start_command = Tag::expand_tags_optional(pool, agent.start_command.as_deref()).await;
    let expanded_deliverable = Tag::expand_tags_optional(pool, column.deliverable.as_deref()).await;

    // Check for existing active workspace first - reuse if available
    let (workspace, reusing_existing) = if let Some(existing) = Workspace::find_active_for_task(pool, task.id).await? {
        tracing::info!(
            "Reusing existing workspace {} for task {} in column '{}'",
            existing.id,
            task.id,
            column_name
        );
        (existing, true)
    } else {
        // No existing workspace - create a new one
        let project = Project::find_by_id(pool, task.project_id)
            .await?
            .ok_or(ProjectError::ProjectNotFound)?;

        let repos = ProjectRepo::find_repos_for_project(pool, project.id).await?;
        if repos.is_empty() {
            return Err(anyhow::anyhow!("Project has no repositories configured"));
        }

        let git_service = GitService {};
        let mut workspace_repos_to_create: Vec<CreateWorkspaceRepo> = Vec::new();
        for repo in &repos {
            let target_branch = git_service
                .get_current_branch(&repo.path)
                .unwrap_or_else(|_| "main".to_string());
            workspace_repos_to_create.push(CreateWorkspaceRepo {
                repo_id: repo.id,
                target_branch,
            });
        }

        let attempt_id = Uuid::new_v4();
        let git_branch_name = deployment
            .container()
            .git_branch_from_workspace(&attempt_id, &task.title)
            .await;

        let agent_working_dir = project
            .default_agent_working_dir
            .as_ref()
            .filter(|dir: &&String| !dir.is_empty())
            .cloned();

        let new_workspace = Workspace::create(
            pool,
            &CreateWorkspace {
                branch: git_branch_name,
                agent_working_dir,
            },
            attempt_id,
            task.id,
        )
        .await?;

        WorkspaceRepo::create_many(pool, new_workspace.id, &workspace_repos_to_create).await?;

        tracing::info!(
            "Created new workspace {} for task {} in column '{}'",
            new_workspace.id,
            task.id,
            column_name
        );
        (new_workspace, false)
    };

    // Emit debug event for attempt creation
    emit_debug_event(DebugEvent::AttemptCreated {
        task_id: task.id.to_string(),
        workspace_id: workspace.id.to_string(),
        branch: workspace.branch.clone(),
        reusing_existing,
    });

    // Parse the executor from agent.executor (e.g., "CLAUDE_CODE")
    let base_agent = BaseCodingAgent::from_str(&agent.executor).map_err(|e| {
        anyhow::anyhow!("Failed to parse executor '{}': {}", agent.executor, e)
    })?;
    let executor_profile_id = ExecutorProfileId::new(base_agent);

    // Read existing decision file for any feedback from prior rejection
    let existing_decision = read_decision_file(&workspace).await;

    // Emit debug event for decision file
    if existing_decision.is_some() {
        emit_debug_event(DebugEvent::DecisionFileRead {
            task_id: task.id.to_string(),
            workspace_id: workspace.id.to_string(),
            decision: existing_decision.clone(),
        });
    }

    // Build decision instructions if this column has conditional transitions
    // Uses hierarchical resolution: task -> project -> board
    let decision_instructions = build_decision_instructions(
        pool,
        column_id,
        task.id,
        task.project_id,
        Some(board_id),
        &existing_decision,
    ).await;

    // Combine agent's start_command (with tags expanded) with decision instructions
    let start_command = match (&expanded_start_command, &decision_instructions) {
        (Some(cmd), Some(instructions)) => Some(format!("{}{}", cmd, instructions)),
        (Some(cmd), None) => Some(cmd.clone()),
        (None, Some(instructions)) => Some(instructions.clone()),
        (None, None) => None,
    };

    // Emit debug event for agent starting
    emit_debug_event(DebugEvent::AgentStarting {
        task_id: task.id.to_string(),
        workspace_id: workspace.id.to_string(),
        agent_name: agent.name.clone(),
        executor: agent.executor.clone(),
        system_prompt_length: agent.system_prompt.len(),
        system_prompt_preview: agent.system_prompt.chars().take(200).collect(),
        start_command_length: start_command.as_ref().map(|s| s.len()),
        start_command_preview: start_command.as_ref().map(|s| s.chars().take(200).collect()),
        column_name: column_name.clone(),
    });

    // Build and emit the FULL prompt that will be sent to the agent
    // This replicates the logic in CodingAgentInitialRequest::build_full_prompt()
    let task_prompt = task.to_prompt();
    let mut full_prompt = String::new();
    if !agent.system_prompt.trim().is_empty() {
        full_prompt.push_str(agent.system_prompt.trim());
        full_prompt.push_str("\n\n---\n\n");
    }
    full_prompt.push_str("## Task\n\n");
    full_prompt.push_str(&task_prompt);
    if let Some(cmd) = &start_command {
        if !cmd.trim().is_empty() {
            full_prompt.push_str("\n\n---\n\n## Instructions\n\n");
            full_prompt.push_str(cmd.trim());
        }
    }
    // Add deliverable section - tells the agent what to produce and when to stop
    // The deliverable is defined at the column level (what this stage produces), with tags expanded
    if let Some(deliverable) = &expanded_deliverable {
        if !deliverable.trim().is_empty() {
            full_prompt.push_str("\n\n---\n\n## Expected Deliverable\n\n");
            full_prompt.push_str(deliverable.trim());
            full_prompt.push_str("\n\n**Important**: Once you have produced the deliverable described above, commit your work and stop. Do not proceed to implement the plan yourself - your job is complete when the deliverable is ready.");
        }
    }
    emit_debug_event(DebugEvent::FullPromptBuilt {
        task_id: task.id.to_string(),
        workspace_id: workspace.id.to_string(),
        agent_name: agent.name.clone(),
        full_prompt_length: full_prompt.len(),
        full_prompt,
    });

    // Start workspace with agent context
    // Deliverable comes from the column (what this stage should produce), with tags expanded
    let agent_context = AgentContext {
        system_prompt: Some(agent.system_prompt.clone()),
        start_command,
        deliverable: expanded_deliverable.clone(),
        name: agent.name.clone(),
        color: agent.color.clone(),
        column_name: column_name.clone(),
    };
    deployment
        .container()
        .start_workspace_with_agent_context(
            &workspace,
            executor_profile_id.clone(),
            agent_context,
        )
        .await
        .inspect_err(|err| {
            emit_debug_event(DebugEvent::Error {
                message: format!("Failed to start agent execution: {}", err),
                context: Some(serde_json::json!({
                    "task_id": task.id.to_string(),
                    "workspace_id": workspace.id.to_string(),
                    "agent_name": agent.name,
                })),
            });
            tracing::error!("Failed to start agent execution: {}", err);
        })?;

    // Record agent start event
    let agent_event = CreateTaskEvent {
        task_id: task.id,
        event_type: db::models::task_event::TaskEventType::AgentStart,
        from_column_id: None,
        to_column_id: None,
        workspace_id: Some(workspace.id),
        session_id: None, // Session created inside start_workspace_with_agent_context
        executor: Some(executor_profile_id.to_string()),
        automation_rule_id: None,
        trigger_type: Some(EventTriggerType::Automation),
        commit_hash: None,
        commit_message: None,
        metadata: None,
        actor_type: Some(ActorType::System),
        actor_id: None,
    };
    if let Err(e) = TaskEvent::create(pool, &agent_event).await {
        tracing::error!("Failed to record agent start event: {}", e);
    }

    tracing::info!(
        "Auto-started agent '{}' for task {} in workspace {}",
        agent.name,
        task.id,
        workspace.id
    );

    Ok(())
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let task_actions_router = Router::new()
        .route("/", put(update_task))
        .route("/", delete(delete_task))
        .route("/share", post(share_task));

    let task_id_router = Router::new()
        .route("/", get(get_task))
        .merge(task_actions_router)
        .layer(from_fn_with_state(deployment.clone(), load_task_middleware));

    let inner = Router::new()
        .route("/", get(get_tasks).post(create_task))
        .route("/stream/ws", get(stream_tasks_ws))
        .route("/create-and-start", post(create_task_and_start))
        .nest("/{task_id}", task_id_router);

    // mount under /projects/:project_id/tasks
    Router::new().nest("/tasks", inner)
}

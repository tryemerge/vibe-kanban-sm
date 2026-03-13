use std::path::PathBuf;

use anyhow;
use axum::{
    Extension, Json, Router,
    extract::{
        Path, Query, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{IntoResponse, Json as ResponseJson},
    routing::{get, post},
};
use db::models::{
    project::{CreateProject, Project, ProjectError, SearchResult, UpdateProject},
    project_repo::{CreateProjectRepo, ProjectRepo, UpdateProjectRepo},
    repo::Repo,
};
use deployment::Deployment;
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use services::services::{
    container::ContainerService,
    events::project_patch,
    file_search_cache::SearchQuery, project::ProjectServiceError,
    remote_client::CreateRemoteProjectPayload,
};
use ts_rs::TS;
use utils::{
    api::projects::{RemoteProject, RemoteProjectMembersResponse},
    response::ApiResponse,
};
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::load_project_middleware};

#[derive(Deserialize, TS)]
pub struct LinkToExistingRequest {
    pub remote_project_id: Uuid,
}

#[derive(Deserialize, TS)]
pub struct CreateRemoteProjectRequest {
    pub organization_id: Uuid,
    pub name: String,
}

pub async fn get_projects(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<Project>>>, ApiError> {
    let projects = Project::find_all(&deployment.db().pool).await?;
    Ok(ResponseJson(ApiResponse::success(projects)))
}

pub async fn stream_projects_ws(
    ws: WebSocketUpgrade,
    State(deployment): State<DeploymentImpl>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_projects_ws(socket, deployment).await {
            tracing::warn!("projects WS closed: {}", e);
        }
    })
}

async fn handle_projects_ws(socket: WebSocket, deployment: DeploymentImpl) -> anyhow::Result<()> {
    let mut stream = deployment
        .events()
        .stream_projects_raw()
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

pub async fn get_project(
    Extension(project): Extension<Project>,
) -> Result<ResponseJson<ApiResponse<Project>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(project)))
}

pub async fn link_project_to_existing_remote(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<LinkToExistingRequest>,
) -> Result<ResponseJson<ApiResponse<Project>>, ApiError> {
    let client = deployment.remote_client()?;

    let remote_project = client.get_project(payload.remote_project_id).await?;

    let updated_project = apply_remote_project_link(&deployment, project, remote_project).await?;

    Ok(ResponseJson(ApiResponse::success(updated_project)))
}

pub async fn create_and_link_remote_project(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateRemoteProjectRequest>,
) -> Result<ResponseJson<ApiResponse<Project>>, ApiError> {
    let repo_name = payload.name.trim().to_string();
    if repo_name.trim().is_empty() {
        return Err(ApiError::Conflict(
            "Remote project name cannot be empty.".to_string(),
        ));
    }

    let client = deployment.remote_client()?;

    let remote_project = client
        .create_project(&CreateRemoteProjectPayload {
            organization_id: payload.organization_id,
            name: repo_name,
            metadata: None,
        })
        .await?;

    let updated_project = apply_remote_project_link(&deployment, project, remote_project).await?;

    Ok(ResponseJson(ApiResponse::success(updated_project)))
}

pub async fn unlink_project(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Project>>, ApiError> {
    let updated_project = deployment
        .project()
        .unlink_from_remote(&deployment.db().pool, &project)
        .await?;

    // Broadcast project update via WebSocket
    deployment
        .events()
        .msg_store()
        .push_patch(project_patch::replace(&updated_project));

    Ok(ResponseJson(ApiResponse::success(updated_project)))
}

pub async fn get_remote_project_by_id(
    State(deployment): State<DeploymentImpl>,
    Path(remote_project_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<RemoteProject>>, ApiError> {
    let client = deployment.remote_client()?;

    let remote_project = client.get_project(remote_project_id).await?;

    Ok(ResponseJson(ApiResponse::success(remote_project)))
}

pub async fn get_project_remote_members(
    State(deployment): State<DeploymentImpl>,
    Extension(project): Extension<Project>,
) -> Result<ResponseJson<ApiResponse<RemoteProjectMembersResponse>>, ApiError> {
    let remote_project_id = project.remote_project_id.ok_or_else(|| {
        ApiError::Conflict("Project is not linked to a remote project".to_string())
    })?;

    let client = deployment.remote_client()?;

    let remote_project = client.get_project(remote_project_id).await?;
    let members = client
        .list_members(remote_project.organization_id)
        .await?
        .members;

    Ok(ResponseJson(ApiResponse::success(
        RemoteProjectMembersResponse {
            organization_id: remote_project.organization_id,
            members,
        },
    )))
}

async fn apply_remote_project_link(
    deployment: &DeploymentImpl,
    project: Project,
    remote_project: RemoteProject,
) -> Result<Project, ApiError> {
    if project.remote_project_id.is_some() {
        return Err(ApiError::Conflict(
            "Project is already linked to a remote project. Unlink it first.".to_string(),
        ));
    }

    let updated_project = deployment
        .project()
        .link_to_remote(&deployment.db().pool, project.id, remote_project)
        .await?;

    // Broadcast project update via WebSocket
    deployment
        .events()
        .msg_store()
        .push_patch(project_patch::replace(&updated_project));

    deployment
        .track_if_analytics_allowed(
            "project_linked_to_remote",
            serde_json::json!({
                "project_id": project.id.to_string(),
            }),
        )
        .await;

    Ok(updated_project)
}

pub async fn create_project(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateProject>,
) -> Result<ResponseJson<ApiResponse<Project>>, ApiError> {
    tracing::debug!("Creating project '{}'", payload.name);
    let repo_count = payload.repositories.len();

    match deployment
        .project()
        .create_project(&deployment.db().pool, deployment.repo(), payload)
        .await
    {
        Ok(project) => {
            // Broadcast project creation via WebSocket
            deployment
                .events()
                .msg_store()
                .push_patch(project_patch::add(&project));

            // Track project creation event
            deployment
                .track_if_analytics_allowed(
                    "project_created",
                    serde_json::json!({
                        "project_id": project.id.to_string(),
                        "repository_count": repo_count,
                        "trigger": "manual",
                    }),
                )
                .await;

            Ok(ResponseJson(ApiResponse::success(project)))
        }
        Err(ProjectServiceError::DuplicateGitRepoPath) => Ok(ResponseJson(ApiResponse::error(
            "Duplicate repository path provided",
        ))),
        Err(ProjectServiceError::DuplicateRepositoryName) => Ok(ResponseJson(ApiResponse::error(
            "Duplicate repository name provided",
        ))),
        Err(ProjectServiceError::PathNotFound(_)) => Ok(ResponseJson(ApiResponse::error(
            "The specified path does not exist",
        ))),
        Err(ProjectServiceError::PathNotDirectory(_)) => Ok(ResponseJson(ApiResponse::error(
            "The specified path is not a directory",
        ))),
        Err(ProjectServiceError::NotGitRepository(_)) => Ok(ResponseJson(ApiResponse::error(
            "The specified directory is not a git repository",
        ))),
        Err(e) => Err(ProjectError::CreateFailed(e.to_string()).into()),
    }
}

pub async fn update_project(
    Extension(existing_project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateProject>,
) -> Result<ResponseJson<ApiResponse<Project>>, StatusCode> {
    match deployment
        .project()
        .update_project(&deployment.db().pool, &existing_project, payload)
        .await
    {
        Ok(project) => {
            // Broadcast project update via WebSocket
            deployment
                .events()
                .msg_store()
                .push_patch(project_patch::replace(&project));

            Ok(ResponseJson(ApiResponse::success(project)))
        }
        Err(e) => {
            tracing::error!("Failed to update project: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_project(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, StatusCode> {
    let project_id = project.id;
    match deployment
        .project()
        .delete_project(&deployment.db().pool, project_id)
        .await
    {
        Ok(rows_affected) => {
            if rows_affected == 0 {
                Err(StatusCode::NOT_FOUND)
            } else {
                // Broadcast project deletion via WebSocket
                deployment
                    .events()
                    .msg_store()
                    .push_patch(project_patch::remove(project_id));

                deployment
                    .track_if_analytics_allowed(
                        "project_deleted",
                        serde_json::json!({
                            "project_id": project_id.to_string(),
                        }),
                    )
                    .await;

                Ok(ResponseJson(ApiResponse::success(())))
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete project: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(serde::Deserialize)]
pub struct OpenEditorRequest {
    editor_type: Option<String>,
    git_repo_path: Option<PathBuf>,
}

#[derive(Debug, serde::Serialize, ts_rs::TS)]
pub struct OpenEditorResponse {
    pub url: Option<String>,
}

pub async fn open_project_in_editor(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<Option<OpenEditorRequest>>,
) -> Result<ResponseJson<ApiResponse<OpenEditorResponse>>, ApiError> {
    let path = if let Some(ref req) = payload
        && let Some(ref specified_path) = req.git_repo_path
    {
        specified_path.clone()
    } else {
        let repositories = deployment
            .project()
            .get_repositories(&deployment.db().pool, project.id)
            .await?;

        repositories
            .first()
            .map(|r| r.path.clone())
            .ok_or_else(|| ApiError::BadRequest("Project has no repositories".to_string()))?
    };

    let editor_config = {
        let config = deployment.config().read().await;
        let editor_type_str = payload.as_ref().and_then(|req| req.editor_type.as_deref());
        config.editor.with_override(editor_type_str)
    };

    match editor_config.open_file(&path).await {
        Ok(url) => {
            tracing::info!(
                "Opened editor for project {} at path: {}{}",
                project.id,
                path.to_string_lossy(),
                if url.is_some() { " (remote mode)" } else { "" }
            );

            deployment
                .track_if_analytics_allowed(
                    "project_editor_opened",
                    serde_json::json!({
                        "project_id": project.id.to_string(),
                        "editor_type": payload.as_ref().and_then(|req| req.editor_type.as_ref()),
                        "remote_mode": url.is_some(),
                    }),
                )
                .await;

            Ok(ResponseJson(ApiResponse::success(OpenEditorResponse {
                url,
            })))
        }
        Err(e) => {
            tracing::error!("Failed to open editor for project {}: {:?}", project.id, e);
            Err(ApiError::EditorOpen(e))
        }
    }
}

pub async fn search_project_files(
    State(deployment): State<DeploymentImpl>,
    Extension(project): Extension<Project>,
    Query(search_query): Query<SearchQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<SearchResult>>>, StatusCode> {
    if search_query.q.trim().is_empty() {
        return Ok(ResponseJson(ApiResponse::error(
            "Query parameter 'q' is required and cannot be empty",
        )));
    }

    let repositories = match deployment
        .project()
        .get_repositories(&deployment.db().pool, project.id)
        .await
    {
        Ok(repos) => repos,
        Err(e) => {
            tracing::error!("Failed to get repositories: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match deployment
        .project()
        .search_files(
            deployment.file_search_cache().as_ref(),
            &repositories,
            &search_query,
        )
        .await
    {
        Ok(results) => Ok(ResponseJson(ApiResponse::success(results))),
        Err(e) => {
            tracing::error!("Failed to search files: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_project_repositories(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<Repo>>>, ApiError> {
    let repositories = deployment
        .project()
        .get_repositories(&deployment.db().pool, project.id)
        .await?;
    Ok(ResponseJson(ApiResponse::success(repositories)))
}

pub async fn add_project_repository(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateProjectRepo>,
) -> Result<ResponseJson<ApiResponse<Repo>>, ApiError> {
    tracing::debug!(
        "Adding repository '{}' to project {} (path: {})",
        payload.display_name,
        project.id,
        payload.git_repo_path
    );

    match deployment
        .project()
        .add_repository(
            &deployment.db().pool,
            deployment.repo(),
            project.id,
            &payload,
        )
        .await
    {
        Ok(repository) => {
            deployment
                .track_if_analytics_allowed(
                    "project_repository_added",
                    serde_json::json!({
                        "project_id": project.id.to_string(),
                        "repository_id": repository.id.to_string(),
                    }),
                )
                .await;

            Ok(ResponseJson(ApiResponse::success(repository)))
        }
        Err(ProjectServiceError::PathNotFound(_)) => {
            tracing::warn!(
                "Failed to add repository to project {}: path does not exist",
                project.id
            );
            Ok(ResponseJson(ApiResponse::error(
                "The specified path does not exist",
            )))
        }
        Err(ProjectServiceError::PathNotDirectory(_)) => {
            tracing::warn!(
                "Failed to add repository to project {}: path is not a directory",
                project.id
            );
            Ok(ResponseJson(ApiResponse::error(
                "The specified path is not a directory",
            )))
        }
        Err(ProjectServiceError::NotGitRepository(_)) => {
            tracing::warn!(
                "Failed to add repository to project {}: not a git repository",
                project.id
            );
            Ok(ResponseJson(ApiResponse::error(
                "The specified directory is not a git repository",
            )))
        }
        Err(ProjectServiceError::DuplicateRepositoryName) => {
            tracing::warn!(
                "Failed to add repository to project {}: duplicate repository name",
                project.id
            );
            Ok(ResponseJson(ApiResponse::error(
                "A repository with this name already exists in the project",
            )))
        }
        Err(ProjectServiceError::DuplicateGitRepoPath) => {
            tracing::warn!(
                "Failed to add repository to project {}: duplicate repository path",
                project.id
            );
            Ok(ResponseJson(ApiResponse::error(
                "A repository with this path already exists in the project",
            )))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn delete_project_repository(
    State(deployment): State<DeploymentImpl>,
    Path((project_id, repo_id)): Path<(Uuid, Uuid)>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    tracing::debug!(
        "Removing repository {} from project {}",
        repo_id,
        project_id
    );

    match deployment
        .project()
        .delete_repository(&deployment.db().pool, project_id, repo_id)
        .await
    {
        Ok(()) => {
            deployment
                .track_if_analytics_allowed(
                    "project_repository_removed",
                    serde_json::json!({
                        "project_id": project_id.to_string(),
                        "repository_id": repo_id.to_string(),
                    }),
                )
                .await;

            Ok(ResponseJson(ApiResponse::success(())))
        }
        Err(ProjectServiceError::RepositoryNotFound) => {
            tracing::warn!(
                "Failed to remove repository {} from project {}: not found",
                repo_id,
                project_id
            );
            Ok(ResponseJson(ApiResponse::error("Repository not found")))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn get_project_repository(
    State(deployment): State<DeploymentImpl>,
    Path((project_id, repo_id)): Path<(Uuid, Uuid)>,
) -> Result<ResponseJson<ApiResponse<ProjectRepo>>, ApiError> {
    match ProjectRepo::find_by_project_and_repo(&deployment.db().pool, project_id, repo_id).await {
        Ok(Some(project_repo)) => Ok(ResponseJson(ApiResponse::success(project_repo))),
        Ok(None) => Err(ApiError::BadRequest(
            "Repository not found in project".to_string(),
        )),
        Err(e) => Err(e.into()),
    }
}

pub async fn update_project_repository(
    State(deployment): State<DeploymentImpl>,
    Path((project_id, repo_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateProjectRepo>,
) -> Result<ResponseJson<ApiResponse<ProjectRepo>>, ApiError> {
    match ProjectRepo::update(&deployment.db().pool, project_id, repo_id, &payload).await {
        Ok(project_repo) => Ok(ResponseJson(ApiResponse::success(project_repo))),
        Err(db::models::project_repo::ProjectRepoError::NotFound) => Err(ApiError::BadRequest(
            "Repository not found in project".to_string(),
        )),
        Err(e) => Err(e.into()),
    }
}

/// POST /projects/{id}/grouper/start — idempotent
pub async fn start_grouper_agent(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<StartAgentResponse>>, ApiError> {
    use services::services::task_grouper::TASK_GROUPER_AGENT_ID;
    let pool = &deployment.db().pool;
    if let Some(ws_id) = project.grouper_workspace_id {
        return Ok(ResponseJson(ApiResponse::success(StartAgentResponse { workspace_id: ws_id, created: false })));
    }
    let (workspace_id, _) = create_column_agent_workspace(pool, &project, TASK_GROUPER_AGENT_ID, "Task Grouper", "grouper").await?;
    Project::set_grouper_workspace_id(pool, project.id, workspace_id).await?;
    broadcast_project_update(pool, &deployment, project.id).await;
    Ok(ResponseJson(ApiResponse::success(StartAgentResponse { workspace_id, created: true })))
}

/// POST /projects/{id}/group-evaluator/start — idempotent
pub async fn start_group_evaluator_agent(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<StartAgentResponse>>, ApiError> {
    use services::services::group_evaluator::GROUP_EVALUATOR_AGENT_ID;
    let pool = &deployment.db().pool;
    if let Some(ws_id) = project.group_evaluator_workspace_id {
        return Ok(ResponseJson(ApiResponse::success(StartAgentResponse { workspace_id: ws_id, created: false })));
    }
    let (workspace_id, _) = create_column_agent_workspace(pool, &project, GROUP_EVALUATOR_AGENT_ID, "Group Evaluator", "group-evaluator").await?;
    Project::set_group_evaluator_workspace_id(pool, project.id, workspace_id).await?;
    broadcast_project_update(pool, &deployment, project.id).await;
    Ok(ResponseJson(ApiResponse::success(StartAgentResponse { workspace_id, created: true })))
}

/// POST /projects/{id}/prereq-eval/start — idempotent
pub async fn start_prereq_eval_agent(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<StartAgentResponse>>, ApiError> {
    use services::services::prereq_evaluator::PREREQ_EVALUATOR_AGENT_ID;
    let pool = &deployment.db().pool;
    if let Some(ws_id) = project.prereq_eval_workspace_id {
        return Ok(ResponseJson(ApiResponse::success(StartAgentResponse { workspace_id: ws_id, created: false })));
    }
    let (workspace_id, _) = create_column_agent_workspace(pool, &project, PREREQ_EVALUATOR_AGENT_ID, "PreReq Evaluator", "prereq-evaluator").await?;
    Project::set_prereq_eval_workspace_id(pool, project.id, workspace_id).await?;
    broadcast_project_update(pool, &deployment, project.id).await;
    Ok(ResponseJson(ApiResponse::success(StartAgentResponse { workspace_id, created: true })))
}

/// Create a persistent lightweight workspace for a column-level agent.
/// Returns (workspace_id, executor) — caller stores the ID on the project.
async fn create_column_agent_workspace(
    pool: &sqlx::PgPool,
    project: &Project,
    agent_id: Uuid,
    agent_label: &str,
    branch_prefix: &str,
) -> Result<(Uuid, String), ApiError> {
    use db::models::agent::Agent;
    use db::models::session::{CreateSession, Session};
    use db::models::workspace::{CreateWorkspace, Workspace};

    let agent = Agent::find_by_id(pool, agent_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest(format!("{} agent not seeded", agent_label)))?;

    // Create a hidden system task to anchor the workspace (filtered from UI)
    let task = db::models::task::Task::create(
        pool,
        &db::models::task::CreateTask {
            project_id: project.id,
            title: format!("{} — {}", agent_label, project.name),
            description: None,
            status: None,
            column_id: None,
            parent_workspace_id: None,
            image_ids: None,
            shared_task_id: None,
            task_group_id: None,
        },
        Uuid::new_v4(),
    )
    .await?;

    let repos = ProjectRepo::find_repos_for_project(pool, project.id).await?;
    let repo_path = repos
        .first()
        .map(|r| r.path.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    let workspace_id = Uuid::new_v4();
    let short_id = utils::text::short_uuid(&workspace_id);
    Workspace::create(
        pool,
        &CreateWorkspace { branch: format!("{}/{}", branch_prefix, short_id), agent_working_dir: None },
        workspace_id,
        task.id,
    )
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to create workspace: {}", e)))?;

    Workspace::update_container_ref(pool, workspace_id, &repo_path).await?;

    // Create session so the panel can open before any execution has run
    Session::create(
        pool,
        &CreateSession { executor: Some(agent.executor.clone()) },
        Uuid::new_v4(),
        workspace_id,
    )
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to create session: {}", e)))?;

    Ok((workspace_id, agent.executor))
}

async fn broadcast_project_update(pool: &sqlx::PgPool, deployment: &DeploymentImpl, project_id: Uuid) {
    if let Ok(Some(updated)) = Project::find_by_id(pool, project_id).await {
        deployment
            .events()
            .msg_store()
            .push_patch(services::services::events::project_patch::replace(&updated));
    }
}

/// POST /projects/{id}/agent/start — idempotent
///
/// If the project already has an agent workspace, returns it.
/// Otherwise, creates a lightweight workspace + session ready for the first user message.
/// No initial execution is run — the first follow-up creates the initial Claude session.
pub async fn start_project_agent(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<StartAgentResponse>>, ApiError> {
    use db::models::agent::Agent;
    use db::models::session::{CreateSession, Session};
    use db::models::workspace::{CreateWorkspace, Workspace};
    use services::services::project_agent::PROJECT_AGENT_ID;

    let pool = &deployment.db().pool;

    // Idempotent: if workspace already exists, return it
    if let Some(workspace_id) = project.agent_workspace_id {
        return Ok(ResponseJson(ApiResponse::success(StartAgentResponse {
            workspace_id,
            created: false,
        })));
    }

    // Load the Project Agent definition to get executor type
    let agent = Agent::find_by_id(pool, PROJECT_AGENT_ID)
        .await?
        .ok_or_else(|| ApiError::BadRequest("Project Agent not seeded".to_string()))?;

    // Create a hidden system task (filtered from UI by title regex)
    let task_title = format!("Project Agent — {}", project.name);
    let task = db::models::task::Task::create(
        pool,
        &db::models::task::CreateTask {
            project_id: project.id,
            title: task_title,
            description: None,
            status: None,
            column_id: None,
            parent_workspace_id: None,
            image_ids: None,
            shared_task_id: None,
            task_group_id: None,
        },
        Uuid::new_v4(),
    )
    .await?;

    // Get the project repo path for container_ref
    let repos = ProjectRepo::find_repos_for_project(pool, project.id).await?;
    let repo_path = repos
        .first()
        .map(|r| r.path.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    // Create lightweight workspace (no git worktree)
    let workspace_id = Uuid::new_v4();
    let short_id = utils::text::short_uuid(&workspace_id);
    let branch_name = format!("project-agent/{}", short_id);
    let create_data = CreateWorkspace {
        branch: branch_name,
        agent_working_dir: None,
    };
    Workspace::create(pool, &create_data, workspace_id, task.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create workspace: {}", e)))?;

    // Pre-set container_ref (prevents worktree creation)
    Workspace::update_container_ref(pool, workspace_id, &repo_path).await?;

    // Create a session so follow-up can be sent immediately
    // The executor is stored so the follow_up handler can use it for the first request
    Session::create(
        pool,
        &CreateSession {
            executor: Some(agent.executor.clone()),
        },
        Uuid::new_v4(),
        workspace_id,
    )
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to create session: {}", e)))?;

    // Link project to workspace
    Project::set_agent_workspace_id(pool, project.id, workspace_id).await?;

    // Broadcast project update (now has agent_workspace_id)
    if let Ok(Some(updated)) = Project::find_by_id(pool, project.id).await {
        deployment
            .events()
            .msg_store()
            .push_patch(project_patch::replace(&updated));
    }

    tracing::info!(
        "Project Agent workspace created for '{}' ({}) — workspace {}",
        project.name,
        project.id,
        workspace_id
    );

    Ok(ResponseJson(ApiResponse::success(StartAgentResponse {
        workspace_id,
        created: true,
    })))
}

#[derive(Debug, Serialize, TS)]
pub struct StartAgentResponse {
    pub workspace_id: Uuid,
    pub created: bool,
}

/// Clear the ready_locked flag on a project.
///
/// Called by the Group Evaluator after it has processed all analyzing groups
/// and verified the project is stable (all groups in ready/executing/done).
/// Clearing the lock allows advance_project_dag() to advance the next group.
async fn unlock_project(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    Project::set_ready_locked(&deployment.db().pool, project.id, false).await?;

    tracing::info!("Project {} unlocked by Group Evaluator", project.id);

    // Immediately try to advance the project DAG now that the lock is cleared
    if let Err(e) = deployment.container().advance_project_dag(project.id).await {
        tracing::warn!("advance_project_dag after unlock failed: {}", e);
    }

    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let project_id_router = Router::new()
        .route(
            "/",
            get(get_project).put(update_project).delete(delete_project),
        )
        .route("/remote/members", get(get_project_remote_members))
        .route("/search", get(search_project_files))
        .route("/open-editor", post(open_project_in_editor))
        .route("/agent/start", post(start_project_agent))
        .route("/grouper/start", post(start_grouper_agent))
        .route("/group-evaluator/start", post(start_group_evaluator_agent))
        .route("/prereq-eval/start", post(start_prereq_eval_agent))
        .route("/unlock", post(unlock_project))
        .route(
            "/link",
            post(link_project_to_existing_remote).delete(unlink_project),
        )
        .route("/link/create", post(create_and_link_remote_project))
        .route(
            "/repositories",
            get(get_project_repositories).post(add_project_repository),
        )
        .layer(from_fn_with_state(
            deployment.clone(),
            load_project_middleware,
        ));

    let projects_router = Router::new()
        .route("/", get(get_projects).post(create_project))
        .route(
            "/{project_id}/repositories/{repo_id}",
            get(get_project_repository)
                .put(update_project_repository)
                .delete(delete_project_repository),
        )
        .route("/stream/ws", get(stream_projects_ws))
        .nest("/{id}", project_id_router);

    Router::new().nest("/projects", projects_router).route(
        "/remote-projects/{remote_project_id}",
        get(get_remote_project_by_id),
    )
}

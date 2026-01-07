use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use db::models::{
    agent::Agent, automation_rule::AutomationRule, board::Board,
    execution_process::ExecutionProcess, kanban_column::KanbanColumn, project::Project,
    session::Session, state_transition::StateTransition, tag::Tag, task::Task,
    workspace::Workspace,
};
use deployment::Deployment;
use uuid::Uuid;

use crate::DeploymentImpl;

pub async fn load_project_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Load the project from the database
    let project = match Project::find_by_id(&deployment.db().pool, project_id).await {
        Ok(Some(project)) => project,
        Ok(None) => {
            tracing::warn!("Project {} not found", project_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("Failed to fetch project {}: {}", project_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Insert the project as an extension
    let mut request = request;
    request.extensions_mut().insert(project);

    // Continue with the next middleware/handler
    Ok(next.run(request).await)
}

pub async fn load_task_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Load the task and validate it belongs to the project
    let task = match Task::find_by_id(&deployment.db().pool, task_id).await {
        Ok(Some(task)) => task,
        Ok(None) => {
            tracing::warn!("Task {} not found", task_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("Failed to fetch task {}: {}", task_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Insert both models as extensions
    let mut request = request;
    request.extensions_mut().insert(task);

    // Continue with the next middleware/handler
    Ok(next.run(request).await)
}

pub async fn load_workspace_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(workspace_id): Path<Uuid>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Load the Workspace from the database
    let workspace = match Workspace::find_by_id(&deployment.db().pool, workspace_id).await {
        Ok(Some(w)) => w,
        Ok(None) => {
            tracing::warn!("Workspace {} not found", workspace_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("Failed to fetch Workspace {}: {}", workspace_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Insert the workspace into extensions
    request.extensions_mut().insert(workspace);

    // Continue on
    Ok(next.run(request).await)
}

pub async fn load_execution_process_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(process_id): Path<Uuid>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Load the execution process from the database
    let execution_process =
        match ExecutionProcess::find_by_id(&deployment.db().pool, process_id).await {
            Ok(Some(process)) => process,
            Ok(None) => {
                tracing::warn!("ExecutionProcess {} not found", process_id);
                return Err(StatusCode::NOT_FOUND);
            }
            Err(e) => {
                tracing::error!("Failed to fetch execution process {}: {}", process_id, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

    // Inject the execution process into the request
    request.extensions_mut().insert(execution_process);

    // Continue to the next middleware/handler
    Ok(next.run(request).await)
}

// Middleware that loads and injects Tag based on the tag_id path parameter
pub async fn load_tag_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(tag_id): Path<Uuid>,
    request: axum::extract::Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Load the tag from the database
    let tag = match Tag::find_by_id(&deployment.db().pool, tag_id).await {
        Ok(Some(tag)) => tag,
        Ok(None) => {
            tracing::warn!("Tag {} not found", tag_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("Failed to fetch tag {}: {}", tag_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Insert the tag as an extension
    let mut request = request;
    request.extensions_mut().insert(tag);

    // Continue with the next middleware/handler
    Ok(next.run(request).await)
}

pub async fn load_session_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(session_id): Path<Uuid>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let session = match Session::find_by_id(&deployment.db().pool, session_id).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            tracing::warn!("Session {} not found", session_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("Failed to fetch session {}: {}", session_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    request.extensions_mut().insert(session);
    Ok(next.run(request).await)
}

pub async fn load_agent_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(agent_id): Path<Uuid>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let agent = match Agent::find_by_id(&deployment.db().pool, agent_id).await {
        Ok(Some(agent)) => agent,
        Ok(None) => {
            tracing::warn!("Agent {} not found", agent_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("Failed to fetch agent {}: {}", agent_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    request.extensions_mut().insert(agent);
    Ok(next.run(request).await)
}

pub async fn load_kanban_column_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(column_id): Path<Uuid>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let column = match KanbanColumn::find_by_id(&deployment.db().pool, column_id).await {
        Ok(Some(column)) => column,
        Ok(None) => {
            tracing::warn!("KanbanColumn {} not found", column_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("Failed to fetch kanban column {}: {}", column_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    request.extensions_mut().insert(column);
    Ok(next.run(request).await)
}

pub async fn load_automation_rule_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(rule_id): Path<Uuid>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let rule = match AutomationRule::find_by_id(&deployment.db().pool, rule_id).await {
        Ok(Some(rule)) => rule,
        Ok(None) => {
            tracing::warn!("AutomationRule {} not found", rule_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("Failed to fetch automation rule {}: {}", rule_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    request.extensions_mut().insert(rule);
    Ok(next.run(request).await)
}

pub async fn load_state_transition_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(transition_id): Path<Uuid>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let transition = match StateTransition::find_by_id(&deployment.db().pool, transition_id).await {
        Ok(Some(transition)) => transition,
        Ok(None) => {
            tracing::warn!("StateTransition {} not found", transition_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("Failed to fetch state transition {}: {}", transition_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    request.extensions_mut().insert(transition);
    Ok(next.run(request).await)
}

/// Board path params - handles both single board routes and nested column routes
#[derive(Debug, serde::Deserialize)]
pub struct BoardPathParams {
    pub board_id: Uuid,
    #[serde(default)]
    pub column_id: Option<Uuid>,
}

pub async fn load_board_middleware(
    State(deployment): State<DeploymentImpl>,
    Path(params): Path<BoardPathParams>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let board = match Board::find_by_id(&deployment.db().pool, params.board_id).await {
        Ok(Some(board)) => board,
        Ok(None) => {
            tracing::warn!("Board {} not found", params.board_id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            tracing::error!("Failed to fetch board {}: {}", params.board_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    request.extensions_mut().insert(board);
    Ok(next.run(request).await)
}

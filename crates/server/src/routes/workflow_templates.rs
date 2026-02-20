use axum::{
    Extension, Json, Router,
    extract::State,
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::agent::Agent;
use db::models::board::{Board, TemplateInfo};
use db::models::kanban_column::{CreateKanbanColumn, KanbanColumn};
use db::models::project::Project;
use db::models::state_transition::{CreateStateTransition, StateTransition};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::load_project_middleware};

/// List all available workflow templates
pub async fn list_templates(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TemplateInfo>>>, ApiError> {
    let templates = Board::find_templates(&deployment.db().pool).await?;
    Ok(ResponseJson(ApiResponse::success(templates)))
}

#[derive(Debug, Deserialize, TS)]
pub struct ApplyTemplateRequest {
    pub template_board_id: Uuid,
}

#[derive(Debug, Serialize, TS)]
pub struct ApplyTemplateResponse {
    pub board_id: Uuid,
    pub agents_created: usize,
    pub columns_created: usize,
    pub transitions_created: usize,
}

/// Apply a workflow template to a project
pub async fn apply_template(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<ApplyTemplateRequest>,
) -> Result<ResponseJson<ApiResponse<ApplyTemplateResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    // 1. Get the template board
    let template_board = Board::find_by_id(pool, payload.template_board_id)
        .await?
        .ok_or(ApiError::BadRequest("Template board not found".to_string()))?;

    if !template_board.is_template {
        return Err(ApiError::BadRequest("Not a template board".to_string()));
    }

    let template_group_id = template_board
        .template_group_id
        .as_ref()
        .ok_or(ApiError::BadRequest("Template has no group ID".to_string()))?;

    // 2. Get the project's board (or create one if it doesn't exist)
    let board = match project.board_id {
        Some(board_id) => Board::find_by_id(pool, board_id)
            .await?
            .ok_or(ApiError::BadRequest("Project board not found".to_string()))?,
        None => {
            // Create a default board for the project
            let new_board = Board::create(
                pool,
                &db::models::board::CreateBoard {
                    name: format!("{} Board", project.name),
                    description: None,
                },
            )
            .await?;
            // Update project to reference the new board
            Project::update_board_id(pool, project.id, new_board.id).await?;
            new_board
        }
    };

    // 3. Get all template entities by group_id
    let template_columns = KanbanColumn::find_by_template_group(pool, template_group_id).await?;
    let template_transitions =
        StateTransition::find_by_template_group(pool, template_group_id).await?;

    // 4. Delete existing columns and transitions for this board
    StateTransition::delete_by_board(pool, board.id).await?;
    KanbanColumn::delete_by_board(pool, board.id).await?;

    // 5. Create columns (build old_id -> new_id mapping)
    // Reuse template agents directly — agents are shared, not cloned
    let mut column_id_map: HashMap<Uuid, Uuid> = HashMap::new();
    for tmpl_col in &template_columns {
        let new_agent_id = tmpl_col.agent_id;

        let column = KanbanColumn::create_for_board(
            pool,
            board.id,
            &CreateKanbanColumn {
                name: tmpl_col.name.clone(),
                slug: tmpl_col.slug.clone(),
                position: tmpl_col.position,
                color: tmpl_col.color.clone(),
                is_initial: Some(tmpl_col.is_initial),
                is_terminal: Some(tmpl_col.is_terminal),
                starts_workflow: Some(tmpl_col.starts_workflow),
                status: Some(tmpl_col.status.clone()),
                agent_id: new_agent_id,
                deliverable: tmpl_col.deliverable.clone(),
                question: tmpl_col.question.clone(),
                answer_options: tmpl_col.answer_options.clone(),
            },
        )
        .await?;
        column_id_map.insert(tmpl_col.id, column.id);
    }

    // 7. Create transitions with remapped column IDs
    let mut transitions_created = 0;
    for tmpl_trans in &template_transitions {
        let new_from = column_id_map
            .get(&tmpl_trans.from_column_id)
            .ok_or(ApiError::BadRequest(
                "Invalid template: missing from_column".to_string(),
            ))?;
        let new_to = column_id_map
            .get(&tmpl_trans.to_column_id)
            .ok_or(ApiError::BadRequest(
                "Invalid template: missing to_column".to_string(),
            ))?;
        let new_else = tmpl_trans
            .else_column_id
            .and_then(|id| column_id_map.get(&id).copied());
        let new_escalation = tmpl_trans
            .escalation_column_id
            .and_then(|id| column_id_map.get(&id).copied());

        StateTransition::create_for_board(
            pool,
            board.id,
            &CreateStateTransition {
                from_column_id: *new_from,
                to_column_id: *new_to,
                else_column_id: new_else,
                escalation_column_id: new_escalation,
                name: tmpl_trans.name.clone(),
                requires_confirmation: Some(tmpl_trans.requires_confirmation),
                condition_value: tmpl_trans.condition_value.clone(),
                max_failures: tmpl_trans.max_failures,
            },
        )
        .await?;
        transitions_created += 1;
    }

    deployment
        .track_if_analytics_allowed(
            "workflow_template_applied",
            serde_json::json!({
                "project_id": project.id.to_string(),
                "template_group_id": template_group_id,
                "template_name": template_board.template_name,
                "agents_created": 0,
                "columns_created": column_id_map.len(),
                "transitions_created": transitions_created,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(ApplyTemplateResponse {
        board_id: board.id,
        agents_created: 0,
        columns_created: column_id_map.len(),
        transitions_created,
    })))
}

#[derive(Debug, Deserialize, TS)]
pub struct SaveAsTemplateRequest {
    pub template_name: String,
    pub template_description: String,
    #[serde(default = "default_template_icon")]
    pub template_icon: String,
}

fn default_template_icon() -> String {
    "LayoutTemplate".to_string()
}

#[derive(Debug, Serialize, TS)]
pub struct SaveAsTemplateResponse {
    pub template_board_id: Uuid,
    pub template_group_id: String,
    pub agents_cloned: usize,
    pub columns_cloned: usize,
    pub transitions_cloned: usize,
}

/// Save a board as a reusable template
pub async fn save_as_template(
    Extension(board): Extension<Board>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<SaveAsTemplateRequest>,
) -> Result<ResponseJson<ApiResponse<SaveAsTemplateResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    if board.is_template {
        return Err(ApiError::BadRequest(
            "Cannot save a template as a template".to_string(),
        ));
    }

    let template_group_id = Uuid::new_v4().to_string();

    // 1. Get the board's columns, transitions, and referenced agents
    let columns = KanbanColumn::find_by_board(pool, board.id).await?;
    let transitions = StateTransition::find_by_board(pool, board.id).await?;

    // Collect unique agent IDs from columns
    let agent_ids: Vec<Uuid> = columns
        .iter()
        .filter_map(|col| col.agent_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // 2. Clone agents as templates (build old_id -> new_id mapping)
    let mut agent_id_map: HashMap<Uuid, Uuid> = HashMap::new();
    for agent_id in &agent_ids {
        if let Some(agent) = Agent::find_by_id(pool, *agent_id).await? {
            let new_agent =
                Agent::clone_as_template(pool, &agent, &template_group_id).await?;
            agent_id_map.insert(*agent_id, new_agent.id);
        }
    }

    // 3. Create template board
    let template_board = Board::create_template(
        pool,
        &format!("{} Template", board.name),
        board.description.as_deref(),
        &template_group_id,
        &payload.template_name,
        &payload.template_description,
        &payload.template_icon,
    )
    .await?;

    // 4. Clone columns as templates (build old_id -> new_id mapping)
    let mut column_id_map: HashMap<Uuid, Uuid> = HashMap::new();
    for col in &columns {
        let new_agent_id = col
            .agent_id
            .and_then(|old_id| agent_id_map.get(&old_id).copied());

        let new_col = KanbanColumn::clone_as_template(
            pool,
            col,
            template_board.id,
            &template_group_id,
            new_agent_id,
        )
        .await?;
        column_id_map.insert(col.id, new_col.id);
    }

    // 5. Clone transitions as templates with remapped column IDs
    let mut transitions_cloned = 0;
    for trans in &transitions {
        let new_from = match column_id_map.get(&trans.from_column_id) {
            Some(id) => *id,
            None => continue,
        };
        let new_to = match column_id_map.get(&trans.to_column_id) {
            Some(id) => *id,
            None => continue,
        };
        let new_else = trans
            .else_column_id
            .and_then(|id| column_id_map.get(&id).copied());
        let new_escalation = trans
            .escalation_column_id
            .and_then(|id| column_id_map.get(&id).copied());

        StateTransition::clone_as_template(
            pool,
            trans,
            template_board.id,
            &template_group_id,
            new_from,
            new_to,
            new_else,
            new_escalation,
        )
        .await?;
        transitions_cloned += 1;
    }

    deployment
        .track_if_analytics_allowed(
            "board_saved_as_template",
            serde_json::json!({
                "source_board_id": board.id.to_string(),
                "template_group_id": template_group_id,
                "template_name": payload.template_name,
                "agents_cloned": agent_id_map.len(),
                "columns_cloned": column_id_map.len(),
                "transitions_cloned": transitions_cloned,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(SaveAsTemplateResponse {
        template_board_id: template_board.id,
        template_group_id,
        agents_cloned: agent_id_map.len(),
        columns_cloned: column_id_map.len(),
        transitions_cloned,
    })))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Route for applying template to a project (requires project context)
    let project_router = Router::new()
        .route("/apply-template", post(apply_template))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_project_middleware,
        ));

    Router::new()
        .route("/workflow-templates", get(list_templates))
        .nest("/projects/{project_id}", project_router)
}

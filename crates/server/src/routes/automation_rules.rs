use axum::{
    Extension, Json, Router,
    extract::State,
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::{
    automation_rule::{AutomationRule, AutomationRuleWithColumn, CreateAutomationRule, UpdateAutomationRule},
    project::Project,
};
use deployment::Deployment;
use utils::response::ApiResponse;

use crate::{
    DeploymentImpl, error::ApiError,
    middleware::{load_automation_rule_middleware, load_project_middleware},
};

/// Get all automation rules for a project (with column names)
pub async fn get_project_rules(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<AutomationRuleWithColumn>>>, ApiError> {
    let rules = AutomationRule::find_by_project_with_columns(&deployment.db().pool, project.id).await?;
    Ok(ResponseJson(ApiResponse::success(rules)))
}

/// Create a new automation rule
pub async fn create_rule(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateAutomationRule>,
) -> Result<ResponseJson<ApiResponse<AutomationRule>>, ApiError> {
    let rule = AutomationRule::create(&deployment.db().pool, project.id, &payload).await?;

    deployment
        .track_if_analytics_allowed(
            "automation_rule_created",
            serde_json::json!({
                "project_id": project.id.to_string(),
                "rule_id": rule.id.to_string(),
                "trigger_type": rule.trigger_type,
                "action_type": rule.action_type,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(rule)))
}

/// Get a single automation rule
pub async fn get_rule(
    Extension(rule): Extension<AutomationRule>,
) -> Result<ResponseJson<ApiResponse<AutomationRule>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(rule)))
}

/// Update an automation rule
pub async fn update_rule(
    Extension(rule): Extension<AutomationRule>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateAutomationRule>,
) -> Result<ResponseJson<ApiResponse<AutomationRule>>, ApiError> {
    let updated = AutomationRule::update(&deployment.db().pool, rule.id, &payload).await?;

    deployment
        .track_if_analytics_allowed(
            "automation_rule_updated",
            serde_json::json!({
                "rule_id": rule.id.to_string(),
                "trigger_type": updated.trigger_type,
                "action_type": updated.action_type,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(updated)))
}

/// Delete an automation rule
pub async fn delete_rule(
    Extension(rule): Extension<AutomationRule>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let rows_affected = AutomationRule::delete(&deployment.db().pool, rule.id).await?;
    if rows_affected == 0 {
        Err(ApiError::Database(sqlx::Error::RowNotFound))
    } else {
        deployment
            .track_if_analytics_allowed(
                "automation_rule_deleted",
                serde_json::json!({
                    "rule_id": rule.id.to_string(),
                }),
            )
            .await;

        Ok(ResponseJson(ApiResponse::success(())))
    }
}

/// Toggle rule enabled state
pub async fn toggle_rule(
    Extension(rule): Extension<AutomationRule>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let new_enabled = !rule.enabled;
    AutomationRule::set_enabled(&deployment.db().pool, rule.id, new_enabled).await?;

    deployment
        .track_if_analytics_allowed(
            "automation_rule_toggled",
            serde_json::json!({
                "rule_id": rule.id.to_string(),
                "enabled": new_enabled,
            }),
        )
        .await;

    Ok(ResponseJson(ApiResponse::success(())))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    // Routes for a specific rule (requires rule_id)
    let rule_router = Router::new()
        .route("/", get(get_rule).put(update_rule).delete(delete_rule))
        .route("/toggle", axum::routing::post(toggle_rule))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_automation_rule_middleware,
        ));

    // Routes under /projects/:project_id/automation-rules
    let project_rules_router = Router::new()
        .route("/", get(get_project_rules).post(create_rule))
        .nest("/{rule_id}", rule_router)
        .layer(from_fn_with_state(
            deployment.clone(),
            load_project_middleware,
        ));

    Router::new().nest("/projects/{project_id}/automation-rules", project_rules_router)
}

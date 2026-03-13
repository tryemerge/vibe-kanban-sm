use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::get,
};
use db::models::{
    agent::Agent,
    skill::{CreateSkill, Skill, UpdateSkill},
};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::load_skill_middleware};

// ── Skill CRUD ─────────────────────────────────────────────────────────────

pub async fn list_skills(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<Skill>>>, ApiError> {
    let skills = Skill::find_all(&deployment.db().pool).await?;
    Ok(ResponseJson(ApiResponse::success(skills)))
}

pub async fn get_skill(
    Extension(skill): Extension<Skill>,
) -> Result<ResponseJson<ApiResponse<Skill>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(skill)))
}

pub async fn create_skill(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateSkill>,
) -> Result<ResponseJson<ApiResponse<Skill>>, ApiError> {
    let skill = Skill::create(&deployment.db().pool, payload).await?;
    Ok(ResponseJson(ApiResponse::success(skill)))
}

pub async fn update_skill(
    Extension(skill): Extension<Skill>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateSkill>,
) -> Result<ResponseJson<ApiResponse<Skill>>, ApiError> {
    let updated = Skill::update(&deployment.db().pool, skill.id, payload)
        .await?
        .ok_or(ApiError::Database(sqlx::Error::RowNotFound))?;
    Ok(ResponseJson(ApiResponse::success(updated)))
}

pub async fn delete_skill(
    Extension(skill): Extension<Skill>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let count = Skill::agent_count(&deployment.db().pool, skill.id).await?;
    if count > 0 {
        return Err(ApiError::BadRequest(format!(
            "Cannot delete skill '{}': it is assigned to {} agent(s). Remove it from all agents first.",
            skill.name, count
        )));
    }
    let deleted = Skill::delete(&deployment.db().pool, skill.id).await?;
    if !deleted {
        return Err(ApiError::Database(sqlx::Error::RowNotFound));
    }
    Ok(ResponseJson(ApiResponse::success(())))
}

// ── Agent ↔ Skill assignment ────────────────────────────────────────────────

pub async fn list_agent_skills(
    State(deployment): State<DeploymentImpl>,
    Path(agent_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<Vec<Skill>>>, ApiError> {
    // Verify agent exists
    Agent::find_by_id(&deployment.db().pool, agent_id)
        .await?
        .ok_or(ApiError::Database(sqlx::Error::RowNotFound))?;

    let skills = Skill::load_for_agent(&deployment.db().pool, agent_id).await?;
    Ok(ResponseJson(ApiResponse::success(skills)))
}

pub async fn assign_skill_to_agent(
    State(deployment): State<DeploymentImpl>,
    Path((agent_id, skill_id)): Path<(Uuid, Uuid)>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    // Verify both exist
    Agent::find_by_id(&deployment.db().pool, agent_id)
        .await?
        .ok_or(ApiError::Database(sqlx::Error::RowNotFound))?;
    Skill::find_by_id(&deployment.db().pool, skill_id)
        .await?
        .ok_or(ApiError::Database(sqlx::Error::RowNotFound))?;

    Skill::assign_to_agent(&deployment.db().pool, agent_id, skill_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

pub async fn unassign_skill_from_agent(
    State(deployment): State<DeploymentImpl>,
    Path((agent_id, skill_id)): Path<(Uuid, Uuid)>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    Skill::unassign_from_agent(&deployment.db().pool, agent_id, skill_id).await?;
    Ok(ResponseJson(ApiResponse::success(())))
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let skill_router = Router::new()
        .route("/", get(get_skill).put(update_skill).delete(delete_skill))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_skill_middleware,
        ));

    let skills_inner = Router::new()
        .route("/", get(list_skills).post(create_skill))
        .nest("/{skill_id}", skill_router);

    let agent_skills_router = Router::new()
        .route(
            "/skills",
            get(list_agent_skills),
        )
        .route(
            "/skills/{skill_id}",
            axum::routing::post(assign_skill_to_agent)
                .delete(unassign_skill_from_agent),
        );

    Router::new()
        .nest("/skills", skills_inner)
        .nest("/agents/{agent_id}", agent_skills_router)
}

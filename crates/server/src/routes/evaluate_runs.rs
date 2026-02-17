use axum::{
    Json, Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::get,
};
use db::models::evaluate_run::{CreateEvaluateRun, EvaluateRun};
use deployment::Deployment;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub async fn list_runs(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<EvaluateRun>>>, ApiError> {
    let runs = EvaluateRun::find_all(&deployment.db().pool).await?;
    Ok(ResponseJson(ApiResponse::success(runs)))
}

pub async fn create_run(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateEvaluateRun>,
) -> Result<ResponseJson<ApiResponse<EvaluateRun>>, ApiError> {
    let run = EvaluateRun::create(&deployment.db().pool, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(run)))
}

pub async fn get_run(
    State(deployment): State<DeploymentImpl>,
    Path(run_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<EvaluateRun>>, ApiError> {
    let run = EvaluateRun::find_by_id(&deployment.db().pool, run_id)
        .await?
        .ok_or(ApiError::BadRequest("Evaluate run not found".to_string()))?;
    Ok(ResponseJson(ApiResponse::success(run)))
}

pub async fn delete_run(
    State(deployment): State<DeploymentImpl>,
    Path(run_id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let rows = EvaluateRun::delete(&deployment.db().pool, run_id).await?;
    if rows == 0 {
        Err(ApiError::BadRequest("Evaluate run not found".to_string()))
    } else {
        Ok(ResponseJson(ApiResponse::success(())))
    }
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let _ = deployment;
    Router::new()
        .route("/evaluate-runs", get(list_runs).post(create_run))
        .route(
            "/evaluate-runs/{run_id}",
            get(get_run).delete(delete_run),
        )
}

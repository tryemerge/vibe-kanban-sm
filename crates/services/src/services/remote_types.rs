//! Stub types that were previously in the remote crate.
//! These are used for communication with a remote task sharing service.
//! Since the remote crate has been removed, these stubs allow the code to compile
//! but the actual remote sync functionality will not work.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

/// Remote task status - mirrors local TaskStatus for remote sync
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "lowercase")]
pub enum RemoteTaskStatus {
    Todo,
    #[serde(rename = "inprogress")]
    InProgress,
    #[serde(rename = "inreview")]
    InReview,
    Done,
    Cancelled,
}

/// Request to create a shared task on remote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSharedTaskRequest {
    pub project_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub assignee_user_id: Option<Uuid>,
}

/// Request to update a shared task on remote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSharedTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<RemoteTaskStatus>,
}

/// Request to assign a shared task to a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignSharedTaskRequest {
    pub new_assignee_user_id: Option<Uuid>,
}

/// Request to check if tasks exist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckTasksRequest {
    pub task_ids: Vec<Uuid>,
}

/// Response containing shared task data
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SharedTaskResponse {
    pub task: SharedTask,
}

/// Shared task data from remote
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SharedTaskData {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: RemoteTaskStatus,
    pub assignee_user_id: Option<Uuid>,
    pub creator_user_id: Option<Uuid>,
}

/// Shared task from remote service
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SharedTask {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub project_id: Uuid,
    pub creator_user_id: Option<Uuid>,
    pub assignee_user_id: Option<Uuid>,
    pub deleted_by_user_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: RemoteTaskStatus,
    #[ts(type = "Date | null")]
    pub deleted_at: Option<DateTime<Utc>>,
    #[ts(type = "Date | null")]
    pub shared_at: Option<DateTime<Utc>>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

/// User data from remote service
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct UserData {
    pub user_id: Uuid,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
}

/// Query parameters for getting assignees
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AssigneesQuery {
    pub project_id: String,
}

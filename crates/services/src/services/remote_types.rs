//! Stub types that were previously in the remote crate.
//! These are used for communication with a remote task sharing service.
//! Since the remote crate has been removed, these stubs allow the code to compile
//! but the actual remote sync functionality will not work.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Remote task status - mirrors local TaskStatus for remote sync
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum RemoteTaskStatus {
    Todo,
    InProgress,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedTaskResponse {
    pub task: SharedTaskData,
}

/// Shared task data from remote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedTaskData {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: RemoteTaskStatus,
    pub assignee_user_id: Option<Uuid>,
    pub creator_user_id: Option<Uuid>,
}

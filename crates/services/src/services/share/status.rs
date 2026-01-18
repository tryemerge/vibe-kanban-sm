use db::models::task::TaskStatus;
use super::super::remote_types::RemoteTaskStatus;

pub(super) fn to_remote(status: &TaskStatus) -> RemoteTaskStatus {
    match status {
        TaskStatus::Todo => RemoteTaskStatus::Todo,
        TaskStatus::InProgress => RemoteTaskStatus::InProgress,
        TaskStatus::InReview => RemoteTaskStatus::InReview,
        TaskStatus::Done => RemoteTaskStatus::Done,
        TaskStatus::Cancelled => RemoteTaskStatus::Cancelled,
    }
}

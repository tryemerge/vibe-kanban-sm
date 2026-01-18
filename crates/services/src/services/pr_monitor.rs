use std::time::Duration;

use db::{
    DBService,
    models::{
        board::Board,
        kanban_column::KanbanColumn,
        merge::{Merge, MergeStatus, PrMerge},
        project::Project,
        task::{Task, TaskStatus},
        task_trigger::TaskTrigger,
        workspace::{Workspace, WorkspaceError},
    },
};
use serde_json::json;
use sqlx::error::Error as SqlxError;
use thiserror::Error;
use tokio::time::interval;
use tracing::{debug, error, info};

use crate::services::{
    analytics::AnalyticsContext,
    github::{GitHubRepoInfo, GitHubService, GitHubServiceError},
    share::SharePublisher,
};

#[derive(Debug, Error)]
enum PrMonitorError {
    #[error(transparent)]
    GitHubServiceError(#[from] GitHubServiceError),
    #[error(transparent)]
    WorkspaceError(#[from] WorkspaceError),
    #[error(transparent)]
    Sqlx(#[from] SqlxError),
}

/// Service to monitor GitHub PRs and update task status when they are merged
pub struct PrMonitorService {
    db: DBService,
    poll_interval: Duration,
    analytics: Option<AnalyticsContext>,
    publisher: Option<SharePublisher>,
}

impl PrMonitorService {
    pub async fn spawn(
        db: DBService,
        analytics: Option<AnalyticsContext>,
        publisher: Option<SharePublisher>,
    ) -> tokio::task::JoinHandle<()> {
        let service = Self {
            db,
            poll_interval: Duration::from_secs(60), // Check every minute
            analytics,
            publisher,
        };
        tokio::spawn(async move {
            service.start().await;
        })
    }

    async fn start(&self) {
        info!(
            "Starting PR monitoring service with interval {:?}",
            self.poll_interval
        );

        let mut interval = interval(self.poll_interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.check_all_open_prs().await {
                error!("Error checking open PRs: {}", e);
            }
        }
    }

    /// Check all open PRs for updates with the provided GitHub token
    async fn check_all_open_prs(&self) -> Result<(), PrMonitorError> {
        let open_prs = Merge::get_open_prs(&self.db.pool).await?;

        if open_prs.is_empty() {
            debug!("No open PRs to check");
            return Ok(());
        }

        info!("Checking {} open PRs", open_prs.len());

        for pr_merge in open_prs {
            if let Err(e) = self.check_pr_status(&pr_merge).await {
                error!(
                    "Error checking PR #{} for workspace {}: {}",
                    pr_merge.pr_info.number, pr_merge.workspace_id, e
                );
            }
        }
        Ok(())
    }

    /// Check the status of a specific PR
    async fn check_pr_status(&self, pr_merge: &PrMerge) -> Result<(), PrMonitorError> {
        // GitHubService now uses gh CLI, no token needed
        let github_service = GitHubService::new()?;
        let repo_info = GitHubRepoInfo::from_remote_url(&pr_merge.pr_info.url)?;

        let pr_status = github_service
            .update_pr_status(&repo_info, pr_merge.pr_info.number.into())
            .await?;

        debug!(
            "PR #{} status: {:?} (was open)",
            pr_merge.pr_info.number, pr_status.status
        );

        // Update the PR status in the database
        if !matches!(&pr_status.status, MergeStatus::Open) {
            // Update merge status with the latest information from GitHub
            Merge::update_status(
                &self.db.pool,
                pr_merge.id,
                pr_status.status.clone(),
                pr_status.merge_commit_sha,
            )
            .await?;

            // If the PR was merged, update the task status to done
            if matches!(&pr_status.status, MergeStatus::Merged)
                && let Some(workspace) =
                    Workspace::find_by_id(&self.db.pool, pr_merge.workspace_id).await?
            {
                info!(
                    "PR #{} was merged, updating task {} to done",
                    pr_merge.pr_info.number, workspace.task_id
                );
                Task::update_status(&self.db.pool, workspace.task_id, TaskStatus::Done).await?;

                // Track analytics event
                if let Some(analytics) = &self.analytics
                    && let Ok(Some(task)) = Task::find_by_id(&self.db.pool, workspace.task_id).await
                {
                    analytics.analytics_service.track_event(
                        &analytics.user_id,
                        "pr_merged",
                        Some(json!({
                            "task_id": workspace.task_id.to_string(),
                            "workspace_id": workspace.id.to_string(),
                            "project_id": task.project_id.to_string(),
                        })),
                    );
                }

                if let Some(publisher) = &self.publisher
                    && let Err(err) = publisher.update_shared_task_by_id(workspace.task_id).await
                {
                    tracing::warn!(
                        ?err,
                        "Failed to propagate shared task update for {}",
                        workspace.task_id
                    );
                }

                // Execute auto-start triggers for dependent tasks
                self.execute_task_triggers(workspace.task_id).await;
            }
        }

        Ok(())
    }

    /// Execute auto-start triggers for a completed task
    async fn execute_task_triggers(&self, completed_task_id: uuid::Uuid) {
        let pool = &self.db.pool;

        // Find all triggers waiting for this task
        let triggers = match TaskTrigger::find_by_trigger_task(pool, completed_task_id).await {
            Ok(triggers) => triggers,
            Err(e) => {
                error!("Failed to find triggers for task {}: {}", completed_task_id, e);
                return;
            }
        };

        for trigger in triggers {
            // For PR monitor, only handle "merged" condition since that's what we're detecting
            // (The trigger.trigger_on contains the condition string)
            if trigger.trigger_on != "merged" && trigger.trigger_on != "completed" {
                continue;
            }

            info!(
                "Executing trigger {} for task {} (waiting task: {})",
                trigger.id,
                completed_task_id,
                trigger.task_id
            );

            // Get the task to auto-start
            let task = match Task::find_by_id(pool, trigger.task_id).await {
                Ok(Some(task)) => task,
                Ok(None) => {
                    tracing::warn!("Trigger {} references non-existent task {}", trigger.id, trigger.task_id);
                    continue;
                }
                Err(e) => {
                    error!("Failed to find task {} for trigger: {}", trigger.task_id, e);
                    continue;
                }
            };

            // Find the project to get its board_id
            let project = match Project::find_by_id(pool, task.project_id).await {
                Ok(Some(project)) => project,
                Ok(None) => {
                    tracing::warn!("No project found for task {}", task.id);
                    continue;
                }
                Err(e) => {
                    error!("Failed to find project {}: {}", task.project_id, e);
                    continue;
                }
            };

            // Get the board from the project
            let board_id = match project.board_id {
                Some(id) => id,
                None => {
                    tracing::warn!("Project {} has no board assigned", task.project_id);
                    continue;
                }
            };

            let board = match Board::find_by_id(pool, board_id).await {
                Ok(Some(board)) => board,
                Ok(None) => {
                    tracing::warn!("No board found with id {}", board_id);
                    continue;
                }
                Err(e) => {
                    error!("Failed to find board {}: {}", board_id, e);
                    continue;
                }
            };

            // Find the first starts_workflow column
            let columns = match KanbanColumn::find_by_board(pool, board.id).await {
                Ok(cols) => cols,
                Err(e) => {
                    error!("Failed to find columns for board {}: {}", board.id, e);
                    continue;
                }
            };

            let workflow_column = columns.iter().find(|c| c.starts_workflow);
            let target_column = match workflow_column {
                Some(col) => col,
                None => {
                    tracing::warn!("No starts_workflow column found in board {}", board.id);
                    continue;
                }
            };

            // Move the task to the workflow-starting column
            if let Err(e) = Task::update_column_id(pool, task.id, Some(target_column.id)).await {
                error!(
                    "Failed to move task {} to column {}: {}",
                    task.id,
                    target_column.id,
                    e
                );
                continue;
            }

            info!(
                "Task {} auto-started via trigger {} (moved to column '{}')",
                task.id,
                trigger.id,
                target_column.name
            );

            // Delete non-persistent triggers after firing
            if !trigger.is_persistent {
                if let Err(e) = TaskTrigger::delete(pool, trigger.id).await {
                    error!("Failed to delete one-shot trigger {}: {}", trigger.id, e);
                }
            }
        }
    }
}

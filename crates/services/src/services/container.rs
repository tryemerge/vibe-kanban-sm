use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Error as AnyhowError, anyhow};
use async_trait::async_trait;
use db::{
    DBService,
    models::{
        agent::Agent,
        coding_agent_turn::{CodingAgentTurn, CreateCodingAgentTurn},
        context_artifact::ContextArtifact,
        execution_process::{
            CreateExecutionProcess, ExecutionContext, ExecutionProcess, ExecutionProcessRunReason,
            ExecutionProcessStatus,
        },
        execution_process_logs::ExecutionProcessLogs,
        execution_process_repo_state::{
            CreateExecutionProcessRepoState, ExecutionProcessRepoState,
        },
        kanban_column::KanbanColumn,
        project::{Project, UpdateProject},
        project_repo::{ProjectRepo, ProjectRepoWithName},
        repo::Repo,
        session::{CreateSession, Session, SessionError},
        state_transition::StateTransition,
        tag::Tag,
        task::{Task, TaskStatus},
        task_event::{ActorType, CreateTaskEvent, EventTriggerType, TaskEvent},
        workspace::{Workspace, WorkspaceError},
        workspace_repo::WorkspaceRepo,
    },
};
use executors::{
    actions::{
        ExecutorAction, ExecutorActionType,
        coding_agent_initial::CodingAgentInitialRequest,
        script::{ScriptContext, ScriptRequest, ScriptRequestLanguage},
    },
    executors::{ExecutorError, StandardCodingAgentExecutor},
    logs::{NormalizedEntry, NormalizedEntryError, NormalizedEntryType, utils::ConversationPatch},
    profile::{ExecutorConfigs, ExecutorProfileId},
};
use futures::{StreamExt, future};
use sqlx::Error as SqlxError;
use thiserror::Error;
use tokio::{sync::RwLock, task::JoinHandle};
use utils::{
    log_msg::LogMsg,
    msg_store::MsgStore,
    text::{git_branch_id, short_uuid},
};
use uuid::Uuid;

use crate::services::{
    git::{GitService, GitServiceError},
    notification::NotificationService,
    share::SharePublisher,
    workspace_manager::WorkspaceError as WorkspaceManagerError,
    worktree_manager::WorktreeError,
};
pub type ContainerRef = String;

#[derive(Debug, Error)]
pub enum ContainerError {
    #[error(transparent)]
    GitServiceError(#[from] GitServiceError),
    #[error(transparent)]
    Sqlx(#[from] SqlxError),
    #[error(transparent)]
    ExecutorError(#[from] ExecutorError),
    #[error(transparent)]
    Worktree(#[from] WorktreeError),
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    #[error(transparent)]
    WorkspaceManager(#[from] WorkspaceManagerError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to kill process: {0}")]
    KillFailed(std::io::Error),
    #[error(transparent)]
    Other(#[from] AnyhowError), // Catches any unclassified errors
}

/// Agent context for workflow execution
pub struct AgentContext {
    pub system_prompt: Option<String>,
    pub workflow_history: Option<String>,
    pub start_command: Option<String>,
    pub deliverable: Option<String>,
    pub name: String,
    pub color: Option<String>,
    pub column_name: String,
    /// Project-level context from context artifacts (module memories, ADRs, patterns)
    pub project_context: Option<String>,
}

/// Read the decision file (.vibe/decision.json) from a workspace
/// Returns the parsed JSON value if the file exists and is valid JSON
pub async fn read_decision_file(workspace: &Workspace) -> Option<serde_json::Value> {
    let worktree_path = workspace.container_ref.as_ref()?;
    let decision_path = PathBuf::from(worktree_path).join(".vibe/decision.json");

    if !decision_path.exists() {
        return None;
    }

    match tokio::fs::read_to_string(&decision_path).await {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(value) => Some(value),
            Err(e) => {
                tracing::warn!("Failed to parse decision file: {}", e);
                None
            }
        },
        Err(e) => {
            tracing::warn!("Failed to read decision file: {}", e);
            None
        }
    }
}

/// Build project context string from context artifacts (ADRs, patterns, recent decisions)
/// This provides project-level knowledge to agents when they start execution
async fn build_project_context(pool: &sqlx::SqlitePool, project_id: uuid::Uuid) -> Option<String> {
    let mut context = String::new();

    // Get recent ADRs (architecture decision records)
    if let Ok(adrs) = ContextArtifact::get_recent_adrs(pool, project_id, 5).await {
        if !adrs.is_empty() {
            context.push_str("## Architecture Decisions\n\n");
            for adr in adrs {
                context.push_str(&format!("### {}\n", adr.title));
                context.push_str(&adr.content);
                context.push_str("\n\n");
            }
        }
    }

    // Get patterns for this project
    if let Ok(patterns) = ContextArtifact::find_by_project_and_type(
        pool,
        project_id,
        &db::models::context_artifact::ArtifactType::Pattern,
    )
    .await
    {
        if !patterns.is_empty() {
            context.push_str("## Patterns & Best Practices\n\n");
            for pattern in patterns.iter().take(5) {
                context.push_str(&format!("### {}\n", pattern.title));
                context.push_str(&pattern.content);
                context.push_str("\n\n");
            }
        }
    }

    if context.is_empty() {
        None
    } else {
        Some(context)
    }
}

/// Result of evaluating a transition against a decision value
pub enum TransitionResult {
    /// Condition matched - use to_column_id (success path)
    Success(Uuid),
    /// Condition didn't match - use else_column_id (normal failure)
    Else(Uuid),
    /// Max failures reached - use escalation_column_id (emergency escalation)
    Escalation(Uuid),
    /// No match and no else/escalation path defined - stay in place
    NoMatch,
}

/// Evaluate a transition against the decision file and failure count.
/// Returns which destination column to use based on the new semantics:
/// - to_column_id: condition matched (success)
/// - else_column_id: condition didn't match, under failure limit (retry)
/// - escalation_column_id: condition didn't match, at/over failure limit (emergency)
fn evaluate_transition(
    transition: &StateTransition,
    decision: &Option<serde_json::Value>,
    failure_count: i64,
) -> TransitionResult {
    // Check if condition matches
    let condition_matches = match (&transition.condition_key, &transition.condition_value, decision) {
        (Some(key), Some(value), Some(dec)) => {
            dec.get(key)
                .and_then(|v| v.as_str())
                .map(|v| v == value)
                .unwrap_or(false)
        }
        // No condition defined - treat as unconditional success (unless requires confirmation)
        (None, None, _) => !transition.requires_confirmation,
        _ => false,
    };

    if condition_matches {
        // Success path - go to to_column_id
        return TransitionResult::Success(transition.to_column_id);
    }

    // Condition didn't match - check failure handling

    // Check if we should escalate (max_failures reached)
    if let Some(max_failures) = transition.max_failures {
        if failure_count >= max_failures {
            // Escalation path - go to escalation_column_id if set
            if let Some(escalation_col) = transition.escalation_column_id {
                return TransitionResult::Escalation(escalation_col);
            }
        }
    }

    // Normal failure path - go to else_column_id if set
    if let Some(else_col) = transition.else_column_id {
        return TransitionResult::Else(else_col);
    }

    // No else path defined - no automatic transition
    TransitionResult::NoMatch
}

/// Build decision instructions for an agent based on conditional transitions from a column.
/// This tells the agent what values to write to .vibe/decision.json to route the task.
/// Also includes feedback from a prior rejection if present in the existing decision file.
/// Uses hierarchical resolution: task-level > project-level > board-level transitions.
pub async fn build_decision_instructions(
    pool: &sqlx::SqlitePool,
    column_id: Uuid,
    task_id: Uuid,
    project_id: Uuid,
    board_id: Option<Uuid>,
    existing_decision: &Option<serde_json::Value>,
) -> Option<String> {
    // Get transitions from this column with hierarchical resolution
    let transitions = StateTransition::find_from_column_for_task(
        pool, column_id, task_id, project_id, board_id
    ).await.ok()?;

    // Filter to only conditional transitions (those with condition_key set)
    let conditional: Vec<_> = transitions
        .iter()
        .filter(|t| t.condition_key.is_some())
        .collect();

    if conditional.is_empty() {
        return None;
    }

    let mut instructions = String::new();
    instructions.push_str("\n\n---\n\n## Decision Required\n\n");
    instructions.push_str(
        "Before committing your changes, write your decision to `.vibe/decision.json`:\n\n",
    );

    for t in &conditional {
        if let (Some(key), Some(value)) = (&t.condition_key, &t.condition_value) {
            // Get target column name for clearer instructions
            let target_name = match KanbanColumn::find_by_id(pool, t.to_column_id).await {
                Ok(Some(col)) => col.name,
                _ => format!("column {}", t.to_column_id),
            };

            let transition_desc = t.name.as_deref().unwrap_or(&target_name);

            // Note if this is a loop prevention path
            // Note about failure handling
            let failure_note = if let Some(max_fails) = t.max_failures {
                if t.escalation_column_id.is_some() {
                    format!(" (escalates after {} failures)", max_fails)
                } else {
                    format!(" (max {} failures)", max_fails)
                }
            } else {
                String::new()
            };

            // Note about else path
            let else_note = if t.else_column_id.is_some() {
                " (has retry path)"
            } else {
                ""
            };

            instructions.push_str(&format!(
                "- `{{\"{}\": \"{}\"}}` → **{}**{}{}\n",
                key, value, transition_desc, failure_note, else_note
            ));
        }
    }

    instructions.push_str("\nExample:\n```json\n");
    // Use first conditional as example
    if let Some(first) = conditional.first() {
        if let (Some(key), Some(value)) = (&first.condition_key, &first.condition_value) {
            instructions.push_str(&format!("{{\"{}\": \"{}\"}}\n", key, value));
        }
    }
    instructions.push_str("```\n");

    // Include feedback from prior rejection if present
    if let Some(decision) = existing_decision {
        if let Some(feedback) = decision.get("feedback").and_then(|f| f.as_str()) {
            instructions.push_str("\n### Feedback from Previous Review\n\n");
            instructions.push_str(feedback);
            instructions.push_str("\n\nPlease address this feedback before proceeding.\n");
        }
    }

    Some(instructions)
}

#[async_trait]
pub trait ContainerService {
    fn msg_stores(&self) -> &Arc<RwLock<HashMap<Uuid, Arc<MsgStore>>>>;

    fn db(&self) -> &DBService;

    fn git(&self) -> &GitService;

    fn share_publisher(&self) -> Option<&SharePublisher>;

    fn notification_service(&self) -> &NotificationService;

    fn workspace_to_current_dir(&self, workspace: &Workspace) -> PathBuf;

    async fn create(&self, workspace: &Workspace) -> Result<ContainerRef, ContainerError>;

    async fn kill_all_running_processes(&self) -> Result<(), ContainerError>;

    async fn delete(&self, workspace: &Workspace) -> Result<(), ContainerError>;

    /// Check if a task has any running execution processes
    async fn has_running_processes(&self, task_id: Uuid) -> Result<bool, ContainerError> {
        let workspaces = Workspace::fetch_all(&self.db().pool, Some(task_id)).await?;

        for workspace in workspaces {
            let sessions = Session::find_by_workspace_id(&self.db().pool, workspace.id).await?;
            for session in sessions {
                if let Ok(processes) =
                    ExecutionProcess::find_by_session_id(&self.db().pool, session.id, false).await
                {
                    for process in processes {
                        if process.status == ExecutionProcessStatus::Running {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// A context is finalized when
    /// - Always when the execution process has failed or been killed
    /// - Never when the run reason is DevServer
    /// - Never when a setup script has no next_action (parallel mode)
    /// - The next action is None (no follow-up actions)
    fn should_finalize(&self, ctx: &ExecutionContext) -> bool {
        // Never finalize DevServer processes
        if matches!(
            ctx.execution_process.run_reason,
            ExecutionProcessRunReason::DevServer
        ) {
            return false;
        }

        // Never finalize setup scripts without a next_action (parallel mode).
        // In sequential mode, setup scripts have next_action pointing to coding agent,
        // so they won't finalize anyway (handled by next_action.is_none() check below).
        let action = ctx.execution_process.executor_action().unwrap();
        if matches!(
            ctx.execution_process.run_reason,
            ExecutionProcessRunReason::SetupScript
        ) && action.next_action.is_none()
        {
            return false;
        }

        // Always finalize failed or killed executions, regardless of next action
        if matches!(
            ctx.execution_process.status,
            ExecutionProcessStatus::Failed | ExecutionProcessStatus::Killed
        ) {
            return true;
        }

        // Otherwise, finalize only if no next action
        action.next_action.is_none()
    }

    /// Finalize task execution by updating status to InReview and sending notifications.
    /// Also handles auto-transition to next column if configured.
    async fn finalize_task(
        &self,
        share_publisher: Option<&SharePublisher>,
        ctx: &ExecutionContext,
    ) {
        let pool = &self.db().pool;

        // Try to auto-transition if execution completed successfully
        let transitioned = if matches!(ctx.execution_process.status, ExecutionProcessStatus::Completed) {
            self.try_auto_transition(ctx).await
        } else {
            false
        };

        // Only update status to InReview if we didn't auto-transition
        // (auto-transition handles status update as part of column change)
        if !transitioned {
            match Task::update_status(pool, ctx.task.id, TaskStatus::InReview).await {
                Ok(_) => {
                    if let Some(publisher) = share_publisher
                        && let Err(err) = publisher.update_shared_task_by_id(ctx.task.id).await
                    {
                        tracing::warn!(
                            ?err,
                            "Failed to propagate shared task update for {}",
                            ctx.task.id
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to update task status to InReview: {e}");
                }
            }
        }

        // Skip notification if process was intentionally killed by user
        if matches!(ctx.execution_process.status, ExecutionProcessStatus::Killed) {
            return;
        }

        let title = format!("Task Complete: {}", ctx.task.title);
        let message = match ctx.execution_process.status {
            ExecutionProcessStatus::Completed => format!(
                "✅ '{}' completed successfully\nBranch: {:?}\nExecutor: {:?}",
                ctx.task.title, ctx.workspace.branch, ctx.session.executor
            ),
            ExecutionProcessStatus::Failed => format!(
                "❌ '{}' execution failed\nBranch: {:?}\nExecutor: {:?}",
                ctx.task.title, ctx.workspace.branch, ctx.session.executor
            ),
            _ => {
                tracing::warn!(
                    "Tried to notify workspace completion for {} but process is still running!",
                    ctx.workspace.id
                );
                return;
            }
        };
        self.notification_service().notify(&title, &message).await;
    }

    /// Try to auto-transition the task to the next column based on state transitions.
    /// Supports conditional transitions based on .vibe/decision.json file.
    /// Returns true if transition occurred, false otherwise.
    async fn try_auto_transition(&self, ctx: &ExecutionContext) -> bool {
        let pool = &self.db().pool;

        // Get current task to check its column
        let task = match Task::find_by_id(pool, ctx.task.id).await {
            Ok(Some(task)) => task,
            Ok(None) => {
                tracing::warn!("Task {} not found for auto-transition", ctx.task.id);
                return false;
            }
            Err(e) => {
                tracing::error!("Failed to fetch task for auto-transition: {}", e);
                return false;
            }
        };

        let Some(current_column_id) = task.column_id else {
            tracing::debug!("Task {} has no column, skipping auto-transition", task.id);
            return false;
        };

        // Get current column to find its board and position
        let current_column = match KanbanColumn::find_by_id(pool, current_column_id).await {
            Ok(Some(col)) => col,
            Ok(None) => {
                tracing::warn!("Current column {} not found", current_column_id);
                return false;
            }
            Err(e) => {
                tracing::error!("Failed to fetch current column: {}", e);
                return false;
            }
        };

        // Read decision file from workspace for conditional transitions
        let decision = read_decision_file(&ctx.workspace).await;
        if decision.is_some() {
            tracing::debug!(
                "Found decision file for task {}: {:?}",
                task.id,
                decision
            );
        }

        // Try state transitions first (with hierarchical resolution)
        let transitions = match StateTransition::find_from_column_for_task(
            pool,
            current_column_id,
            task.id,
            task.project_id,
            Some(current_column.board_id),
        ).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Failed to fetch transitions: {}", e);
                return false;
            }
        };

        // Find target column - either from explicit transition or by position fallback
        let target_column = if !transitions.is_empty() {
            // Count failures (times we previously took the else path from this column)
            // This is used for escalation logic
            let failure_count = TaskEvent::count_else_transitions(
                pool,
                task.id,
                current_column_id
            )
            .await
            .unwrap_or(0);

            tracing::debug!(
                "Task {} has {} prior failures from column {}",
                task.id,
                failure_count,
                current_column.name
            );

            // Evaluate each transition to find one that can route the task
            let mut target_column_id: Option<Uuid> = None;
            let mut transition_path = "unknown";

            for transition in &transitions {
                match evaluate_transition(transition, &decision, failure_count) {
                    TransitionResult::Success(col_id) => {
                        tracing::debug!(
                            "Transition '{}' matched (success) -> column {} for task {}",
                            transition.name.as_deref().unwrap_or("unnamed"),
                            col_id,
                            task.id
                        );
                        target_column_id = Some(col_id);
                        transition_path = "success";
                        break;
                    }
                    TransitionResult::Else(col_id) => {
                        tracing::debug!(
                            "Transition '{}' did not match, using else path -> column {} for task {}",
                            transition.name.as_deref().unwrap_or("unnamed"),
                            col_id,
                            task.id
                        );
                        target_column_id = Some(col_id);
                        transition_path = "else";
                        // Don't break - a later transition might have a matching condition
                        // Actually, we should use the first transition's else path
                        break;
                    }
                    TransitionResult::Escalation(col_id) => {
                        tracing::info!(
                            "Transition '{}' escalating (max failures reached) -> column {} for task {}",
                            transition.name.as_deref().unwrap_or("unnamed"),
                            col_id,
                            task.id
                        );
                        target_column_id = Some(col_id);
                        transition_path = "escalation";
                        break;
                    }
                    TransitionResult::NoMatch => {
                        // Try next transition
                        continue;
                    }
                }
            }

            let Some(col_id) = target_column_id else {
                tracing::debug!(
                    "No matching transition for task {} in column {} (decision: {:?})",
                    task.id, current_column.name, decision
                );
                return false;
            };

            // Record additional metadata for else transitions (for failure counting)
            if transition_path == "else" {
                // Record that this was an else path transition
                let event = CreateTaskEvent::else_transition(
                    task.id,
                    current_column_id,
                );
                if let Err(e) = TaskEvent::create(pool, &event).await {
                    tracing::error!("Failed to record else transition event: {}", e);
                }
            }

            match KanbanColumn::find_by_id(pool, col_id).await {
                Ok(Some(col)) => col,
                Ok(None) => {
                    tracing::error!("Target column {} not found", col_id);
                    return false;
                }
                Err(e) => {
                    tracing::error!("Failed to fetch target column: {}", e);
                    return false;
                }
            }
        } else {
            // Fallback: use column position order (next column by position)
            let columns = match KanbanColumn::find_by_board(pool, current_column.board_id).await {
                Ok(cols) => cols,
                Err(e) => {
                    tracing::error!("Failed to fetch board columns: {}", e);
                    return false;
                }
            };

            // Find next column by position
            let next_col = columns.iter()
                .filter(|c| c.position > current_column.position)
                .min_by_key(|c| c.position);

            let Some(col) = next_col else {
                tracing::debug!(
                    "No next column by position for task {} in column {} (position {})",
                    task.id, current_column_id, current_column.position
                );
                return false;
            };

            col.clone()
        };

        // Update task's column and status
        if let Err(e) = Task::update_column_id(pool, task.id, Some(target_column.id)).await {
            tracing::error!("Failed to update column for task {}: {}", task.id, e);
            return false;
        }

        if let Err(e) = Task::update_status(pool, task.id, target_column.status.clone()).await {
            tracing::error!("Failed to update status for task {}: {}", task.id, e);
            return false;
        }

        // Record column transition event
        let event = CreateTaskEvent::column_transition(
            task.id,
            Some(current_column_id),
            target_column.id,
            EventTriggerType::Automation,
            ActorType::System,
            None,
        );
        if let Err(e) = TaskEvent::create(pool, &event).await {
            tracing::error!("Failed to record auto-transition event: {}", e);
        }

        tracing::info!(
            "Auto-transitioned task {} from column '{}' to column '{}'",
            task.id,
            current_column.name,
            target_column.name
        );

        // If target column has an agent, start execution
        if let Some(agent_id) = target_column.agent_id {
            // Fetch the agent for context
            match Agent::find_by_id(pool, agent_id).await {
                Ok(Some(agent)) => {
                    if let Err(e) = self.start_agent_execution_for_task(&task, &agent, &target_column).await {
                        tracing::error!(
                            "Failed to auto-start agent execution for task {} in column '{}': {}",
                            task.id,
                            target_column.name,
                            e
                        );
                    }
                }
                Ok(None) => {
                    tracing::warn!("Agent {} not found for column {}", agent_id, target_column.name);
                }
                Err(e) => {
                    tracing::error!("Failed to fetch agent {}: {}", agent_id, e);
                }
            }
        }

        true
    }

    /// Start agent execution for a task (used by auto-transition)
    async fn start_agent_execution_for_task(
        &self,
        task: &Task,
        agent: &Agent,
        column: &KanbanColumn,
    ) -> Result<(), ContainerError> {
        let pool = &self.db().pool;
        let column_id = column.id;
        let board_id = column.board_id;
        let column_name = &column.name;

        // Expand @tagname references in agent start_command and column deliverable
        let expanded_start_command = Tag::expand_tags_optional(pool, agent.start_command.as_deref()).await;
        let expanded_deliverable = Tag::expand_tags_optional(pool, column.deliverable.as_deref()).await;

        // Get the existing active workspace to reuse for next column's agent
        // For auto-transition, we continue with the existing workspace
        let workspace = Workspace::find_active_for_task(pool, task.id).await?;

        if let Some(workspace) = workspace {
            // Parse executor from agent
            use std::str::FromStr;
            use executors::executors::BaseCodingAgent;

            let base_agent = BaseCodingAgent::from_str(&agent.executor)
                .map_err(|e| anyhow!("Failed to parse executor '{}': {}", agent.executor, e))?;
            let executor_profile_id = ExecutorProfileId::new(base_agent);

            // Read existing decision file for any feedback from prior rejection
            let existing_decision = read_decision_file(&workspace).await;

            // Build decision instructions if this column has conditional transitions
            // Uses hierarchical resolution: task -> project -> board
            let decision_instructions = build_decision_instructions(
                pool,
                column_id,
                task.id,
                task.project_id,
                Some(board_id),
                &existing_decision,
            ).await;

            // Combine agent's start_command (with tags expanded) with decision instructions
            let start_command = match (&expanded_start_command, &decision_instructions) {
                (Some(cmd), Some(instructions)) => Some(format!("{}{}", cmd, instructions)),
                (Some(cmd), None) => Some(cmd.clone()),
                (None, Some(instructions)) => Some(instructions.clone()),
                (None, None) => None,
            };

            // Build workflow history showing prior work from other columns
            let workflow_history = match TaskEvent::build_workflow_history(pool, task.id).await {
                Ok(history) if !history.is_empty() => Some(history),
                _ => None,
            };

            // Build project context from context artifacts (ADRs, patterns)
            let project_context = build_project_context(pool, task.project_id).await;

            // Start execution with agent context
            // Deliverable comes from the column (what this stage should produce), with tags expanded
            let agent_context = AgentContext {
                system_prompt: Some(agent.system_prompt.clone()),
                workflow_history,
                start_command,
                deliverable: expanded_deliverable.clone(),
                name: agent.name.clone(),
                color: agent.color.clone(),
                column_name: column_name.to_string(),
                project_context,
            };
            self.start_workspace_with_agent_context(
                &workspace,
                executor_profile_id.clone(),
                agent_context,
            )
            .await?;

            // Record agent start event
            let agent_event = CreateTaskEvent {
                task_id: task.id,
                event_type: db::models::task_event::TaskEventType::AgentStart,
                from_column_id: None,
                to_column_id: None,
                workspace_id: Some(workspace.id),
                session_id: None,
                executor: Some(executor_profile_id.to_string()),
                automation_rule_id: None,
                trigger_type: Some(EventTriggerType::Automation),
                commit_hash: None,
                commit_message: None,
                metadata: None,
                actor_type: Some(ActorType::System),
                actor_id: None,
            };
            if let Err(e) = TaskEvent::create(pool, &agent_event).await {
                tracing::error!("Failed to record agent start event: {}", e);
            }

            tracing::info!(
                "Auto-started agent '{}' for task {} in workspace {}",
                agent.name,
                task.id,
                workspace.id
            );
        } else {
            tracing::debug!(
                "No workspace found for task {} to continue with agent",
                task.id
            );
        }

        Ok(())
    }

    /// Cleanup executions marked as running in the db, call at startup
    async fn cleanup_orphan_executions(&self) -> Result<(), ContainerError> {
        let running_processes = ExecutionProcess::find_running(&self.db().pool).await?;
        for process in running_processes {
            tracing::info!(
                "Found orphaned execution process {} for session {}",
                process.id,
                process.session_id
            );
            // Update the execution process status first
            if let Err(e) = ExecutionProcess::update_completion(
                &self.db().pool,
                process.id,
                ExecutionProcessStatus::Failed,
                None, // No exit code for orphaned processes
            )
            .await
            {
                tracing::error!(
                    "Failed to update orphaned execution process {} status: {}",
                    process.id,
                    e
                );
                continue;
            }
            // Capture after-head commit OID per repository
            if let Ok(ctx) = ExecutionProcess::load_context(&self.db().pool, process.id).await
                && let Some(ref container_ref) = ctx.workspace.container_ref
            {
                let workspace_root = PathBuf::from(container_ref);
                for repo in &ctx.repos {
                    let repo_path = workspace_root.join(&repo.name);
                    if let Ok(head) = self.git().get_head_info(&repo_path)
                        && let Err(err) = ExecutionProcessRepoState::update_after_head_commit(
                            &self.db().pool,
                            process.id,
                            repo.id,
                            &head.oid,
                        )
                        .await
                    {
                        tracing::warn!(
                            "Failed to update after_head_commit for repo {} on process {}: {}",
                            repo.id,
                            process.id,
                            err
                        );
                    }
                }
            }
            // Process marked as failed
            tracing::info!("Marked orphaned execution process {} as failed", process.id);
            // Update task status to InReview for coding agent and setup script failures
            if matches!(
                process.run_reason,
                ExecutionProcessRunReason::CodingAgent
                    | ExecutionProcessRunReason::SetupScript
                    | ExecutionProcessRunReason::CleanupScript
            ) && let Ok(Some(session)) =
                Session::find_by_id(&self.db().pool, process.session_id).await
                && let Ok(Some(workspace)) =
                    Workspace::find_by_id(&self.db().pool, session.workspace_id).await
                && let Ok(Some(task)) = workspace.parent_task(&self.db().pool).await
            {
                match Task::update_status(&self.db().pool, task.id, TaskStatus::InReview).await {
                    Ok(_) => {
                        if let Some(publisher) = self.share_publisher()
                            && let Err(err) = publisher.update_shared_task_by_id(task.id).await
                        {
                            tracing::warn!(
                                ?err,
                                "Failed to propagate shared task update for {}",
                                task.id
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to update task status to InReview for orphaned session: {}",
                            e
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// Backfill before_head_commit for legacy execution processes.
    /// Rules:
    /// - If a process has after_head_commit and missing before_head_commit,
    ///   then set before_head_commit to the previous process's after_head_commit.
    /// - If there is no previous process, set before_head_commit to the base branch commit.
    async fn backfill_before_head_commits(&self) -> Result<(), ContainerError> {
        let pool = &self.db().pool;
        let rows = ExecutionProcess::list_missing_before_context(pool).await?;
        for row in rows {
            // Skip if no after commit at all (shouldn't happen due to WHERE)
            // Prefer previous process after-commit if present
            let mut before = row.prev_after_head_commit.clone();

            // Fallback to base branch commit OID
            if before.is_none() {
                let repo_path = std::path::Path::new(row.repo_path.as_deref().unwrap_or_default());
                match self
                    .git()
                    .get_branch_oid(repo_path, row.target_branch.as_str())
                {
                    Ok(oid) => before = Some(oid),
                    Err(e) => {
                        tracing::warn!(
                            "Backfill: Failed to resolve base branch OID for workspace {} (branch {}): {}",
                            row.workspace_id,
                            row.target_branch,
                            e
                        );
                    }
                }
            }

            if let Some(before_oid) = before
                && let Err(e) = ExecutionProcessRepoState::update_before_head_commit(
                    pool,
                    row.id,
                    row.repo_id,
                    &before_oid,
                )
                .await
            {
                tracing::warn!(
                    "Backfill: Failed to update before_head_commit for process {}: {}",
                    row.id,
                    e
                );
            }
        }

        Ok(())
    }

    /// Backfill repo names that were migrated with a sentinel placeholder.
    /// Also backfills dev_script_working_dir and agent_working_dir for single-repo projects.
    async fn backfill_repo_names(&self) -> Result<(), ContainerError> {
        let pool = &self.db().pool;
        let repos = Repo::list_needing_name_fix(pool).await?;

        if repos.is_empty() {
            return Ok(());
        }

        tracing::info!("Backfilling {} repo names", repos.len());

        for repo in repos {
            let name = repo
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&repo.id.to_string())
                .to_string();

            Repo::update_name(pool, repo.id, &name, &name).await?;

            // Also update dev_script_working_dir and agent_working_dir for single-repo projects
            let project_repos = ProjectRepo::find_by_repo_id(pool, repo.id).await?;
            for pr in project_repos {
                let all_repos = ProjectRepo::find_by_project_id(pool, pr.project_id).await?;
                if all_repos.len() == 1
                    && let Some(project) = Project::find_by_id(pool, pr.project_id).await?
                {
                    let needs_dev_script_working_dir = project
                        .dev_script
                        .as_ref()
                        .map(|s| !s.is_empty())
                        .unwrap_or(false)
                        && project
                            .dev_script_working_dir
                            .as_ref()
                            .map(|s| s.is_empty())
                            .unwrap_or(true);

                    let needs_default_agent_working_dir = project
                        .default_agent_working_dir
                        .as_ref()
                        .map(|s| s.is_empty())
                        .unwrap_or(true);

                    if needs_dev_script_working_dir || needs_default_agent_working_dir {
                        Project::update(
                            pool,
                            pr.project_id,
                            &UpdateProject {
                                name: Some(project.name.clone()),
                                dev_script: project.dev_script.clone(),
                                dev_script_working_dir: if needs_dev_script_working_dir {
                                    Some(name.clone())
                                } else {
                                    project.dev_script_working_dir.clone()
                                },
                                default_agent_working_dir: if needs_default_agent_working_dir {
                                    Some(name.clone())
                                } else {
                                    project.default_agent_working_dir.clone()
                                },
                                board_id: None,
                            },
                        )
                        .await?;
                    }
                }
            }
        }

        Ok(())
    }

    fn cleanup_actions_for_repos(&self, repos: &[ProjectRepoWithName]) -> Option<ExecutorAction> {
        let repos_with_cleanup: Vec<_> = repos
            .iter()
            .filter(|r| r.cleanup_script.is_some())
            .collect();

        if repos_with_cleanup.is_empty() {
            return None;
        }

        let mut iter = repos_with_cleanup.iter();
        let first = iter.next()?;
        let mut root_action = ExecutorAction::new(
            ExecutorActionType::ScriptRequest(ScriptRequest {
                script: first.cleanup_script.clone().unwrap(),
                language: ScriptRequestLanguage::Bash,
                context: ScriptContext::CleanupScript,
                working_dir: Some(first.repo_name.clone()),
            }),
            None,
        );

        for repo in iter {
            root_action = root_action.append_action(ExecutorAction::new(
                ExecutorActionType::ScriptRequest(ScriptRequest {
                    script: repo.cleanup_script.clone().unwrap(),
                    language: ScriptRequestLanguage::Bash,
                    context: ScriptContext::CleanupScript,
                    working_dir: Some(repo.repo_name.clone()),
                }),
                None,
            ));
        }

        Some(root_action)
    }

    fn setup_actions_for_repos(&self, repos: &[ProjectRepoWithName]) -> Option<ExecutorAction> {
        let repos_with_setup: Vec<_> = repos.iter().filter(|r| r.setup_script.is_some()).collect();

        if repos_with_setup.is_empty() {
            return None;
        }

        let mut iter = repos_with_setup.iter();
        let first = iter.next()?;
        let mut root_action = ExecutorAction::new(
            ExecutorActionType::ScriptRequest(ScriptRequest {
                script: first.setup_script.clone().unwrap(),
                language: ScriptRequestLanguage::Bash,
                context: ScriptContext::SetupScript,
                working_dir: Some(first.repo_name.clone()),
            }),
            None,
        );

        for repo in iter {
            root_action = root_action.append_action(ExecutorAction::new(
                ExecutorActionType::ScriptRequest(ScriptRequest {
                    script: repo.setup_script.clone().unwrap(),
                    language: ScriptRequestLanguage::Bash,
                    context: ScriptContext::SetupScript,
                    working_dir: Some(repo.repo_name.clone()),
                }),
                None,
            ));
        }

        Some(root_action)
    }

    fn setup_action_for_repo(repo: &ProjectRepoWithName) -> Option<ExecutorAction> {
        repo.setup_script.as_ref().map(|script| {
            ExecutorAction::new(
                ExecutorActionType::ScriptRequest(ScriptRequest {
                    script: script.clone(),
                    language: ScriptRequestLanguage::Bash,
                    context: ScriptContext::SetupScript,
                    working_dir: Some(repo.repo_name.clone()),
                }),
                None,
            )
        })
    }

    fn build_sequential_setup_chain(
        repos: &[&ProjectRepoWithName],
        next_action: ExecutorAction,
    ) -> ExecutorAction {
        let mut chained = next_action;
        for repo in repos.iter().rev() {
            if let Some(script) = &repo.setup_script {
                chained = ExecutorAction::new(
                    ExecutorActionType::ScriptRequest(ScriptRequest {
                        script: script.clone(),
                        language: ScriptRequestLanguage::Bash,
                        context: ScriptContext::SetupScript,
                        working_dir: Some(repo.repo_name.clone()),
                    }),
                    Some(Box::new(chained)),
                );
            }
        }
        chained
    }

    async fn try_stop(&self, workspace: &Workspace, include_dev_server: bool) {
        // stop execution processes for this workspace's sessions
        let sessions = match Session::find_by_workspace_id(&self.db().pool, workspace.id).await {
            Ok(s) => s,
            Err(_) => return,
        };

        for session in sessions {
            if let Ok(processes) =
                ExecutionProcess::find_by_session_id(&self.db().pool, session.id, false).await
            {
                for process in processes {
                    // Skip dev server processes unless explicitly included
                    if !include_dev_server
                        && process.run_reason == ExecutionProcessRunReason::DevServer
                    {
                        continue;
                    }
                    if process.status == ExecutionProcessStatus::Running {
                        self.stop_execution(&process, ExecutionProcessStatus::Killed)
                            .await
                            .unwrap_or_else(|e| {
                                tracing::debug!(
                                    "Failed to stop execution process {} for workspace {}: {}",
                                    process.id,
                                    workspace.id,
                                    e
                                );
                            });
                    }
                }
            }
        }
    }

    async fn ensure_container_exists(
        &self,
        workspace: &Workspace,
    ) -> Result<ContainerRef, ContainerError>;

    async fn is_container_clean(&self, workspace: &Workspace) -> Result<bool, ContainerError>;

    async fn start_execution_inner(
        &self,
        workspace: &Workspace,
        execution_process: &ExecutionProcess,
        executor_action: &ExecutorAction,
    ) -> Result<(), ContainerError>;

    async fn stop_execution(
        &self,
        execution_process: &ExecutionProcess,
        status: ExecutionProcessStatus,
    ) -> Result<(), ContainerError>;

    async fn try_commit_changes(&self, ctx: &ExecutionContext) -> Result<bool, ContainerError>;

    async fn copy_project_files(
        &self,
        source_dir: &Path,
        target_dir: &Path,
        copy_files: &str,
    ) -> Result<(), ContainerError>;

    /// Stream diff updates as LogMsg for WebSocket endpoints.
    async fn stream_diff(
        &self,
        workspace: &Workspace,
        stats_only: bool,
    ) -> Result<futures::stream::BoxStream<'static, Result<LogMsg, std::io::Error>>, ContainerError>;

    /// Fetch the MsgStore for a given execution ID, panicking if missing.
    async fn get_msg_store_by_id(&self, uuid: &Uuid) -> Option<Arc<MsgStore>> {
        let map = self.msg_stores().read().await;
        map.get(uuid).cloned()
    }

    async fn git_branch_prefix(&self) -> String;

    async fn git_branch_from_workspace(&self, workspace_id: &Uuid, task_title: &str) -> String {
        let task_title_id = git_branch_id(task_title);
        let prefix = self.git_branch_prefix().await;

        if prefix.is_empty() {
            format!("{}-{}", short_uuid(workspace_id), task_title_id)
        } else {
            format!("{}/{}-{}", prefix, short_uuid(workspace_id), task_title_id)
        }
    }

    async fn stream_raw_logs(
        &self,
        id: &Uuid,
    ) -> Option<futures::stream::BoxStream<'static, Result<LogMsg, std::io::Error>>> {
        if let Some(store) = self.get_msg_store_by_id(id).await {
            // First try in-memory store
            return Some(
                store
                    .history_plus_stream()
                    .filter(|msg| {
                        future::ready(matches!(
                            msg,
                            Ok(LogMsg::Stdout(..) | LogMsg::Stderr(..) | LogMsg::Finished)
                        ))
                    })
                    .boxed(),
            );
        } else {
            // Fallback: load from DB and create direct stream
            let log_records =
                match ExecutionProcessLogs::find_by_execution_id(&self.db().pool, *id).await {
                    Ok(records) if !records.is_empty() => records,
                    Ok(_) => return None, // No logs exist
                    Err(e) => {
                        tracing::error!("Failed to fetch logs for execution {}: {}", id, e);
                        return None;
                    }
                };

            let messages = match ExecutionProcessLogs::parse_logs(&log_records) {
                Ok(msgs) => msgs,
                Err(e) => {
                    tracing::error!("Failed to parse logs for execution {}: {}", id, e);
                    return None;
                }
            };

            // Direct stream from parsed messages
            let stream = futures::stream::iter(
                messages
                    .into_iter()
                    .filter(|m| matches!(m, LogMsg::Stdout(_) | LogMsg::Stderr(_)))
                    .chain(std::iter::once(LogMsg::Finished))
                    .map(Ok::<_, std::io::Error>),
            )
            .boxed();

            Some(stream)
        }
    }

    async fn stream_normalized_logs(
        &self,
        id: &Uuid,
    ) -> Option<futures::stream::BoxStream<'static, Result<LogMsg, std::io::Error>>> {
        // First try in-memory store (existing behavior)
        if let Some(store) = self.get_msg_store_by_id(id).await {
            Some(
                store
                    .history_plus_stream() // BoxStream<Result<LogMsg, io::Error>>
                    .filter(|msg| future::ready(matches!(msg, Ok(LogMsg::JsonPatch(..)))))
                    .chain(futures::stream::once(async {
                        Ok::<_, std::io::Error>(LogMsg::Finished)
                    }))
                    .boxed(),
            )
        } else {
            // Fallback: load from DB and normalize
            let log_records =
                match ExecutionProcessLogs::find_by_execution_id(&self.db().pool, *id).await {
                    Ok(records) if !records.is_empty() => records,
                    Ok(_) => return None, // No logs exist
                    Err(e) => {
                        tracing::error!("Failed to fetch logs for execution {}: {}", id, e);
                        return None;
                    }
                };

            let raw_messages = match ExecutionProcessLogs::parse_logs(&log_records) {
                Ok(msgs) => msgs,
                Err(e) => {
                    tracing::error!("Failed to parse logs for execution {}: {}", id, e);
                    return None;
                }
            };

            // Create temporary store and populate
            // Include JsonPatch messages (already normalized) and Stdout/Stderr (need normalization)
            let temp_store = Arc::new(MsgStore::new());
            for msg in raw_messages {
                if matches!(
                    msg,
                    LogMsg::Stdout(_) | LogMsg::Stderr(_) | LogMsg::JsonPatch(_)
                ) {
                    temp_store.push(msg);
                }
            }
            temp_store.push_finished();

            let process = match ExecutionProcess::find_by_id(&self.db().pool, *id).await {
                Ok(Some(process)) => process,
                Ok(None) => {
                    tracing::error!("No execution process found for ID: {}", id);
                    return None;
                }
                Err(e) => {
                    tracing::error!("Failed to fetch execution process {}: {}", id, e);
                    return None;
                }
            };

            // Get the workspace to determine correct directory
            let (workspace, _session) =
                match process.parent_workspace_and_session(&self.db().pool).await {
                    Ok(Some((workspace, session))) => (workspace, session),
                    Ok(None) => {
                        tracing::error!(
                            "No workspace/session found for session ID: {}",
                            process.session_id
                        );
                        return None;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to fetch workspace for session {}: {}",
                            process.session_id,
                            e
                        );
                        return None;
                    }
                };

            if let Err(err) = self.ensure_container_exists(&workspace).await {
                tracing::warn!(
                    "Failed to recreate worktree before log normalization for workspace {}: {}",
                    workspace.id,
                    err
                );
            }

            let current_dir = self.workspace_to_current_dir(&workspace);

            let executor_action = if let Ok(executor_action) = process.executor_action() {
                executor_action
            } else {
                tracing::error!(
                    "Failed to parse executor action: {:?}",
                    process.executor_action()
                );
                return None;
            };

            // Spawn normalizer on populated store
            match executor_action.typ() {
                ExecutorActionType::CodingAgentInitialRequest(request) => {
                    let executor = ExecutorConfigs::get_cached()
                        .get_coding_agent_or_default(&request.executor_profile_id);
                    executor.normalize_logs(temp_store.clone(), &current_dir);
                }
                ExecutorActionType::CodingAgentFollowUpRequest(request) => {
                    let executor = ExecutorConfigs::get_cached()
                        .get_coding_agent_or_default(&request.executor_profile_id);
                    executor.normalize_logs(temp_store.clone(), &current_dir);
                }
                _ => {
                    tracing::debug!(
                        "Executor action doesn't support log normalization: {:?}",
                        process.executor_action()
                    );
                    return None;
                }
            }
            Some(
                temp_store
                    .history_plus_stream()
                    .filter(|msg| future::ready(matches!(msg, Ok(LogMsg::JsonPatch(..)))))
                    .chain(futures::stream::once(async {
                        Ok::<_, std::io::Error>(LogMsg::Finished)
                    }))
                    .boxed(),
            )
        }
    }

    fn spawn_stream_raw_logs_to_db(&self, execution_id: &Uuid) -> JoinHandle<()> {
        let execution_id = *execution_id;
        let msg_stores = self.msg_stores().clone();
        let db = self.db().clone();

        tokio::spawn(async move {
            // Get the message store for this execution
            let store = {
                let map = msg_stores.read().await;
                map.get(&execution_id).cloned()
            };

            if let Some(store) = store {
                let mut stream = store.history_plus_stream();

                while let Some(Ok(msg)) = stream.next().await {
                    match &msg {
                        LogMsg::Stdout(_) | LogMsg::Stderr(_) => {
                            // Serialize this individual message as a JSONL line
                            match serde_json::to_string(&msg) {
                                Ok(jsonl_line) => {
                                    let jsonl_line_with_newline = format!("{jsonl_line}\n");

                                    // Append this line to the database
                                    if let Err(e) = ExecutionProcessLogs::append_log_line(
                                        &db.pool,
                                        execution_id,
                                        &jsonl_line_with_newline,
                                    )
                                    .await
                                    {
                                        tracing::error!(
                                            "Failed to append log line for execution {}: {}",
                                            execution_id,
                                            e
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to serialize log message for execution {}: {}",
                                        execution_id,
                                        e
                                    );
                                }
                            }
                        }
                        LogMsg::SessionId(agent_session_id) => {
                            // Append this line to the database
                            if let Err(e) = CodingAgentTurn::update_agent_session_id(
                                &db.pool,
                                execution_id,
                                agent_session_id,
                            )
                            .await
                            {
                                tracing::error!(
                                    "Failed to update agent_session_id {} for execution process {}: {}",
                                    agent_session_id,
                                    execution_id,
                                    e
                                );
                            }
                        }
                        LogMsg::Finished => {
                            break;
                        }
                        LogMsg::JsonPatch(_) => continue,
                    }
                }
            }
        })
    }

    async fn start_workspace(
        &self,
        workspace: &Workspace,
        executor_profile_id: ExecutorProfileId,
    ) -> Result<ExecutionProcess, ContainerError> {
        // Create container
        self.create(workspace).await?;

        // Get parent task
        let task = workspace
            .parent_task(&self.db().pool)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        // Get parent project
        let project = task
            .parent_project(&self.db().pool)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        let project_repos =
            ProjectRepo::find_by_project_id_with_names(&self.db().pool, project.id).await?;

        let workspace = Workspace::find_by_id(&self.db().pool, workspace.id)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        // Create a session for this workspace
        let session = Session::create(
            &self.db().pool,
            &CreateSession {
                executor: Some(executor_profile_id.executor.to_string()),
            },
            Uuid::new_v4(),
            workspace.id,
        )
        .await?;

        let prompt = task.to_prompt();

        let repos_with_setup: Vec<_> = project_repos
            .iter()
            .filter(|pr| pr.setup_script.is_some())
            .collect();

        let all_parallel = repos_with_setup.iter().all(|pr| pr.parallel_setup_script);

        let cleanup_action = self.cleanup_actions_for_repos(&project_repos);

        let working_dir = workspace
            .agent_working_dir
            .as_ref()
            .filter(|dir| !dir.is_empty())
            .cloned();

        let coding_action = ExecutorAction::new(
            ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
                prompt,
                executor_profile_id: executor_profile_id.clone(),
                working_dir,
                agent_system_prompt: None,
                agent_project_context: None,
                agent_workflow_history: None,
                agent_start_command: None,
                agent_deliverable: None,
            }),
            cleanup_action.map(Box::new),
        );

        let execution_process = if all_parallel {
            // All parallel: start each setup independently, then start coding agent
            for repo in &repos_with_setup {
                if let Some(action) = Self::setup_action_for_repo(repo)
                    && let Err(e) = self
                        .start_execution(
                            &workspace,
                            &session,
                            &action,
                            &ExecutionProcessRunReason::SetupScript,
                        )
                        .await
                {
                    tracing::warn!(?e, "Failed to start setup script in parallel mode");
                }
            }
            self.start_execution(
                &workspace,
                &session,
                &coding_action,
                &ExecutionProcessRunReason::CodingAgent,
            )
            .await?
        } else {
            // Any sequential: chain ALL setups → coding agent via next_action
            let main_action = Self::build_sequential_setup_chain(&repos_with_setup, coding_action);
            self.start_execution(
                &workspace,
                &session,
                &main_action,
                &ExecutionProcessRunReason::SetupScript,
            )
            .await?
        };

        Ok(execution_process)
    }

    /// Start workspace execution with agent context (system prompt and start command)
    /// This is used when a task enters a column with an assigned agent
    async fn start_workspace_with_agent_context(
        &self,
        workspace: &Workspace,
        executor_profile_id: ExecutorProfileId,
        agent_context: AgentContext,
    ) -> Result<ExecutionProcess, ContainerError> {
        // Create container
        self.create(workspace).await?;

        // Get parent task
        let task = workspace
            .parent_task(&self.db().pool)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        // Get parent project
        let project = task
            .parent_project(&self.db().pool)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        let project_repos =
            ProjectRepo::find_by_project_id_with_names(&self.db().pool, project.id).await?;

        let workspace = Workspace::find_by_id(&self.db().pool, workspace.id)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        // Create a session for this workspace
        let session = Session::create(
            &self.db().pool,
            &CreateSession {
                executor: Some(executor_profile_id.executor.to_string()),
            },
            Uuid::new_v4(),
            workspace.id,
        )
        .await?;

        let prompt = task.to_prompt();

        let repos_with_setup: Vec<_> = project_repos
            .iter()
            .filter(|pr| pr.setup_script.is_some())
            .collect();

        let all_parallel = repos_with_setup.iter().all(|pr| pr.parallel_setup_script);

        let cleanup_action = self.cleanup_actions_for_repos(&project_repos);

        let working_dir = workspace
            .agent_working_dir
            .as_ref()
            .filter(|dir| !dir.is_empty())
            .cloned();

        // Include agent context in the request
        let coding_action = ExecutorAction::new(
            ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
                prompt,
                executor_profile_id: executor_profile_id.clone(),
                working_dir,
                agent_system_prompt: agent_context.system_prompt,
                agent_project_context: agent_context.project_context,
                agent_workflow_history: agent_context.workflow_history,
                agent_start_command: agent_context.start_command,
                agent_deliverable: agent_context.deliverable,
            }),
            cleanup_action.map(Box::new),
        );

        let execution_process = if all_parallel {
            // All parallel: start each setup independently, then start coding agent
            for repo in &repos_with_setup {
                if let Some(action) = Self::setup_action_for_repo(repo)
                    && let Err(e) = self
                        .start_execution(
                            &workspace,
                            &session,
                            &action,
                            &ExecutionProcessRunReason::SetupScript,
                        )
                        .await
                {
                    tracing::warn!(?e, "Failed to start setup script in parallel mode");
                }
            }
            self.start_execution(
                &workspace,
                &session,
                &coding_action,
                &ExecutionProcessRunReason::CodingAgent,
            )
            .await?
        } else {
            // Any sequential: chain ALL setups → coding agent via next_action
            let main_action = Self::build_sequential_setup_chain(&repos_with_setup, coding_action);
            self.start_execution(
                &workspace,
                &session,
                &main_action,
                &ExecutionProcessRunReason::SetupScript,
            )
            .await?
        };

        // Emit AgentSwitch entry to announce the agent taking over
        let agent_switch_entry = NormalizedEntry {
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
            entry_type: NormalizedEntryType::AgentSwitch {
                agent_name: agent_context.name.clone(),
                agent_color: agent_context.color.clone(),
                column_name: agent_context.column_name.clone(),
            },
            content: format!("Switching to {} for {}", agent_context.name, agent_context.column_name),
            metadata: None,
            agent_id: None,
            agent_color: agent_context.color,
        };
        let patch = ConversationPatch::add_normalized_entry(0, agent_switch_entry);
        match serde_json::to_string::<LogMsg>(&LogMsg::JsonPatch(patch)) {
            Ok(json_line) => {
                if let Err(e) = ExecutionProcessLogs::append_log_line(
                    &self.db().pool,
                    execution_process.id,
                    &format!("{json_line}\n"),
                )
                .await
                {
                    tracing::error!("Failed to write AgentSwitch log entry: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize AgentSwitch entry: {}", e);
            }
        }

        Ok(execution_process)
    }

    async fn start_execution(
        &self,
        workspace: &Workspace,
        session: &Session,
        executor_action: &ExecutorAction,
        run_reason: &ExecutionProcessRunReason,
    ) -> Result<ExecutionProcess, ContainerError> {
        // Update task status to InProgress when starting an execution
        let task = workspace
            .parent_task(&self.db().pool)
            .await?
            .ok_or(SqlxError::RowNotFound)?;
        if task.status != TaskStatus::InProgress
            && run_reason != &ExecutionProcessRunReason::DevServer
        {
            Task::update_status(&self.db().pool, task.id, TaskStatus::InProgress).await?;

            if let Some(publisher) = self.share_publisher()
                && let Err(err) = publisher.update_shared_task_by_id(task.id).await
            {
                tracing::warn!(
                    ?err,
                    "Failed to propagate shared task update for {}",
                    task.id
                );
            }
        }
        // Create new execution process record
        // Capture current HEAD per repository as the "before" commit for this execution
        let repositories =
            WorkspaceRepo::find_repos_for_workspace(&self.db().pool, workspace.id).await?;
        if repositories.is_empty() {
            return Err(ContainerError::Other(anyhow!(
                "Workspace has no repositories configured"
            )));
        }

        let workspace_root = workspace
            .container_ref
            .as_ref()
            .map(std::path::PathBuf::from)
            .ok_or_else(|| ContainerError::Other(anyhow!("Container ref not found")))?;

        let mut repo_states = Vec::with_capacity(repositories.len());
        for repo in &repositories {
            let repo_path = workspace_root.join(&repo.name);
            let before_head_commit = self.git().get_head_info(&repo_path).ok().map(|h| h.oid);
            repo_states.push(CreateExecutionProcessRepoState {
                repo_id: repo.id,
                before_head_commit,
                after_head_commit: None,
                merge_commit: None,
            });
        }
        let create_execution_process = CreateExecutionProcess {
            session_id: session.id,
            executor_action: executor_action.clone(),
            run_reason: run_reason.clone(),
        };

        let execution_process = ExecutionProcess::create(
            &self.db().pool,
            &create_execution_process,
            Uuid::new_v4(),
            &repo_states,
        )
        .await?;

        if let Some(prompt) = match executor_action.typ() {
            ExecutorActionType::CodingAgentInitialRequest(coding_agent_request) => {
                Some(coding_agent_request.prompt.clone())
            }
            ExecutorActionType::CodingAgentFollowUpRequest(follow_up_request) => {
                Some(follow_up_request.prompt.clone())
            }
            _ => None,
        } {
            let create_coding_agent_turn = CreateCodingAgentTurn {
                execution_process_id: execution_process.id,
                prompt: Some(prompt),
            };

            let coding_agent_turn_id = Uuid::new_v4();

            CodingAgentTurn::create(
                &self.db().pool,
                &create_coding_agent_turn,
                coding_agent_turn_id,
            )
            .await?;
        }

        if let Err(start_error) = self
            .start_execution_inner(workspace, &execution_process, executor_action)
            .await
        {
            // Mark process as failed
            if let Err(update_error) = ExecutionProcess::update_completion(
                &self.db().pool,
                execution_process.id,
                ExecutionProcessStatus::Failed,
                None,
            )
            .await
            {
                tracing::error!(
                    "Failed to mark execution process {} as failed after start error: {}",
                    execution_process.id,
                    update_error
                );
            }
            Task::update_status(&self.db().pool, task.id, TaskStatus::InReview).await?;

            // Emit stderr error message
            let log_message = LogMsg::Stderr(format!("Failed to start execution: {start_error}"));
            if let Ok(json_line) = serde_json::to_string(&log_message) {
                let _ = ExecutionProcessLogs::append_log_line(
                    &self.db().pool,
                    execution_process.id,
                    &format!("{json_line}\n"),
                )
                .await;
            }

            // Emit NextAction with failure context for coding agent requests
            if let ContainerError::ExecutorError(ExecutorError::ExecutableNotFound { program }) =
                &start_error
            {
                let help_text = format!("The required executable `{program}` is not installed.");
                let error_message = NormalizedEntry {
                    timestamp: None,
                    entry_type: NormalizedEntryType::ErrorMessage {
                        error_type: NormalizedEntryError::SetupRequired,
                    },
                    content: help_text,
                    metadata: None,
                    agent_id: None,
                    agent_color: None,
                };
                let patch = ConversationPatch::add_normalized_entry(2, error_message);
                if let Ok(json_line) = serde_json::to_string::<LogMsg>(&LogMsg::JsonPatch(patch)) {
                    let _ = ExecutionProcessLogs::append_log_line(
                        &self.db().pool,
                        execution_process.id,
                        &format!("{json_line}\n"),
                    )
                    .await;
                }
            };
            return Err(start_error);
        }

        // Start processing normalised logs for executor requests and follow ups
        if let Some(msg_store) = self.get_msg_store_by_id(&execution_process.id).await
            && let Some(executor_profile_id) = match executor_action.typ() {
                ExecutorActionType::CodingAgentInitialRequest(request) => {
                    Some(&request.executor_profile_id)
                }
                ExecutorActionType::CodingAgentFollowUpRequest(request) => {
                    Some(&request.executor_profile_id)
                }
                _ => None,
            }
        {
            if let Some(executor) =
                ExecutorConfigs::get_cached().get_coding_agent(executor_profile_id)
            {
                executor.normalize_logs(msg_store, &self.workspace_to_current_dir(workspace));
            } else {
                tracing::error!(
                    "Failed to resolve profile '{:?}' for normalization",
                    executor_profile_id
                );
            }
        }

        self.spawn_stream_raw_logs_to_db(&execution_process.id);
        Ok(execution_process)
    }

    async fn try_start_next_action(&self, ctx: &ExecutionContext) -> Result<(), ContainerError> {
        let action = ctx.execution_process.executor_action()?;
        let next_action = if let Some(next_action) = action.next_action() {
            next_action
        } else {
            tracing::debug!("No next action configured");
            return Ok(());
        };

        // Determine the run reason of the next action
        let next_run_reason = match (action.typ(), next_action.typ()) {
            (ExecutorActionType::ScriptRequest(_), ExecutorActionType::ScriptRequest(_)) => {
                ExecutionProcessRunReason::SetupScript
            }
            (
                ExecutorActionType::CodingAgentInitialRequest(_)
                | ExecutorActionType::CodingAgentFollowUpRequest(_),
                ExecutorActionType::ScriptRequest(_),
            ) => ExecutionProcessRunReason::CleanupScript,
            (
                _,
                ExecutorActionType::CodingAgentFollowUpRequest(_)
                | ExecutorActionType::CodingAgentInitialRequest(_),
            ) => ExecutionProcessRunReason::CodingAgent,
        };

        self.start_execution(&ctx.workspace, &ctx.session, next_action, &next_run_reason)
            .await?;

        tracing::debug!("Started next action: {:?}", next_action);
        Ok(())
    }
}

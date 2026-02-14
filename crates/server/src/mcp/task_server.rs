use std::{future::Future, str::FromStr};

use db::models::{
    project::Project,
    repo::Repo,
    tag::Tag,
    task::{CreateTask, Task, TaskStatus, TaskWithAttemptStatus, UpdateTask},
    workspace::{Workspace, WorkspaceContext},
};
use executors::{executors::BaseCodingAgent, profile::ExecutorProfileId};
use regex::Regex;
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::tool::{Parameters, ToolRouter},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json;
use uuid::Uuid;

use crate::routes::{
    containers::ContainerQuery,
    task_attempts::{CreateTaskAttemptBody, WorkspaceRepoInput},
};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateTaskRequest {
    #[schemars(description = "The ID of the project to create the task in. This is required!")]
    pub project_id: Uuid,
    #[schemars(description = "The title of the task")]
    pub title: String,
    #[schemars(description = "Optional description of the task")]
    pub description: Option<String>,
    #[schemars(description = "Optional labels to assign to the task. If a label doesn't exist, it will be created.")]
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CreateTaskResponse {
    pub task_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ProjectSummary {
    #[schemars(description = "The unique identifier of the project")]
    pub id: String,
    #[schemars(description = "The name of the project")]
    pub name: String,
    #[schemars(description = "When the project was created")]
    pub created_at: String,
    #[schemars(description = "When the project was last updated")]
    pub updated_at: String,
}

impl ProjectSummary {
    fn from_project(project: Project) -> Self {
        Self {
            id: project.id.to_string(),
            name: project.name,
            created_at: project.created_at.to_rfc3339(),
            updated_at: project.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct McpRepoSummary {
    #[schemars(description = "The unique identifier of the repository")]
    pub id: String,
    #[schemars(description = "The name of the repository")]
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListReposRequest {
    #[schemars(description = "The ID of the project to list repositories from")]
    pub project_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListReposResponse {
    pub repos: Vec<McpRepoSummary>,
    pub count: usize,
    pub project_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListProjectsResponse {
    pub projects: Vec<ProjectSummary>,
    pub count: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListTasksRequest {
    #[schemars(description = "The ID of the project to list tasks from")]
    pub project_id: Uuid,
    #[schemars(
        description = "Optional status filter: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'"
    )]
    pub status: Option<String>,
    #[schemars(description = "Maximum number of tasks to return (default: 50)")]
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TaskSummary {
    #[schemars(description = "The unique identifier of the task")]
    pub id: String,
    #[schemars(description = "The title of the task")]
    pub title: String,
    #[schemars(description = "Current status of the task")]
    pub status: String,
    #[schemars(description = "When the task was created")]
    pub created_at: String,
    #[schemars(description = "When the task was last updated")]
    pub updated_at: String,
    #[schemars(description = "Whether the task has an in-progress execution attempt")]
    pub has_in_progress_attempt: Option<bool>,
    #[schemars(description = "Whether the last execution attempt failed")]
    pub last_attempt_failed: Option<bool>,
}

impl TaskSummary {
    fn from_task_with_status(task: TaskWithAttemptStatus) -> Self {
        Self {
            id: task.id.to_string(),
            title: task.title.to_string(),
            status: task.status.to_string(),
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            has_in_progress_attempt: Some(task.has_in_progress_attempt),
            last_attempt_failed: Some(task.last_attempt_failed),
        }
    }
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TaskDetails {
    #[schemars(description = "The unique identifier of the task")]
    pub id: String,
    #[schemars(description = "The title of the task")]
    pub title: String,
    #[schemars(description = "Optional description of the task")]
    pub description: Option<String>,
    #[schemars(description = "Current status of the task")]
    pub status: String,
    #[schemars(description = "When the task was created")]
    pub created_at: String,
    #[schemars(description = "When the task was last updated")]
    pub updated_at: String,
    #[schemars(description = "Whether the task has an in-progress execution attempt")]
    pub has_in_progress_attempt: Option<bool>,
    #[schemars(description = "Whether the last execution attempt failed")]
    pub last_attempt_failed: Option<bool>,
    #[schemars(description = "Labels assigned to this task")]
    pub labels: Vec<String>,
}

impl TaskDetails {
    fn from_task(task: Task) -> Self {
        Self {
            id: task.id.to_string(),
            title: task.title,
            description: task.description,
            status: task.status.to_string(),
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            has_in_progress_attempt: None,
            last_attempt_failed: None,
            labels: vec![],
        }
    }

    fn from_task_with_labels(task: Task, labels: Vec<String>) -> Self {
        Self {
            id: task.id.to_string(),
            title: task.title,
            description: task.description,
            status: task.status.to_string(),
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            has_in_progress_attempt: None,
            last_attempt_failed: None,
            labels,
        }
    }
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksResponse {
    pub tasks: Vec<TaskSummary>,
    pub count: usize,
    pub project_id: String,
    pub applied_filters: ListTasksFilters,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksFilters {
    pub status: Option<String>,
    pub limit: i32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateTaskRequest {
    #[schemars(description = "The ID of the task to update")]
    pub task_id: Uuid,
    #[schemars(description = "New title for the task")]
    pub title: Option<String>,
    #[schemars(description = "New description for the task")]
    pub description: Option<String>,
    #[schemars(description = "New status: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'")]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct UpdateTaskResponse {
    pub task: TaskDetails,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteTaskRequest {
    #[schemars(description = "The ID of the task to delete")]
    pub task_id: Uuid,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct McpWorkspaceRepoInput {
    #[schemars(description = "The repository ID")]
    pub repo_id: Uuid,
    #[schemars(description = "The base branch for this repository")]
    pub base_branch: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StartWorkspaceSessionRequest {
    #[schemars(description = "The ID of the task to start")]
    pub task_id: Uuid,
    #[schemars(
        description = "The coding agent executor to run ('CLAUDE_CODE', 'CODEX', 'GEMINI', 'CURSOR_AGENT', 'OPENCODE')"
    )]
    pub executor: String,
    #[schemars(description = "Optional executor variant, if needed")]
    pub variant: Option<String>,
    #[schemars(description = "Base branch for each repository in the project")]
    pub repos: Vec<McpWorkspaceRepoInput>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct StartWorkspaceSessionResponse {
    pub task_id: String,
    pub workspace_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DeleteTaskResponse {
    pub deleted_task_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTaskRequest {
    #[schemars(description = "The ID of the task to retrieve")]
    pub task_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetTaskResponse {
    pub task: TaskDetails,
}

// ============================================
// Board Management Types
// ============================================

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct BoardSummary {
    #[schemars(description = "The unique identifier of the board")]
    pub id: String,
    #[schemars(description = "The name of the board")]
    pub name: String,
    #[schemars(description = "Optional description of the board")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListBoardsResponse {
    pub boards: Vec<BoardSummary>,
    pub count: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateBoardRequest {
    #[schemars(description = "The name of the board")]
    pub name: String,
    #[schemars(description = "Optional description of the board")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CreateBoardResponse {
    pub board_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBoardRequest {
    #[schemars(description = "The ID of the board to retrieve")]
    pub board_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ColumnSummary {
    #[schemars(description = "The unique identifier of the column")]
    pub id: String,
    #[schemars(description = "The name of the column")]
    pub name: String,
    #[schemars(description = "The slug/identifier for this column")]
    pub slug: String,
    #[schemars(description = "The color of the column")]
    pub color: Option<String>,
    #[schemars(description = "The workflow status (todo, inprogress, inreview, done, cancelled)")]
    pub status: String,
    #[schemars(description = "Whether this is the initial column")]
    pub is_initial: bool,
    #[schemars(description = "Whether this is a terminal column")]
    pub is_terminal: bool,
    #[schemars(description = "Whether entering this column starts a workflow")]
    pub starts_workflow: bool,
    #[schemars(description = "The agent ID assigned to this column, if any")]
    pub agent_id: Option<String>,
    #[schemars(description = "The position/order of this column")]
    pub position: i32,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TransitionSummary {
    #[schemars(description = "The unique identifier of the transition")]
    pub id: String,
    #[schemars(description = "The source column ID")]
    pub from_column_id: String,
    #[schemars(description = "The target column ID")]
    pub to_column_id: String,
    #[schemars(description = "Optional name for the transition")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetBoardResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub columns: Vec<ColumnSummary>,
    pub transitions: Vec<TransitionSummary>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateColumnRequest {
    #[schemars(description = "The ID of the board to add the column to")]
    pub board_id: Uuid,
    #[schemars(description = "The name of the column")]
    pub name: String,
    #[schemars(description = "The slug/identifier for this column (e.g., 'backlog', 'inprogress')")]
    pub slug: String,
    #[schemars(description = "The color of the column (hex format, e.g., '#3b82f6')")]
    pub color: Option<String>,
    #[schemars(description = "The workflow status: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'")]
    pub status: String,
    #[schemars(description = "Whether this is the initial column (where new tasks start)")]
    pub is_initial: Option<bool>,
    #[schemars(description = "Whether this is a terminal column (done/cancelled)")]
    pub is_terminal: Option<bool>,
    #[schemars(description = "Whether entering this column starts a workflow (triggers agent)")]
    pub starts_workflow: Option<bool>,
    #[schemars(description = "The agent ID to assign to this column")]
    pub agent_id: Option<Uuid>,
    #[schemars(description = "The position/order of this column (0-indexed)")]
    pub position: Option<i32>,
    #[schemars(description = "Description of what the agent should deliver when task leaves this column")]
    pub deliverable: Option<String>,
    #[schemars(description = "Variable name for structured deliverable in .vibe/decision.json (e.g., 'review_outcome'). Agent must set this variable.")]
    pub deliverable_variable: Option<String>,
    #[schemars(description = "JSON array of allowed values for the deliverable variable (e.g., '[\"approve\", \"request_changes\"]'). Transitions route based on these values.")]
    pub deliverable_options: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CreateColumnResponse {
    pub column_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateTransitionRequest {
    #[schemars(description = "The ID of the board")]
    pub board_id: Uuid,
    #[schemars(description = "The source column ID")]
    pub from_column_id: Uuid,
    #[schemars(description = "The target column ID")]
    pub to_column_id: Uuid,
    #[schemars(description = "Optional name for the transition (e.g., 'Approve', 'Reject')")]
    pub name: Option<String>,
    #[schemars(description = "JSON key to check in .vibe/decision.json for conditional routing (e.g., 'decision', 'review_outcome')")]
    pub condition_key: Option<String>,
    #[schemars(description = "Value to match for this transition (e.g., 'approve', 'reject'). When the decision file contains {condition_key: condition_value}, this transition is taken.")]
    pub condition_value: Option<String>,
    #[schemars(description = "Column ID to route to when condition doesn't match (else/retry path)")]
    pub else_column_id: Option<Uuid>,
    #[schemars(description = "Column ID to route to after max_failures is reached (escalation path)")]
    pub escalation_column_id: Option<Uuid>,
    #[schemars(description = "Number of times the else path can be taken before escalation")]
    pub max_failures: Option<i32>,
    #[schemars(description = "Whether this transition requires user confirmation before proceeding")]
    pub requires_confirmation: Option<bool>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CreateTransitionResponse {
    pub transition_id: String,
}

// ============================================
// Agent Management Types
// ============================================

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AgentSummary {
    #[schemars(description = "The unique identifier of the agent")]
    pub id: String,
    #[schemars(description = "The name of the agent")]
    pub name: String,
    #[schemars(description = "The executor type (CLAUDE_CODE, CODEX, GEMINI, etc.)")]
    pub executor: String,
    #[schemars(description = "The executor variant (e.g., OPUS, SONNET)")]
    pub variant: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentSummary>,
    pub count: usize,
}

// ============================================
// Project Management Types
// ============================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateProjectMcpRequest {
    #[schemars(description = "The ID of the project to update")]
    pub project_id: Uuid,
    #[schemars(description = "New name for the project")]
    pub name: Option<String>,
    #[schemars(description = "Board ID to assign to the project")]
    pub board_id: Option<Uuid>,
    #[schemars(description = "Setup script to run before agent starts")]
    pub setup_script: Option<String>,
    #[schemars(description = "Cleanup script to run after agent finishes")]
    pub cleanup_script: Option<String>,
    #[schemars(description = "Dev server script")]
    pub dev_script: Option<String>,
    #[schemars(description = "Working directory for the agent")]
    pub agent_working_dir: Option<String>,
    #[schemars(description = "Comma-separated list of files to copy to worktree")]
    pub copy_files: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct UpdateProjectMcpResponse {
    pub project_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetProjectRequest {
    #[schemars(description = "The ID of the project to retrieve")]
    pub project_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetProjectResponse {
    pub id: String,
    pub name: String,
    pub board_id: Option<String>,
    pub setup_script: Option<String>,
    pub cleanup_script: Option<String>,
    pub dev_script: Option<String>,
    pub agent_working_dir: Option<String>,
    pub copy_files: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateProjectRepoInput {
    #[schemars(description = "Display name for the repository")]
    pub display_name: String,
    #[schemars(description = "Path to the git repository on the local filesystem")]
    pub git_repo_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateProjectMcpRequest {
    #[schemars(description = "The name of the project")]
    pub name: String,
    #[schemars(
        description = "List of repositories to add to the project. Each repository needs a display_name and git_repo_path."
    )]
    pub repositories: Vec<CreateProjectRepoInput>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CreateProjectMcpResponse {
    pub project_id: String,
    pub project_name: String,
}

// ============================================================================
// Context Artifact MCP types
// ============================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateArtifactRequest {
    #[schemars(description = "The ID of the project to create the artifact in")]
    pub project_id: Uuid,
    #[schemars(description = "Type of artifact: 'adr', 'pattern', 'module_memory', 'decision', 'dependency', 'iplan', 'changelog_entry'")]
    pub artifact_type: String,
    #[schemars(description = "Human-readable title for the artifact")]
    pub title: String,
    #[schemars(description = "The artifact content (markdown)")]
    pub content: String,
    #[schemars(description = "Scope: 'global' (always injected), 'task' (specific task only), 'path' (file-path matched). Defaults to 'global'.")]
    pub scope: Option<String>,
    #[schemars(description = "File/module path (required for scope='path' or artifact_type='module_memory')")]
    pub path: Option<String>,
    #[schemars(description = "Source task ID that produced this artifact")]
    pub source_task_id: Option<Uuid>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CreateArtifactResponse {
    pub artifact_id: String,
    pub title: String,
    pub artifact_type: String,
    pub scope: String,
    pub token_estimate: i32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListArtifactsRequest {
    #[schemars(description = "The ID of the project to list artifacts from")]
    pub project_id: Uuid,
    #[schemars(description = "Optional type filter: 'adr', 'pattern', 'module_memory', 'decision', 'dependency', 'iplan', 'changelog_entry'")]
    pub artifact_type: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ArtifactSummary {
    pub id: String,
    pub artifact_type: String,
    pub title: String,
    pub scope: String,
    pub token_estimate: i32,
    pub path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListArtifactsResponse {
    pub artifacts: Vec<ArtifactSummary>,
    pub count: usize,
    pub project_id: String,
}

#[derive(Debug, Clone)]
pub struct TaskServer {
    client: reqwest::Client,
    base_url: String,
    tool_router: ToolRouter<TaskServer>,
    context: Option<McpContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct McpRepoContext {
    #[schemars(description = "The unique identifier of the repository")]
    pub repo_id: Uuid,
    #[schemars(description = "The name of the repository")]
    pub repo_name: String,
    #[schemars(description = "The target branch for this repository in this workspace")]
    pub target_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct McpColumnContext {
    #[schemars(description = "The unique identifier of the column")]
    pub column_id: Uuid,
    #[schemars(description = "The display name of the column")]
    pub column_name: String,
    #[schemars(description = "The slug/identifier for this column (e.g., 'inprogress', 'review')")]
    pub column_slug: String,
    #[schemars(description = "Whether this is an initial column (entry point)")]
    pub is_initial: bool,
    #[schemars(description = "Whether this is a terminal column (workflow end)")]
    pub is_terminal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct McpContext {
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub task_title: String,
    pub workspace_id: Uuid,
    pub workspace_branch: String,
    #[schemars(
        description = "Repository info and target branches for each repo in this workspace"
    )]
    pub workspace_repos: Vec<McpRepoContext>,
    #[schemars(description = "The current kanban column for this task (if assigned)")]
    pub column: Option<McpColumnContext>,
}

impl TaskServer {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
            tool_router: Self::tool_router(),
            context: None,
        }
    }

    pub async fn init(mut self) -> Self {
        let context = self.fetch_context_at_startup().await;

        if context.is_none() {
            self.tool_router.map.remove("get_context");
            tracing::debug!("VK context not available, get_context tool will not be registered");
        } else {
            tracing::info!("VK context loaded, get_context tool available");
        }

        self.context = context;
        self
    }

    async fn fetch_context_at_startup(&self) -> Option<McpContext> {
        let current_dir = std::env::current_dir().ok()?;
        let canonical_path = current_dir.canonicalize().unwrap_or(current_dir);
        let normalized_path = utils::path::normalize_macos_private_alias(&canonical_path);

        let url = self.url("/api/containers/attempt-context");
        let query = ContainerQuery {
            container_ref: normalized_path.to_string_lossy().to_string(),
        };

        let response = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            self.client.get(&url).query(&query).send(),
        )
        .await
        .ok()?
        .ok()?;

        if !response.status().is_success() {
            return None;
        }

        let api_response: ApiResponseEnvelope<WorkspaceContext> = response.json().await.ok()?;

        if !api_response.success {
            return None;
        }

        let ctx = api_response.data?;

        // Map RepoWithTargetBranch to McpRepoContext
        let workspace_repos: Vec<McpRepoContext> = ctx
            .workspace_repos
            .into_iter()
            .map(|rwb| McpRepoContext {
                repo_id: rwb.repo.id,
                repo_name: rwb.repo.name,
                target_branch: rwb.target_branch,
            })
            .collect();

        // Map column info if available
        let column = ctx.column.map(|col| McpColumnContext {
            column_id: col.id,
            column_name: col.name,
            column_slug: col.slug,
            is_initial: col.is_initial,
            is_terminal: col.is_terminal,
        });

        Some(McpContext {
            project_id: ctx.project.id,
            task_id: ctx.task.id,
            task_title: ctx.task.title,
            workspace_id: ctx.workspace.id,
            workspace_branch: ctx.workspace.branch,
            workspace_repos,
            column,
        })
    }
}

#[derive(Debug, Deserialize)]
struct ApiResponseEnvelope<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

/// Simplified label info for MCP operations
#[derive(Debug, Deserialize, Default)]
struct TaskLabelInfo {
    id: String,
    name: String,
}

impl TaskServer {
    /// Generate a consistent color for a label based on its name
    fn default_label_color(label_name: &str) -> &'static str {
        // Predefined color palette
        const COLORS: [&str; 9] = [
            "#ef4444", // red
            "#f97316", // orange
            "#eab308", // yellow
            "#22c55e", // green
            "#14b8a6", // teal
            "#3b82f6", // blue
            "#8b5cf6", // purple
            "#ec4899", // pink
            "#6b7280", // gray
        ];

        // Use a simple hash of the label name to pick a color
        let hash: usize = label_name
            .to_lowercase()
            .bytes()
            .fold(0usize, |acc, b| acc.wrapping_add(b as usize));
        COLORS[hash % COLORS.len()]
    }

    fn success<T: Serialize>(data: &T) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(data)
                .unwrap_or_else(|_| "Failed to serialize response".to_string()),
        )]))
    }

    fn err_value(v: serde_json::Value) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::error(vec![Content::text(
            serde_json::to_string_pretty(&v)
                .unwrap_or_else(|_| "Failed to serialize error".to_string()),
        )]))
    }

    fn err<S: Into<String>>(msg: S, details: Option<S>) -> Result<CallToolResult, ErrorData> {
        let mut v = serde_json::json!({"success": false, "error": msg.into()});
        if let Some(d) = details {
            v["details"] = serde_json::json!(d.into());
        };
        Self::err_value(v)
    }

    async fn send_json<T: DeserializeOwned>(
        &self,
        rb: reqwest::RequestBuilder,
    ) -> Result<T, CallToolResult> {
        let resp = rb
            .send()
            .await
            .map_err(|e| Self::err("Failed to connect to VK API", Some(&e.to_string())).unwrap())?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(
                Self::err(format!("VK API returned error status: {}", status), None).unwrap(),
            );
        }

        let api_response = resp.json::<ApiResponseEnvelope<T>>().await.map_err(|e| {
            Self::err("Failed to parse VK API response", Some(&e.to_string())).unwrap()
        })?;

        if !api_response.success {
            let msg = api_response.message.as_deref().unwrap_or("Unknown error");
            return Err(Self::err("VK API returned error", Some(msg)).unwrap());
        }

        api_response
            .data
            .ok_or_else(|| Self::err("VK API response missing data field", None).unwrap())
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    /// Expands @tagname references in text by replacing them with tag content.
    /// Returns the original text if expansion fails (e.g., network error).
    /// Unknown tags are left as-is (not expanded, not an error).
    async fn expand_tags(&self, text: &str) -> String {
        // Pattern matches @tagname where tagname is non-whitespace, non-@ characters
        let tag_pattern = match Regex::new(r"@([^\s@]+)") {
            Ok(re) => re,
            Err(_) => return text.to_string(),
        };

        // Find all unique tag names referenced in the text
        let tag_names: Vec<String> = tag_pattern
            .captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if tag_names.is_empty() {
            return text.to_string();
        }

        // Fetch all tags from the API
        let url = self.url("/api/tags");
        let tags: Vec<Tag> = match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<ApiResponseEnvelope<Vec<Tag>>>().await {
                    Ok(envelope) if envelope.success => envelope.data.unwrap_or_default(),
                    _ => return text.to_string(),
                }
            }
            _ => return text.to_string(),
        };

        // Build a map of tag_name -> content for quick lookup
        let tag_map: std::collections::HashMap<&str, &str> = tags
            .iter()
            .map(|t| (t.tag_name.as_str(), t.content.as_str()))
            .collect();

        // Replace each @tagname with its content (if found)
        let result = tag_pattern.replace_all(text, |caps: &regex::Captures| {
            let tag_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            match tag_map.get(tag_name) {
                Some(content) => (*content).to_string(),
                None => caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string(),
            }
        });

        result.into_owned()
    }
}

#[tool_router]
impl TaskServer {
    #[tool(
        description = "Return project, task, and workspace metadata for the current workspace session context."
    )]
    async fn get_context(&self) -> Result<CallToolResult, ErrorData> {
        // Context was fetched at startup and cached
        // This tool is only registered if context exists, so unwrap is safe
        let context = self.context.as_ref().expect("VK context should exist");
        TaskServer::success(context)
    }

    #[tool(
        description = "Create a new task/ticket in a project. Always pass the `project_id` of the project you want to create the task in - it is required! You can also pass `labels` (array of label names) to categorize the task - labels will be auto-created if they don't exist."
    )]
    async fn create_task(
        &self,
        Parameters(CreateTaskRequest {
            project_id,
            title,
            description,
            labels,
        }): Parameters<CreateTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        // Expand @tagname references in description
        let expanded_description = match description {
            Some(desc) => Some(self.expand_tags(&desc).await),
            None => None,
        };

        let url = self.url("/api/tasks");

        let task: Task = match self
            .send_json(
                self.client
                    .post(&url)
                    .json(&CreateTask::from_title_description(
                        project_id,
                        title,
                        expanded_description,
                    )),
            )
            .await
        {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        // Handle labels if provided
        if let Some(label_names) = labels {
            if !label_names.is_empty() {
                tracing::debug!("Processing {} labels for task {}", label_names.len(), task.id);

                // Get existing labels for the project
                let labels_url = self.url(&format!("/api/projects/{}/labels", project_id));
                let existing_labels: Vec<TaskLabelInfo> = match self
                    .send_json(self.client.get(&labels_url))
                    .await
                {
                    Ok(labels) => labels,
                    Err(e) => {
                        tracing::warn!("Failed to fetch existing labels for project {}: {:?}", project_id, e);
                        vec![]
                    }
                };

                let existing_label_map: std::collections::HashMap<String, String> = existing_labels
                    .iter()
                    .map(|l| (l.name.to_lowercase(), l.id.clone()))
                    .collect();

                for label_name in label_names {
                    let label_name_lower = label_name.to_lowercase();

                    // Check if label exists (case-insensitive)
                    let label_id = if let Some(id) = existing_label_map.get(&label_name_lower) {
                        id.clone()
                    } else {
                        // Create the label with a default color
                        let create_label_url = self.url(&format!("/api/projects/{}/labels", project_id));
                        let create_payload = serde_json::json!({
                            "name": label_name,
                            "color": Self::default_label_color(&label_name),
                            "position": existing_labels.len()
                        });

                        match self
                            .send_json::<TaskLabelInfo>(
                                self.client.post(&create_label_url).json(&create_payload),
                            )
                            .await
                        {
                            Ok(new_label) => new_label.id,
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to create label '{}' for task {}: {:?}",
                                    label_name, task.id, e
                                );
                                continue;
                            }
                        }
                    };

                    // Assign label to task
                    let assign_url = self.url(&format!("/api/tasks/{}/labels/{}", task.id, label_id));
                    let resp = self.client.post(&assign_url).send().await;
                    match resp {
                        Ok(r) if !r.status().is_success() => {
                            tracing::warn!(
                                "Failed to assign label {} to task {}: status {}",
                                label_id, task.id, r.status()
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to assign label {} to task {}: {:?}",
                                label_id, task.id, e
                            );
                        }
                        _ => {
                            tracing::debug!("Assigned label {} to task {}", label_id, task.id);
                        }
                    }
                }
            }
        }

        TaskServer::success(&CreateTaskResponse {
            task_id: task.id.to_string(),
        })
    }

    #[tool(description = "List all the available projects")]
    async fn list_projects(&self) -> Result<CallToolResult, ErrorData> {
        let url = self.url("/api/projects");
        let projects: Vec<Project> = match self.send_json(self.client.get(&url)).await {
            Ok(ps) => ps,
            Err(e) => return Ok(e),
        };

        let project_summaries: Vec<ProjectSummary> = projects
            .into_iter()
            .map(ProjectSummary::from_project)
            .collect();

        let response = ListProjectsResponse {
            count: project_summaries.len(),
            projects: project_summaries,
        };

        TaskServer::success(&response)
    }

    #[tool(description = "List all repositories for a project. `project_id` is required!")]
    async fn list_repos(
        &self,
        Parameters(ListReposRequest { project_id }): Parameters<ListReposRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/projects/{}/repositories", project_id));
        let repos: Vec<Repo> = match self.send_json(self.client.get(&url)).await {
            Ok(rs) => rs,
            Err(e) => return Ok(e),
        };

        let repo_summaries: Vec<McpRepoSummary> = repos
            .into_iter()
            .map(|r| McpRepoSummary {
                id: r.id.to_string(),
                name: r.name,
            })
            .collect();

        let response = ListReposResponse {
            count: repo_summaries.len(),
            repos: repo_summaries,
            project_id: project_id.to_string(),
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "List all the task/tickets in a project with optional filtering and execution status. `project_id` is required!"
    )]
    async fn list_tasks(
        &self,
        Parameters(ListTasksRequest {
            project_id,
            status,
            limit,
        }): Parameters<ListTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let status_filter = if let Some(ref status_str) = status {
            match TaskStatus::from_str(status_str) {
                Ok(s) => Some(s),
                Err(_) => {
                    return Self::err(
                        "Invalid status filter. Valid values: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'".to_string(),
                        Some(status_str.to_string()),
                    );
                }
            }
        } else {
            None
        };

        let url = self.url(&format!("/api/tasks?project_id={}", project_id));
        let all_tasks: Vec<TaskWithAttemptStatus> =
            match self.send_json(self.client.get(&url)).await {
                Ok(t) => t,
                Err(e) => return Ok(e),
            };

        let task_limit = limit.unwrap_or(50).max(0) as usize;
        let filtered = all_tasks.into_iter().filter(|t| {
            if let Some(ref want) = status_filter {
                &t.status == want
            } else {
                true
            }
        });
        let limited: Vec<TaskWithAttemptStatus> = filtered.take(task_limit).collect();

        let task_summaries: Vec<TaskSummary> = limited
            .into_iter()
            .map(TaskSummary::from_task_with_status)
            .collect();

        let response = ListTasksResponse {
            count: task_summaries.len(),
            tasks: task_summaries,
            project_id: project_id.to_string(),
            applied_filters: ListTasksFilters {
                status: status.clone(),
                limit: task_limit as i32,
            },
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Start working on a task by creating and launching a new workspace session."
    )]
    async fn start_workspace_session(
        &self,
        Parameters(StartWorkspaceSessionRequest {
            task_id,
            executor,
            variant,
            repos,
        }): Parameters<StartWorkspaceSessionRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        if repos.is_empty() {
            return Self::err(
                "At least one repository must be specified.".to_string(),
                None::<String>,
            );
        }

        let executor_trimmed = executor.trim();
        if executor_trimmed.is_empty() {
            return Self::err("Executor must not be empty.".to_string(), None::<String>);
        }

        let normalized_executor = executor_trimmed.replace('-', "_").to_ascii_uppercase();
        let base_executor = match BaseCodingAgent::from_str(&normalized_executor) {
            Ok(exec) => exec,
            Err(_) => {
                return Self::err(
                    format!("Unknown executor '{executor_trimmed}'."),
                    None::<String>,
                );
            }
        };

        let variant = variant.and_then(|v| {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        let executor_profile_id = ExecutorProfileId {
            executor: base_executor,
            variant,
        };

        let workspace_repos: Vec<WorkspaceRepoInput> = repos
            .into_iter()
            .map(|r| WorkspaceRepoInput {
                repo_id: r.repo_id,
                target_branch: r.base_branch,
            })
            .collect();

        let payload = CreateTaskAttemptBody {
            task_id,
            executor_profile_id,
            repos: workspace_repos,
        };

        let url = self.url("/api/task-attempts");
        let workspace: Workspace = match self.send_json(self.client.post(&url).json(&payload)).await
        {
            Ok(workspace) => workspace,
            Err(e) => return Ok(e),
        };

        let response = StartWorkspaceSessionResponse {
            task_id: workspace.task_id.to_string(),
            workspace_id: workspace.id.to_string(),
        };

        TaskServer::success(&response)
    }

    #[tool(
        description = "Update an existing task/ticket's title, description, or status. `project_id` and `task_id` are required! `title`, `description`, and `status` are optional."
    )]
    async fn update_task(
        &self,
        Parameters(UpdateTaskRequest {
            task_id,
            title,
            description,
            status,
        }): Parameters<UpdateTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let status = if let Some(ref status_str) = status {
            match TaskStatus::from_str(status_str) {
                Ok(s) => Some(s),
                Err(_) => {
                    return Self::err(
                        "Invalid status filter. Valid values: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'".to_string(),
                        Some(status_str.to_string()),
                    );
                }
            }
        } else {
            None
        };

        // Expand @tagname references in description
        let expanded_description = match description {
            Some(desc) => Some(self.expand_tags(&desc).await),
            None => None,
        };

        let payload = UpdateTask {
            title,
            description: expanded_description,
            status,
            column_id: None,
            parent_workspace_id: None,
            image_ids: None,
        };
        let url = self.url(&format!("/api/tasks/{}", task_id));
        let updated_task: Task = match self.send_json(self.client.put(&url).json(&payload)).await {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        let details = TaskDetails::from_task(updated_task);
        let repsonse = UpdateTaskResponse { task: details };
        TaskServer::success(&repsonse)
    }

    #[tool(
        description = "Delete a task/ticket from a project. `project_id` and `task_id` are required!"
    )]
    async fn delete_task(
        &self,
        Parameters(DeleteTaskRequest { task_id }): Parameters<DeleteTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/tasks/{}", task_id));
        if let Err(e) = self
            .send_json::<serde_json::Value>(self.client.delete(&url))
            .await
        {
            return Ok(e);
        }

        let repsonse = DeleteTaskResponse {
            deleted_task_id: Some(task_id.to_string()),
        };

        TaskServer::success(&repsonse)
    }

    #[tool(
        description = "Get detailed information (like task description and labels) about a specific task/ticket. You can use `list_tasks` to find the `task_ids` of all tasks in a project. `task_id` is required."
    )]
    async fn get_task(
        &self,
        Parameters(GetTaskRequest { task_id }): Parameters<GetTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/tasks/{}", task_id));
        let task: Task = match self.send_json(self.client.get(&url)).await {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        // Fetch labels for this task
        let labels_url = self.url(&format!("/api/tasks/{}/labels", task_id));
        let task_labels: Vec<TaskLabelInfo> = self
            .send_json(self.client.get(&labels_url))
            .await
            .unwrap_or_default();
        let label_names: Vec<String> = task_labels.into_iter().map(|l| l.name).collect();

        let details = TaskDetails::from_task_with_labels(task, label_names);
        let response = GetTaskResponse { task: details };

        TaskServer::success(&response)
    }

    // ============================================
    // Board Management Tools
    // ============================================

    #[tool(description = "List all available board templates")]
    async fn list_boards(&self) -> Result<CallToolResult, ErrorData> {
        let url = self.url("/api/boards");
        let boards: Vec<serde_json::Value> = match self.send_json(self.client.get(&url)).await {
            Ok(bs) => bs,
            Err(e) => return Ok(e),
        };

        let board_summaries: Vec<BoardSummary> = boards
            .into_iter()
            .map(|b| BoardSummary {
                id: b["id"].as_str().unwrap_or("").to_string(),
                name: b["name"].as_str().unwrap_or("").to_string(),
                description: b["description"].as_str().map(|s| s.to_string()),
            })
            .collect();

        let response = ListBoardsResponse {
            count: board_summaries.len(),
            boards: board_summaries,
        };

        TaskServer::success(&response)
    }

    #[tool(description = "Create a new board template")]
    async fn create_board(
        &self,
        Parameters(CreateBoardRequest { name, description }): Parameters<CreateBoardRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url("/api/boards");
        let payload = serde_json::json!({
            "name": name,
            "description": description
        });

        let board: serde_json::Value = match self
            .send_json(self.client.post(&url).json(&payload))
            .await
        {
            Ok(b) => b,
            Err(e) => return Ok(e),
        };

        TaskServer::success(&CreateBoardResponse {
            board_id: board["id"].as_str().unwrap_or("").to_string(),
        })
    }

    #[tool(description = "Get a board with its columns and transitions")]
    async fn get_board(
        &self,
        Parameters(GetBoardRequest { board_id }): Parameters<GetBoardRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        // Get board details
        let url = self.url(&format!("/api/boards/{}", board_id));
        let board: serde_json::Value = match self.send_json(self.client.get(&url)).await {
            Ok(b) => b,
            Err(e) => return Ok(e),
        };

        // Get columns
        let columns_url = self.url(&format!("/api/boards/{}/columns", board_id));
        let columns: Vec<serde_json::Value> = self
            .send_json(self.client.get(&columns_url))
            .await
            .unwrap_or_default();

        let column_summaries: Vec<ColumnSummary> = columns
            .into_iter()
            .map(|c| ColumnSummary {
                id: c["id"].as_str().unwrap_or("").to_string(),
                name: c["name"].as_str().unwrap_or("").to_string(),
                slug: c["slug"].as_str().unwrap_or("").to_string(),
                color: c["color"].as_str().map(|s| s.to_string()),
                status: c["status"].as_str().unwrap_or("todo").to_string(),
                is_initial: c["is_initial"].as_bool().unwrap_or(false),
                is_terminal: c["is_terminal"].as_bool().unwrap_or(false),
                starts_workflow: c["starts_workflow"].as_bool().unwrap_or(false),
                agent_id: c["agent_id"].as_str().map(|s| s.to_string()),
                position: c["position"].as_i64().unwrap_or(0) as i32,
            })
            .collect();

        // Get transitions
        let transitions_url = self.url(&format!("/api/boards/{}/transitions", board_id));
        let transitions: Vec<serde_json::Value> = self
            .send_json(self.client.get(&transitions_url))
            .await
            .unwrap_or_default();

        let transition_summaries: Vec<TransitionSummary> = transitions
            .into_iter()
            .map(|t| TransitionSummary {
                id: t["id"].as_str().unwrap_or("").to_string(),
                from_column_id: t["from_column_id"].as_str().unwrap_or("").to_string(),
                to_column_id: t["to_column_id"].as_str().unwrap_or("").to_string(),
                name: t["name"].as_str().map(|s| s.to_string()),
            })
            .collect();

        let response = GetBoardResponse {
            id: board["id"].as_str().unwrap_or("").to_string(),
            name: board["name"].as_str().unwrap_or("").to_string(),
            description: board["description"].as_str().map(|s| s.to_string()),
            columns: column_summaries,
            transitions: transition_summaries,
        };

        TaskServer::success(&response)
    }

    #[tool(description = "Create a column on a board. Use deliverable_variable and deliverable_options to define structured deliverables that agents must set in .vibe/decision.json.")]
    async fn create_column(
        &self,
        Parameters(CreateColumnRequest {
            board_id,
            name,
            slug,
            color,
            status,
            is_initial,
            is_terminal,
            starts_workflow,
            agent_id,
            position,
            deliverable,
            deliverable_variable,
            deliverable_options,
        }): Parameters<CreateColumnRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/boards/{}/columns", board_id));
        let payload = serde_json::json!({
            "name": name,
            "slug": slug,
            "color": color,
            "status": status,
            "is_initial": is_initial.unwrap_or(false),
            "is_terminal": is_terminal.unwrap_or(false),
            "starts_workflow": starts_workflow.unwrap_or(false),
            "agent_id": agent_id,
            "position": position,
            "deliverable": deliverable,
            "deliverable_variable": deliverable_variable,
            "deliverable_options": deliverable_options,
        });

        let column: serde_json::Value = match self
            .send_json(self.client.post(&url).json(&payload))
            .await
        {
            Ok(c) => c,
            Err(e) => return Ok(e),
        };

        TaskServer::success(&CreateColumnResponse {
            column_id: column["id"].as_str().unwrap_or("").to_string(),
        })
    }

    #[tool(description = "Create a state transition between columns on a board. Supports conditional routing: set condition_key and condition_value to route based on .vibe/decision.json values.")]
    async fn create_transition(
        &self,
        Parameters(CreateTransitionRequest {
            board_id,
            from_column_id,
            to_column_id,
            name,
            condition_key,
            condition_value,
            else_column_id,
            escalation_column_id,
            max_failures,
            requires_confirmation,
        }): Parameters<CreateTransitionRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/boards/{}/transitions", board_id));
        let payload = serde_json::json!({
            "from_column_id": from_column_id,
            "to_column_id": to_column_id,
            "name": name,
            "condition_key": condition_key,
            "condition_value": condition_value,
            "else_column_id": else_column_id,
            "escalation_column_id": escalation_column_id,
            "max_failures": max_failures,
            "requires_confirmation": requires_confirmation,
        });

        let transition: serde_json::Value = match self
            .send_json(self.client.post(&url).json(&payload))
            .await
        {
            Ok(t) => t,
            Err(e) => return Ok(e),
        };

        TaskServer::success(&CreateTransitionResponse {
            transition_id: transition["id"].as_str().unwrap_or("").to_string(),
        })
    }

    // ============================================
    // Agent Management Tools
    // ============================================

    #[tool(description = "List all available agents")]
    async fn list_agents(&self) -> Result<CallToolResult, ErrorData> {
        let url = self.url("/api/agents");
        let agents: Vec<serde_json::Value> = match self.send_json(self.client.get(&url)).await {
            Ok(a) => a,
            Err(e) => return Ok(e),
        };

        let agent_summaries: Vec<AgentSummary> = agents
            .into_iter()
            .map(|a| AgentSummary {
                id: a["id"].as_str().unwrap_or("").to_string(),
                name: a["name"].as_str().unwrap_or("").to_string(),
                executor: a["executor"].as_str().unwrap_or("").to_string(),
                variant: a["variant"].as_str().map(|s| s.to_string()),
            })
            .collect();

        let response = ListAgentsResponse {
            count: agent_summaries.len(),
            agents: agent_summaries,
        };

        TaskServer::success(&response)
    }

    // ============================================
    // Project Management Tools
    // ============================================

    #[tool(description = "Get project details including board and settings")]
    async fn get_project(
        &self,
        Parameters(GetProjectRequest { project_id }): Parameters<GetProjectRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/projects/{}", project_id));
        let project: serde_json::Value = match self.send_json(self.client.get(&url)).await {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        let response = GetProjectResponse {
            id: project["id"].as_str().unwrap_or("").to_string(),
            name: project["name"].as_str().unwrap_or("").to_string(),
            board_id: project["board_id"].as_str().map(|s| s.to_string()),
            setup_script: project["setup_script"].as_str().map(|s| s.to_string()),
            cleanup_script: project["cleanup_script"].as_str().map(|s| s.to_string()),
            dev_script: project["dev_script"].as_str().map(|s| s.to_string()),
            agent_working_dir: project["agent_working_dir"].as_str().map(|s| s.to_string()),
            copy_files: project["copy_files"].as_str().map(|s| s.to_string()),
        };

        TaskServer::success(&response)
    }

    #[tool(description = "Update project settings (board, scripts, agent_working_dir, copy_files)")]
    async fn update_project(
        &self,
        Parameters(UpdateProjectMcpRequest {
            project_id,
            name,
            board_id,
            setup_script,
            cleanup_script,
            dev_script,
            agent_working_dir,
            copy_files,
        }): Parameters<UpdateProjectMcpRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/projects/{}", project_id));
        let payload = serde_json::json!({
            "name": name,
            "board_id": board_id,
            "setup_script": setup_script,
            "cleanup_script": cleanup_script,
            "dev_script": dev_script,
            "agent_working_dir": agent_working_dir,
            "copy_files": copy_files
        });

        let _project: serde_json::Value = match self
            .send_json(self.client.put(&url).json(&payload))
            .await
        {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        TaskServer::success(&UpdateProjectMcpResponse {
            project_id: project_id.to_string(),
        })
    }

    #[tool(description = "Create a new project with one or more repositories")]
    async fn create_project(
        &self,
        Parameters(CreateProjectMcpRequest { name, repositories }): Parameters<CreateProjectMcpRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url("/api/projects");
        let repos_payload: Vec<serde_json::Value> = repositories
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "display_name": r.display_name,
                    "git_repo_path": r.git_repo_path
                })
            })
            .collect();

        let payload = serde_json::json!({
            "name": name,
            "repositories": repos_payload
        });

        let project: serde_json::Value = match self
            .send_json(self.client.post(&url).json(&payload))
            .await
        {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        TaskServer::success(&CreateProjectMcpResponse {
            project_id: project["id"].as_str().unwrap_or("").to_string(),
            project_name: project["name"].as_str().unwrap_or("").to_string(),
        })
    }

    #[tool(description = "Create a context artifact (ADR, pattern, decision, module memory, etc.) for a project. Artifacts are injected into future agent prompts based on their scope. Use scope='global' for project-wide knowledge, scope='task' for task-specific context, scope='path' for file-specific memories.")]
    async fn create_artifact(
        &self,
        Parameters(CreateArtifactRequest {
            project_id,
            artifact_type,
            title,
            content,
            scope,
            path,
            source_task_id,
        }): Parameters<CreateArtifactRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let valid_types = ["adr", "pattern", "module_memory", "decision", "dependency", "iplan", "changelog_entry"];
        if !valid_types.contains(&artifact_type.as_str()) {
            return Self::err(
                format!("Invalid artifact_type '{}'. Valid types: {}", artifact_type, valid_types.join(", ")),
                None::<String>,
            );
        }

        let scope_str = scope.unwrap_or_else(|| "global".to_string());
        let valid_scopes = ["global", "task", "path"];
        if !valid_scopes.contains(&scope_str.as_str()) {
            return Self::err(
                format!("Invalid scope '{}'. Valid scopes: {}", scope_str, valid_scopes.join(", ")),
                None::<String>,
            );
        }

        let url = self.url("/api/context-artifacts");
        let mut payload = serde_json::json!({
            "project_id": project_id.to_string(),
            "artifact_type": artifact_type,
            "title": title,
            "content": content,
            "scope": scope_str,
        });

        if let Some(p) = path {
            payload["path"] = serde_json::Value::String(p);
        }
        if let Some(tid) = source_task_id {
            payload["source_task_id"] = serde_json::Value::String(tid.to_string());
        }

        let artifact: serde_json::Value = match self
            .send_json(self.client.post(&url).json(&payload))
            .await
        {
            Ok(a) => a,
            Err(e) => return Ok(e),
        };

        TaskServer::success(&CreateArtifactResponse {
            artifact_id: artifact["id"].as_str().unwrap_or("").to_string(),
            title: artifact["title"].as_str().unwrap_or("").to_string(),
            artifact_type: artifact["artifact_type"].as_str().unwrap_or("").to_string(),
            scope: artifact["scope"].as_str().unwrap_or("").to_string(),
            token_estimate: artifact["token_estimate"].as_i64().unwrap_or(0) as i32,
        })
    }

    #[tool(description = "List context artifacts for a project, optionally filtered by type. Shows what knowledge will be injected into future agent prompts.")]
    async fn list_artifacts(
        &self,
        Parameters(ListArtifactsRequest {
            project_id,
            artifact_type,
        }): Parameters<ListArtifactsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let mut url = format!("/api/context-artifacts?project_id={}", project_id);
        if let Some(ref at) = artifact_type {
            url.push_str(&format!("&artifact_type={}", at));
        }
        let url = self.url(&url);

        let artifacts: Vec<serde_json::Value> = match self
            .send_json(self.client.get(&url))
            .await
        {
            Ok(a) => a,
            Err(e) => return Ok(e),
        };

        let summaries: Vec<ArtifactSummary> = artifacts
            .iter()
            .map(|a| ArtifactSummary {
                id: a["id"].as_str().unwrap_or("").to_string(),
                artifact_type: a["artifact_type"].as_str().unwrap_or("").to_string(),
                title: a["title"].as_str().unwrap_or("").to_string(),
                scope: a["scope"].as_str().unwrap_or("").to_string(),
                token_estimate: a["token_estimate"].as_i64().unwrap_or(0) as i32,
                path: a["path"].as_str().map(|s| s.to_string()),
                created_at: a["created_at"].as_str().unwrap_or("").to_string(),
                updated_at: a["updated_at"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        let count = summaries.len();
        TaskServer::success(&ListArtifactsResponse {
            artifacts: summaries,
            count,
            project_id: project_id.to_string(),
        })
    }
}

#[tool_handler]
impl ServerHandler for TaskServer {
    fn get_info(&self) -> ServerInfo {
        let mut instruction = "A task and project management server. If you need to create or update tickets or tasks then use these tools. Most of them absolutely require that you pass the `project_id` of the project that you are currently working on. You can get project ids by using `list projects`. Call `list_tasks` to fetch the `task_ids` of all the tasks in a project`.. TOOLS: 'list_projects', 'list_tasks', 'create_task', 'start_workspace_session', 'get_task', 'update_task', 'delete_task', 'list_repos', 'list_boards', 'create_board', 'get_board', 'create_column', 'create_transition', 'list_agents', 'get_project', 'update_project', 'create_project', 'create_artifact', 'list_artifacts'. Make sure to pass `project_id` or `task_id` where required. You can use list tools to get the available ids.".to_string();

        if let Some(ctx) = &self.context {
            let context_instruction = "Use 'get_context' to fetch project/task/workspace metadata for the active Vibe Kanban workspace session when available.";

            // Add workflow commit format instructions if we have column context
            let workflow_instruction = if ctx.column.is_some() {
                format!(
                    " WORKFLOW COMMIT FORMAT: When making commits for workflow tasks, use git trailers to track context. Format your commits as:\n\n<summary>\n\n<body with notes for next stage>\n\nTask-Id: {}\nColumn: <column-slug>\n\nThis allows workflow stages to be tracked and enables rollback by checking out prior commits.",
                    ctx.task_id
                )
            } else {
                String::new()
            };

            instruction = format!("{} {}{}", context_instruction, instruction, workflow_instruction);
        }

        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "vibe-kanban".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some(instruction),
        }
    }
}

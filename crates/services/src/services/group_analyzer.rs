use db::{
    DBService,
    models::{
        group_event::{CreateGroupEvent, GroupEvent},
        task::{CreateTask, Task},
        task_group::TaskGroup,
        workspace::Workspace,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::error::Error as SqlxError;
use std::path::Path;
use thiserror::Error;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum GroupAnalyzerError {
    #[error(transparent)]
    Sqlx(#[from] SqlxError),
    #[error("Task group not found")]
    GroupNotFound,
    #[error("Workspace not found for analysis task")]
    WorkspaceNotFound,
    #[error("Analysis file not found: {0}")]
    AnalysisFileNotFound(String),
    #[error("Failed to read analysis file: {0}")]
    AnalysisFileReadError(String),
    #[error("Failed to parse analysis JSON: {0}")]
    AnalysisParseError(String),
    #[error("Invalid recommendation: {0}")]
    InvalidRecommendation(String),
}

const GROUP_EVALUATOR_AGENT_ID: Uuid = uuid::uuid!("55555555-0000-0001-0001-000000000001");

/// Analysis results from the Group Evaluator agent
#[derive(Debug, Serialize, Deserialize)]
struct AnalysisResult {
    group_name: String,
    group_description: String,
    execution_dag: ExecutionDAG,
    gaps_identified: Vec<String>,
    questions: Vec<String>,
    recommendation: String, // "ready" or "needs_work"
}

#[derive(Debug, Serialize, Deserialize)]
struct ExecutionDAG {
    parallel_sets: Vec<Vec<String>>, // Vec of parallel sets, each containing task IDs
}

#[derive(Debug, Serialize, Deserialize)]
struct DecisionResult {
    answer: String, // "ready" or "needs_work"
}

/// Service for managing task group analysis lifecycle
pub struct GroupAnalyzer {
    db: DBService,
}

impl GroupAnalyzer {
    pub fn new(db: DBService) -> Self {
        Self { db }
    }

    /// Create analysis task when group enters 'analyzing' state
    /// This task will be assigned to the Group Evaluator agent
    pub async fn create_analysis_task(
        &self,
        group_id: Uuid,
    ) -> Result<Task, GroupAnalyzerError> {
        let group = TaskGroup::find_by_id(&self.db.pool, group_id)
            .await?
            .ok_or(GroupAnalyzerError::GroupNotFound)?;

        // Verify group is in analyzing state
        if group.state != "analyzing" {
            return Err(GroupAnalyzerError::Sqlx(sqlx::Error::Protocol(
                format!("Group {} is not in analyzing state (current: {})", group_id, group.state).into()
            )));
        }

        // Get all tasks in the group for context
        let group_tasks = TaskGroup::get_tasks(&self.db.pool, group_id).await?;

        // Build analysis task description with all group tasks
        let tasks_summary: Vec<String> = group_tasks
            .iter()
            .map(|t| format!("- {} ({})", t.title, t.id))
            .collect();

        let description = format!(
            r#"**Group Analysis Task**

Review the following {} tasks in group "{}" and prepare for execution:

{}

**Your job:**
1. Review all tasks to understand the full scope
2. Identify any gaps or missing tasks
3. Build an execution DAG (which tasks can run in parallel?)
4. Give the group a final descriptive name
5. Ask any clarifying questions
6. Wait for human approval

**Deliverables:**
- Create `.vibe/analysis.json` with your findings
- Create `.vibe/decision.json` with your recommendation

See your system prompt for full details and JSON format.
"#,
            group_tasks.len(),
            group.name,
            tasks_summary.join("\n")
        );

        // Create the analysis task
        let task_data = CreateTask {
            project_id: group.project_id,
            title: format!("Analyze: {}", group.name),
            description: Some(description),
            status: None, // Defaults to Todo
            column_id: None, // Not in workflow yet
            parent_workspace_id: None,
            image_ids: None,
            shared_task_id: None,
            task_group_id: Some(group_id),
        };

        let task_id = Uuid::new_v4();
        let task = Task::create(&self.db.pool, &task_data, task_id).await?;

        info!(
            "Created analysis task {} for group {} ({})",
            task.id, group.name, group_id
        );

        // Log the event
        let event = CreateGroupEvent {
            task_group_id: group_id,
            task_id: Some(task.id),
            event_type: "analysis_task_created".to_string(),
            actor_type: "system".to_string(),
            summary: format!(
                "Analysis task created for group '{}' - Group Evaluator will review {} tasks",
                group.name,
                group_tasks.len()
            ),
            payload: Some(json!({
                "analysis_task_id": task.id.to_string(),
                "task_count": group_tasks.len(),
                "agent_id": GROUP_EVALUATOR_AGENT_ID.to_string(),
            }).to_string()),
        };
        GroupEvent::create(&self.db.pool, &event).await?;

        Ok(task)
    }

    /// Handle completion of analysis task
    /// Parse the DAG, update group state, transition to ready or back to draft
    pub async fn handle_analysis_completion(
        &self,
        group_id: Uuid,
        analysis_task_id: Uuid,
    ) -> Result<(), GroupAnalyzerError> {
        let group = TaskGroup::find_by_id(&self.db.pool, group_id)
            .await?
            .ok_or(GroupAnalyzerError::GroupNotFound)?;

        // Find the workspace for the analysis task
        let workspace = Workspace::find_active_for_task(&self.db.pool, analysis_task_id)
            .await?
            .ok_or(GroupAnalyzerError::WorkspaceNotFound)?;

        // Get the worktree path (container_ref contains the worktree path)
        let worktree_path = workspace.container_ref
            .as_ref()
            .ok_or_else(|| GroupAnalyzerError::AnalysisFileNotFound("No worktree path found".to_string()))?;

        // Read analysis.json
        let analysis_path = Path::new(worktree_path).join(".vibe/analysis.json");
        let analysis_content = std::fs::read_to_string(&analysis_path)
            .map_err(|e| GroupAnalyzerError::AnalysisFileReadError(format!("analysis.json: {}", e)))?;

        let analysis: AnalysisResult = serde_json::from_str(&analysis_content)
            .map_err(|e| GroupAnalyzerError::AnalysisParseError(format!("analysis.json: {}", e)))?;

        // Read decision.json
        let decision_path = Path::new(worktree_path).join(".vibe/decision.json");
        let decision_content = std::fs::read_to_string(&decision_path)
            .map_err(|e| GroupAnalyzerError::AnalysisFileReadError(format!("decision.json: {}", e)))?;

        let decision: DecisionResult = serde_json::from_str(&decision_content)
            .map_err(|e| GroupAnalyzerError::AnalysisParseError(format!("decision.json: {}", e)))?;

        info!(
            "Analysis complete for group {} ({}): recommendation={}, final_name={}",
            group.name, group_id, decision.answer, analysis.group_name
        );

        // Log the analysis results
        let analysis_event = CreateGroupEvent {
            task_group_id: group_id,
            task_id: Some(analysis_task_id),
            event_type: "analysis_complete".to_string(),
            actor_type: "agent".to_string(),
            summary: format!(
                "Group Evaluator completed analysis: {} → '{}' (recommendation: {})",
                group.name, analysis.group_name, decision.answer
            ),
            payload: Some(json!({
                "original_name": group.name,
                "final_name": analysis.group_name,
                "recommendation": decision.answer,
                "gaps_identified": analysis.gaps_identified,
                "questions": analysis.questions,
                "parallel_sets_count": analysis.execution_dag.parallel_sets.len(),
            }).to_string()),
        };
        GroupEvent::create(&self.db.pool, &analysis_event).await?;

        // Handle based on recommendation
        match decision.answer.as_str() {
            "ready" => {
                // Update group name
                let update_query = sqlx::query(
                    "UPDATE task_groups SET name = $1 WHERE id = $2"
                )
                .bind(&analysis.group_name)
                .bind(group_id);
                update_query.execute(&self.db.pool).await?;

                // Store the execution DAG
                let dag_json = serde_json::to_string(&analysis.execution_dag)
                    .map_err(|e| GroupAnalyzerError::AnalysisParseError(format!("Failed to serialize DAG: {}", e)))?;

                TaskGroup::update_execution_dag(&self.db.pool, group_id, &dag_json).await?;

                // Transition analyzing → ready
                let updated_group = TaskGroup::transition_state(&self.db.pool, group_id, "analyzing", "ready")
                    .await?;

                if let Some(g) = updated_group {
                    info!(
                        "Group '{}' ({}) ready for execution with {} parallel sets",
                        g.name, group_id, analysis.execution_dag.parallel_sets.len()
                    );

                    // Log transition event
                    let transition_event = CreateGroupEvent {
                        task_group_id: group_id,
                        task_id: None,
                        event_type: "group_state_change".to_string(),
                        actor_type: "system".to_string(),
                        summary: format!(
                            "Group '{}' approved for execution (analyzing → ready)",
                            g.name
                        ),
                        payload: Some(json!({
                            "from": "analyzing",
                            "to": "ready",
                            "dag_parallel_sets": analysis.execution_dag.parallel_sets.len(),
                        }).to_string()),
                    };
                    GroupEvent::create(&self.db.pool, &transition_event).await?;
                } else {
                    warn!("Failed to transition group {} to ready state", group_id);
                }
            },
            "needs_work" => {
                // Transition analyzing → draft (so grouper can adjust)
                let updated_group = TaskGroup::transition_state(&self.db.pool, group_id, "analyzing", "failed")
                    .await?;

                if let Some(g) = updated_group {
                    info!(
                        "Group '{}' ({}) needs work, returned to failed state",
                        g.name, group_id
                    );

                    // Log transition event
                    let transition_event = CreateGroupEvent {
                        task_group_id: group_id,
                        task_id: None,
                        event_type: "group_state_change".to_string(),
                        actor_type: "system".to_string(),
                        summary: format!(
                            "Group '{}' needs work (analyzing → failed)",
                            g.name
                        ),
                        payload: Some(json!({
                            "from": "analyzing",
                            "to": "failed",
                            "gaps": analysis.gaps_identified,
                            "questions": analysis.questions,
                        }).to_string()),
                    };
                    GroupEvent::create(&self.db.pool, &transition_event).await?;
                }
            },
            other => {
                return Err(GroupAnalyzerError::InvalidRecommendation(
                    format!("Expected 'ready' or 'needs_work', got '{}'", other)
                ));
            }
        }

        Ok(())
    }
}

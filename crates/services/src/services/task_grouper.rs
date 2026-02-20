use std::time::Duration;

use db::{
    DBService,
    models::{
        agent::Agent,
        group_event::{CreateGroupEvent, GroupEvent},
        project::Project,
        task::{CreateTask, Task},
        task_group::{CreateTaskGroup, TaskGroup},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::error::Error as SqlxError;
use thiserror::Error;
use tokio::time::interval;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::services::analytics::AnalyticsContext;

#[derive(Debug, Error)]
pub enum TaskGrouperError {
    #[error(transparent)]
    Sqlx(#[from] SqlxError),
    #[error("Task Grouper agent not found")]
    AgentNotFound,
    #[allow(dead_code)]
    #[error("Claude API error: {0}")]
    ClaudeApiError(String),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}

const TASK_GROUPER_AGENT_ID: Uuid = uuid::uuid!("44444444-0000-0001-0001-000000000001");

/// Grouping recommendation from Claude
#[derive(Debug, Serialize, Deserialize)]
struct GroupingRecommendation {
    groups: Vec<RecommendedGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RecommendedGroup {
    name: String,
    description: String,
    color: Option<String>,
    task_ids: Vec<Uuid>,
    rationale: String,
}

/// Service to periodically analyze backlog and group related tasks
pub struct TaskGrouperService {
    db: DBService,
    poll_interval: Duration,
    analytics: Option<AnalyticsContext>,
}

impl TaskGrouperService {
    /// Create a new service instance for manual use (without background timer)
    pub fn new(db: DBService, analytics: Option<AnalyticsContext>) -> Self {
        Self {
            db,
            poll_interval: Duration::from_secs(300), // Run every 5 minutes (only used if spawned)
            analytics,
        }
    }

    /// Spawn the service as a background task with periodic execution
    pub async fn spawn(
        db: DBService,
        analytics: Option<AnalyticsContext>,
    ) -> tokio::task::JoinHandle<()> {
        let service = Self::new(db, analytics);
        tokio::spawn(async move {
            service.start().await;
        })
    }

    async fn start(&self) {
        info!(
            "Starting Task Grouper service with interval {:?}",
            self.poll_interval
        );

        // Verify agent exists
        if let Err(e) = self.verify_agent().await {
            error!("Task Grouper agent not found, service will not run: {}", e);
            return;
        }

        let mut interval = interval(self.poll_interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.analyze_all_backlogs().await {
                error!("Error analyzing backlogs: {}", e);
            }
        }
    }

    /// Verify the Task Grouper agent exists in the database
    async fn verify_agent(&self) -> Result<(), TaskGrouperError> {
        Agent::find_by_id(&self.db.pool, TASK_GROUPER_AGENT_ID)
            .await?
            .ok_or(TaskGrouperError::AgentNotFound)?;
        Ok(())
    }

    /// Analyze backlogs for all projects with ungrouped tasks
    async fn analyze_all_backlogs(&self) -> Result<(), TaskGrouperError> {
        // Get all projects
        let projects = Project::find_all(&self.db.pool).await?;

        if projects.is_empty() {
            debug!("No projects to analyze");
            return Ok(());
        }

        for project in projects {
            if let Err(e) = self.analyze_project_backlog(&project).await {
                error!(
                    "Error analyzing backlog for project {} ({}): {}",
                    project.name, project.id, e
                );
            }
        }

        Ok(())
    }

    /// Analyze backlog for a specific project (public for manual triggering via API)
    pub async fn analyze_project_backlog(&self, project: &Project) -> Result<(), TaskGrouperError> {
        // Query ungrouped tasks in this project
        let ungrouped_tasks = Task::find_ungrouped_by_project(&self.db.pool, project.id).await?;

        if ungrouped_tasks.is_empty() {
            debug!("No ungrouped tasks for project {}", project.name);
            return Ok(());
        }

        // Skip if too few tasks (need at least 2 to group)
        if ungrouped_tasks.len() < 2 {
            debug!(
                "Only {} ungrouped task in project {}, skipping grouping",
                ungrouped_tasks.len(),
                project.name
            );
            return Ok(());
        }

        info!(
            "Found {} ungrouped tasks in project {}, creating grouping task...",
            ungrouped_tasks.len(),
            project.name
        );

        // Verify Task Grouper agent exists
        let agent = Agent::find_by_id(&self.db.pool, TASK_GROUPER_AGENT_ID)
            .await?
            .ok_or(TaskGrouperError::AgentNotFound)?;

        // Build the task description with all ungrouped tasks
        let tasks_summary: Vec<String> = ungrouped_tasks
            .iter()
            .map(|t| {
                format!(
                    "- **{}** ({})\n  {}",
                    t.title,
                    t.id,
                    t.description.as_deref().unwrap_or("No description")
                )
            })
            .collect();

        let description = format!(
            r#"**Task Grouper Assignment**

Analyze the following {} ungrouped tasks in project "{}" and organize them into task groups.

## Ungrouped Tasks

{}

## Your Mission

Use the available MCP tools to:
1. Identify patterns and relationships between tasks
2. Create new task groups for related tasks (use simple names like "Auth System", "Payment Flow")
3. Assign tasks to the appropriate groups using `add_task_to_group`
4. Set inter-group dependencies when needed using `add_group_dependency`
5. Document your grouping rationale using `create_artifact`

## Guidelines

- Groups should have 3-8 related tasks
- Group by feature/domain, dependency, or purpose commonality
- Use SIMPLE, descriptive group names
- Don't force unrelated tasks into groups - leave them ungrouped if they don't fit
- The Group Evaluator will refine groups later, so focus on clustering similar work

## MCP Tools Available

- `list_tasks` - Query tasks in the project
- `get_task` - Get detailed task information
- `create_task_group` - Create a new group (name, color, project_id)
- `add_task_to_group` - Assign a task to a group
- `add_group_dependency` - Set prerequisite between groups
- `create_artifact` - Log your grouping decisions

See your system prompt for complete instructions.
"#,
            ungrouped_tasks.len(),
            project.name,
            tasks_summary.join("\n\n")
        );

        // Create the grouping task assigned to Task Grouper agent
        let task_data = CreateTask {
            project_id: project.id,
            title: format!("Group {} ungrouped tasks", ungrouped_tasks.len()),
            description: Some(description),
            status: None, // Defaults to Todo
            column_id: None, // Not in workflow columns
            parent_workspace_id: None,
            image_ids: None,
            shared_task_id: None,
            task_group_id: None, // System task, not in a group
        };

        let task_id = Uuid::new_v4();
        let grouping_task = Task::create(&self.db.pool, &task_data, task_id).await?;

        info!(
            "Created grouping task {} for project {} ({}) - assigned to agent '{}'",
            grouping_task.id,
            project.name,
            project.id,
            agent.name
        );

        // Track analytics event if available
        if let Some(analytics) = &self.analytics {
            analytics.analytics_service.track_event(
                &analytics.user_id,
                "grouping_task_created",
                Some(json!({
                    "project_id": project.id.to_string(),
                    "ungrouped_count": ungrouped_tasks.len(),
                    "grouping_task_id": grouping_task.id.to_string(),
                    "agent_id": TASK_GROUPER_AGENT_ID.to_string(),
                })),
            );
        }

        // The task is now ready to be executed by the Task Grouper agent via MCP
        // The agent will use MCP tools to create groups and assign tasks directly
        // No need to read results or apply groupings - the agent does it all!

        Ok(())
    }

    /// Apply grouping recommendations (creates groups and assigns tasks)
    #[allow(dead_code)]
    async fn apply_grouping(
        &self,
        project_id: Uuid,
        recommendations: GroupingRecommendation,
    ) -> Result<(), TaskGrouperError> {
        for rec_group in recommendations.groups {
            let task_count = rec_group.task_ids.len();

            // Create the task group
            let group_data = CreateTaskGroup {
                project_id,
                name: rec_group.name.clone(),
                color: rec_group.color.clone(),
                is_backlog: Some(false),
            };

            let group = TaskGroup::create(&self.db.pool, &group_data).await?;

            info!(
                "Created task group '{}' ({})",
                group.name, group.id
            );

            // Log group creation event
            let create_event = CreateGroupEvent {
                task_group_id: group.id,
                task_id: None,
                event_type: "group_created".to_string(),
                actor_type: "system".to_string(),
                summary: format!(
                    "Task Grouper created group '{}' with {} tasks",
                    group.name,
                    task_count
                ),
                payload: Some(json!({
                    "rationale": rec_group.rationale,
                    "task_count": task_count,
                }).to_string()),
            };
            GroupEvent::create(&self.db.pool, &create_event).await?;

            // Assign tasks to the group
            for task_id in rec_group.task_ids {
                if let Err(e) = sqlx::query(
                    "UPDATE tasks SET task_group_id = $1 WHERE id = $2"
                )
                .bind(group.id)
                .bind(task_id)
                .execute(&self.db.pool)
                .await
                {
                    error!("Failed to assign task {} to group {}: {}", task_id, group.id, e);
                    continue;
                }

                // Log task assignment event
                let assign_event = CreateGroupEvent {
                    task_group_id: group.id,
                    task_id: Some(task_id),
                    event_type: "task_added".to_string(),
                    actor_type: "system".to_string(),
                    summary: format!("Task assigned to group '{}'", group.name),
                    payload: None,
                };
                GroupEvent::create(&self.db.pool, &assign_event).await?;
            }

            info!(
                "Assigned {} tasks to group '{}'",
                task_count,
                group.name
            );
        }

        Ok(())
    }

    /// Manually trigger analysis for a specific project ID (for on-demand API calls)
    pub async fn analyze_project_by_id(
        &self,
        project_id: Uuid,
    ) -> Result<(), TaskGrouperError> {
        let project = Project::find_by_id(&self.db.pool, project_id)
            .await?
            .ok_or_else(|| {
                TaskGrouperError::Sqlx(SqlxError::RowNotFound)
            })?;

        self.analyze_project_backlog(&project).await
    }
}

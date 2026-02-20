use std::time::Duration;

use db::{
    DBService,
    models::{
        agent::Agent,
        group_event::{CreateGroupEvent, GroupEvent},
        project::Project,
        task::Task,
        task_group::{CreateTaskGroup, TaskGroup},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::error::Error as SqlxError;
use thiserror::Error;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
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
            "Found {} ungrouped tasks in project {}, analyzing...",
            ungrouped_tasks.len(),
            project.name
        );

        // Get the Task Grouper agent for its system prompt (will be used when Claude API is integrated)
        let _agent = Agent::find_by_id(&self.db.pool, TASK_GROUPER_AGENT_ID)
            .await?
            .ok_or(TaskGrouperError::AgentNotFound)?;

        // Build the analysis prompt
        let tasks_json: Vec<_> = ungrouped_tasks
            .iter()
            .map(|t| {
                json!({
                    "id": t.id.to_string(),
                    "title": t.title,
                    "description": t.description,
                    "status": format!("{:?}", t.status),
                })
            })
            .collect();

        let user_prompt = format!(
            "Analyze the following ungrouped tasks from project \"{}\" and recommend how to group them.

IMPORTANT: Your job is to CLUSTER related tasks together. Use simple, generic names like \"Auth-related tasks\" or \"Payment work\".
The group will be analyzed and given a final descriptive name later by the evaluator agent.

Tasks:
{}

Return your recommendations as JSON in this exact format:
{{
  \"groups\": [
    {{
      \"name\": \"Simple descriptive label (e.g., 'Auth work', 'Database tasks')\",
      \"description\": \"Brief note on what these tasks have in common\",
      \"color\": \"#hex-color\",
      \"task_ids\": [\"uuid1\", \"uuid2\"],
      \"rationale\": \"Why these tasks belong together\"
    }}
  ]
}}

Guidelines:
- Create groups of 3-8 related tasks
- Group by feature/domain, dependency, or purpose commonality
- Use SIMPLE names - they're just tracking labels
- Focus on identifying tasks that should be worked on together
- If tasks don't relate, leave them ungrouped (don't force it)
- The evaluator agent will refine the group later
",
            project.name,
            serde_json::to_string_pretty(&tasks_json)?
        );

        info!("Calling Claude API for grouping analysis...");

        // TODO: Integrate Claude API client
        // Options:
        // 1. Use anthropic-sdk-rust crate (if it exists)
        // 2. Direct HTTP calls with reqwest
        // 3. Spawn claude CLI as subprocess (like executors do)
        //
        // For now, grouping analysis is logged but not executed.
        // Once API integration is complete, call Claude with:
        // - System prompt from agent.system_prompt
        // - User prompt built above
        // - Parse JSON response into GroupingRecommendation
        // - Call self.apply_grouping() to create groups and assign tasks

        warn!(
            "Claude API integration pending. Would analyze {} tasks for grouping.",
            ungrouped_tasks.len(),
        );

        debug!("Grouping prompt:\n{}", user_prompt);

        // Track analytics event if available
        if let Some(analytics) = &self.analytics {
            analytics.analytics_service.track_event(
                &analytics.user_id,
                "grouping_analysis_requested",
                Some(json!({
                    "project_id": project.id.to_string(),
                    "ungrouped_count": ungrouped_tasks.len(),
                })),
            );
        }

        // TODO: Parse response, create groups, assign tasks, log events
        // This will be completed once we integrate Claude API

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

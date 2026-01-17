use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    actions::Executable,
    approvals::ExecutorApprovalService,
    env::ExecutionEnv,
    executors::{BaseCodingAgent, ExecutorError, SpawnedChild, StandardCodingAgentExecutor},
    profile::{ExecutorConfigs, ExecutorProfileId},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub struct CodingAgentInitialRequest {
    pub prompt: String,
    /// Executor profile specification
    #[serde(alias = "profile_variant_label")]
    // Backwards compatability with ProfileVariantIds, esp stored in DB under ExecutorAction
    pub executor_profile_id: ExecutorProfileId,
    /// Optional relative path to execute the agent in (relative to container_ref).
    /// If None, uses the container_ref directory directly.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Optional agent system prompt to prepend (establishes persona/role)
    #[serde(default)]
    pub agent_system_prompt: Option<String>,
    /// Optional project context from context artifacts (ADRs, patterns, module memories)
    #[serde(default)]
    pub agent_project_context: Option<String>,
    /// Optional workflow history showing prior work from other columns
    #[serde(default)]
    pub agent_workflow_history: Option<String>,
    /// Optional agent start command to append (initial instruction)
    #[serde(default)]
    pub agent_start_command: Option<String>,
    /// Optional deliverable description - tells the agent what to produce and when to stop
    #[serde(default)]
    pub agent_deliverable: Option<String>,
}

impl CodingAgentInitialRequest {
    pub fn base_executor(&self) -> BaseCodingAgent {
        self.executor_profile_id.executor
    }

    /// Build the full prompt with agent context prepended and start command appended
    pub fn build_full_prompt(&self) -> String {
        let mut full = String::new();

        // Prepend agent system prompt if present
        if let Some(system_prompt) = &self.agent_system_prompt {
            if !system_prompt.trim().is_empty() {
                full.push_str(system_prompt.trim());
                full.push_str("\n\n---\n\n");
            }
        }

        // Add project context if present (ADRs, patterns, module memories from context artifacts)
        if let Some(project_context) = &self.agent_project_context {
            if !project_context.trim().is_empty() {
                full.push_str("# Project Context\n\n");
                full.push_str(project_context.trim());
                full.push_str("\n\n---\n\n");
            }
        }

        // Add workflow history if present (shows prior work from previous columns)
        if let Some(workflow_history) = &self.agent_workflow_history {
            if !workflow_history.trim().is_empty() {
                full.push_str(workflow_history.trim());
                full.push_str("\n\n---\n\n");
            }
        }

        // Add the task prompt
        full.push_str("## Task\n\n");
        full.push_str(&self.prompt);

        // Append agent start command if present
        if let Some(start_command) = &self.agent_start_command {
            if !start_command.trim().is_empty() {
                full.push_str("\n\n---\n\n## Instructions\n\n");
                full.push_str(start_command.trim());
            }
        }

        // Add deliverable section - tells the agent what to produce and when to stop
        if let Some(deliverable) = &self.agent_deliverable {
            if !deliverable.trim().is_empty() {
                full.push_str("\n\n---\n\n## Expected Deliverable\n\n");
                full.push_str(deliverable.trim());
                full.push_str("\n\n**Important**: Once you have produced the deliverable described above, commit your work and stop. Do not proceed to implement the plan yourself - your job is complete when the deliverable is ready.");
            }
        }

        full
    }
}

#[async_trait]
impl Executable for CodingAgentInitialRequest {
    async fn spawn(
        &self,
        current_dir: &Path,
        approvals: Arc<dyn ExecutorApprovalService>,
        env: &ExecutionEnv,
    ) -> Result<SpawnedChild, ExecutorError> {
        // Use working_dir if specified, otherwise use current_dir
        let effective_dir = match &self.working_dir {
            Some(rel_path) => current_dir.join(rel_path),
            None => current_dir.to_path_buf(),
        };

        let executor_profile_id = self.executor_profile_id.clone();
        let mut agent = ExecutorConfigs::get_cached()
            .get_coding_agent(&executor_profile_id)
            .ok_or(ExecutorError::UnknownExecutorType(
                executor_profile_id.to_string(),
            ))?;

        agent.use_approvals(approvals.clone());

        // Build full prompt with agent context (system prompt + task + start command)
        let full_prompt = self.build_full_prompt();
        agent.spawn(&effective_dir, &full_prompt, env).await
    }
}

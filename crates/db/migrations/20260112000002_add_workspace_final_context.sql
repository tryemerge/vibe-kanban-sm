-- Add final_context field to workspaces to store task-level context before worktree deletion
-- This captures what the agent learned/decided during execution

ALTER TABLE workspaces ADD COLUMN final_context TEXT;

-- Also add a summary field for a brief description of what was accomplished
ALTER TABLE workspaces ADD COLUMN completion_summary TEXT;

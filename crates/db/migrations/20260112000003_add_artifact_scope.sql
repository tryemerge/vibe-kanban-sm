-- Add scope field to context_artifacts
-- Scope determines when an artifact is included in agent context:
--   'path' (default) - include when working on matching file paths
--   'task' - include only for the specific task (uses source_task_id)
--   'global' - always include for all agents in the project

ALTER TABLE context_artifacts ADD COLUMN scope TEXT NOT NULL DEFAULT 'path';

-- Create index for efficient global artifact queries
CREATE INDEX idx_context_artifacts_scope ON context_artifacts(project_id, scope);

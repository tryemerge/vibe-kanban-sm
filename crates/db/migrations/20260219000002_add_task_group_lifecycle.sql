-- Add lifecycle state machine to task groups
-- Replaces binary started_at approach with explicit states

-- Add state column (draft, analyzing, ready, executing, done, failed)
ALTER TABLE task_groups ADD COLUMN state TEXT NOT NULL DEFAULT 'draft';

-- Mark backlog groups for auto-promotion behavior
ALTER TABLE task_groups ADD COLUMN is_backlog BOOLEAN NOT NULL DEFAULT FALSE;

-- Store the analysis task's execution DAG (JSON)
-- Format: {"parallel_groups": [["task-id-1", "task-id-2"], ["task-id-3"]]}
ALTER TABLE task_groups ADD COLUMN execution_dag TEXT;

-- Index for finding groups by state (useful for auto-promotion queries)
CREATE INDEX idx_task_groups_state ON task_groups(project_id, state);

-- Note: Keep started_at for backward compat and as execution start timestamp
-- New immutability logic uses state != 'draft' instead of started_at IS NOT NULL

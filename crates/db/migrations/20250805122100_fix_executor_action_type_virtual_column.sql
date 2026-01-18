-- Drop the existing virtual column and index
DROP INDEX IF EXISTS idx_execution_processes_task_attempt_type_created;
ALTER TABLE execution_processes DROP COLUMN IF EXISTS executor_action_type;

-- Recreate the generated column with the correct JSON path (PostgreSQL version)
ALTER TABLE execution_processes
ADD COLUMN executor_action_type TEXT
  GENERATED ALWAYS AS ((executor_action::jsonb->'typ'->>'type')) STORED;

-- Recreate the index
CREATE INDEX idx_execution_processes_task_attempt_type_created
ON execution_processes (task_attempt_id, executor_action_type, created_at DESC);

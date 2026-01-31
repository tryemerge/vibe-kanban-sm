-- Add fired_at timestamp to track when each trigger has been satisfied
-- This enables "ALL" semantics: task only starts when ALL triggers have fired
ALTER TABLE task_triggers ADD COLUMN fired_at TIMESTAMPTZ;

-- Index for quickly finding unfired triggers
CREATE INDEX idx_task_triggers_unfired ON task_triggers(task_id) WHERE fired_at IS NULL;

-- Add starts_workflow flag to kanban_columns
-- This marks which column triggers attempt/workspace creation when a task enters it
-- Separate from is_initial (backlog) and agent_id (execution)

ALTER TABLE kanban_columns ADD COLUMN starts_workflow INTEGER NOT NULL DEFAULT 0;

-- Set starts_workflow=1 for any column with slug 'in_progress' (existing convention)
UPDATE kanban_columns SET starts_workflow = 1 WHERE slug = 'in_progress';

-- Add status field to kanban_columns to map columns to workflow states
-- This allows flexible column naming while preserving workflow functionality
-- (e.g., a "Unit Testing" column can map to "inprogress" status for agent triggers)

-- Add the status column with default 'todo'
ALTER TABLE kanban_columns ADD COLUMN status TEXT NOT NULL DEFAULT 'todo';

-- Update existing default columns to have correct status values
UPDATE kanban_columns SET status = 'todo' WHERE slug = 'backlog';
UPDATE kanban_columns SET status = 'inprogress' WHERE slug = 'in_progress';
UPDATE kanban_columns SET status = 'inreview' WHERE slug = 'in_review';
UPDATE kanban_columns SET status = 'done' WHERE slug = 'done';
UPDATE kanban_columns SET status = 'cancelled' WHERE slug = 'cancelled';

-- Create index for efficient status-based queries
CREATE INDEX idx_kanban_columns_status ON kanban_columns(status);

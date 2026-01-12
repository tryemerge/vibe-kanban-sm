-- Add deliverable field to kanban_columns
-- The deliverable describes what the agent should produce before moving to the next column
ALTER TABLE kanban_columns ADD COLUMN deliverable TEXT;

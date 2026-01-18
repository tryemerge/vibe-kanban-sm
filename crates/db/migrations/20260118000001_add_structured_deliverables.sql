-- Add structured deliverable options to kanban_columns
-- Allows defining a variable name and allowed values for agent decision output

ALTER TABLE kanban_columns ADD COLUMN deliverable_variable TEXT;
ALTER TABLE kanban_columns ADD COLUMN deliverable_options TEXT; -- JSON array of allowed values

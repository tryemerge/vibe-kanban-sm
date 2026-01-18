-- Remove project_id from kanban_columns
-- Columns belong to boards, not projects
-- PostgreSQL can directly drop columns

ALTER TABLE kanban_columns DROP COLUMN IF EXISTS project_id;

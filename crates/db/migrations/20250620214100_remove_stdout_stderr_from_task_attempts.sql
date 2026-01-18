-- Remove stdout and stderr columns from task_attempts table
-- These are now tracked in the execution_processes table for better granularity

-- PostgreSQL supports ALTER TABLE DROP COLUMN directly
ALTER TABLE task_attempts DROP COLUMN IF EXISTS stdout;
ALTER TABLE task_attempts DROP COLUMN IF EXISTS stderr;

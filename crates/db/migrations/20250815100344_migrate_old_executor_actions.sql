-- JSON format changed, means you can access logs from old execution_processes
-- This migration was for migrating SQLite data - not needed for fresh PostgreSQL setup
-- The original migration updated old executor_action JSON structure

-- This is a no-op for PostgreSQL as there's no legacy data to migrate
SELECT 1;

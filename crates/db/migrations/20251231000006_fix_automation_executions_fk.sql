-- Fix automation_executions: Originally fixed invalid foreign key to task_attempts
-- This is now a no-op since the previous migration was corrected to use workspaces

SELECT 1;

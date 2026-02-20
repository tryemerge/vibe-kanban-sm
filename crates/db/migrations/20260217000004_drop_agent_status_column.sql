-- Step 2: Migrate agent_status data into task_state, then drop the column
-- (Split from previous migration because PostgreSQL can't use new enum values
--  in the same transaction that adds them)

UPDATE tasks SET task_state = 'inprogress' WHERE agent_status = 'running';
UPDATE tasks SET task_state = 'awaitingresponse' WHERE agent_status = 'awaiting_response';

DROP INDEX IF EXISTS idx_tasks_agent_status;
ALTER TABLE tasks DROP COLUMN agent_status;
DROP TYPE IF EXISTS agent_status;

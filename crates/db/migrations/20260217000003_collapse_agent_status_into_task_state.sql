-- Step 1: Add 'awaitingresponse' to task_state enum
-- (The actual data migration and column drop happen in the next migration
--  because PostgreSQL can't use a new enum value in the same transaction)
ALTER TYPE task_state ADD VALUE IF NOT EXISTS 'awaitingresponse' AFTER 'inprogress';

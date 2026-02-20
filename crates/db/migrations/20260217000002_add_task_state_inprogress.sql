-- Add 'inprogress' value to the task_state enum type
-- This represents "agent is actively working on the task" within a column
ALTER TYPE task_state ADD VALUE IF NOT EXISTS 'inprogress' BEFORE 'transitioning';

-- Add task_state and agent_status to track the nested state machines
-- task_state: where the task is in the workflow process (queued vs transitioning)
-- agent_status: what the agent is doing with the task (nullable - null means no agent engaged)

-- Create the task_state enum type
CREATE TYPE task_state AS ENUM ('queued', 'transitioning');

-- Create the agent_status enum type
CREATE TYPE agent_status AS ENUM ('running', 'awaiting_response');

-- Add the new columns to tasks
ALTER TABLE tasks
ADD COLUMN task_state task_state NOT NULL DEFAULT 'queued',
ADD COLUMN agent_status agent_status DEFAULT NULL;

-- Add index for querying tasks by state
CREATE INDEX idx_tasks_task_state ON tasks(task_state);
CREATE INDEX idx_tasks_agent_status ON tasks(agent_status) WHERE agent_status IS NOT NULL;

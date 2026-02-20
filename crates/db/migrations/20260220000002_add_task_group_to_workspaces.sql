-- Add task_group_id to workspaces
-- Workspaces now belong to TaskGroups (not individual tasks)

ALTER TABLE workspaces
  ADD COLUMN task_group_id UUID REFERENCES task_groups(id);

-- Index for fast group lookups
CREATE INDEX idx_workspaces_task_group_id ON workspaces(task_group_id);

-- Unique constraint: one workspace per group
CREATE UNIQUE INDEX idx_workspaces_unique_task_group ON workspaces(task_group_id)
  WHERE task_group_id IS NOT NULL;

-- Comment documenting the change
COMMENT ON COLUMN workspaces.task_group_id IS
  'The TaskGroup that owns this workspace. One workspace per group, shared by all tasks.';

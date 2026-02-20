-- Add workspace_id to task_groups
-- TaskGroups now own the workspace/worktree (not individual tasks)
-- All tasks in a group share the same worktree

ALTER TABLE task_groups
  ADD COLUMN workspace_id UUID REFERENCES workspaces(id);

-- Index for fast workspace lookups
CREATE INDEX idx_task_groups_workspace_id ON task_groups(workspace_id);

-- Comment documenting the change
COMMENT ON COLUMN task_groups.workspace_id IS
  'The workspace/worktree for this group. All tasks in the group execute in this shared worktree.';

-- Agent file locks
-- Allows agents to claim exclusive access to files during execution

CREATE TABLE file_locks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_path TEXT NOT NULL,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    acquired_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    UNIQUE(file_path, project_id)
);

CREATE INDEX idx_file_locks_project_id ON file_locks(project_id);
CREATE INDEX idx_file_locks_task_id ON file_locks(task_id);
CREATE INDEX idx_file_locks_workspace_id ON file_locks(workspace_id);

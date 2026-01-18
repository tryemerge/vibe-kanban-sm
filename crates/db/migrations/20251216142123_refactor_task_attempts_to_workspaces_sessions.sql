-- Refactor task_attempts into workspaces and sessions (PostgreSQL version)
-- - Rename task_attempts -> workspaces (keeps workspace-related fields)
-- - Create sessions table (executor moves here)
-- - Update execution_processes.task_attempt_id -> session_id
-- - Rename executor_sessions -> coding_agent_turns (drop redundant task_attempt_id)
-- - Rename merges.task_attempt_id -> workspace_id
-- - Rename tasks.parent_task_attempt -> parent_workspace_id

-- 1. Rename task_attempts to workspaces
ALTER TABLE task_attempts RENAME TO workspaces;

-- 2. Create sessions table
CREATE TABLE sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id    UUID NOT NULL,
    executor        TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX idx_sessions_workspace_id ON sessions(workspace_id);

-- 3. Migrate data: create one session per workspace
INSERT INTO sessions (id, workspace_id, executor, created_at, updated_at)
SELECT gen_random_uuid(), id, executor, created_at, updated_at FROM workspaces;

-- 4. Drop executor column from workspaces
ALTER TABLE workspaces DROP COLUMN executor;

-- 5. Rename merges.task_attempt_id to workspace_id
DROP INDEX IF EXISTS idx_merges_task_attempt_id;
DROP INDEX IF EXISTS idx_merges_open_pr;
ALTER TABLE merges RENAME COLUMN task_attempt_id TO workspace_id;
CREATE INDEX idx_merges_workspace_id ON merges(workspace_id);
CREATE INDEX idx_merges_open_pr ON merges(workspace_id, pr_status)
WHERE merge_type = 'pr' AND pr_status = 'open';

-- 6. Rename tasks.parent_task_attempt to parent_workspace_id
DROP INDEX IF EXISTS idx_tasks_parent_task_attempt;
ALTER TABLE tasks RENAME COLUMN parent_task_attempt TO parent_workspace_id;
CREATE INDEX idx_tasks_parent_workspace_id ON tasks(parent_workspace_id);

-- 7. Add session_id to execution_processes (PostgreSQL can add columns with FK)
ALTER TABLE execution_processes ADD COLUMN session_id UUID;

-- Populate session_id by joining through sessions
UPDATE execution_processes ep
SET session_id = s.id
FROM sessions s
WHERE ep.task_attempt_id = s.workspace_id;

-- Make session_id NOT NULL and add FK
ALTER TABLE execution_processes ALTER COLUMN session_id SET NOT NULL;
ALTER TABLE execution_processes
    ADD CONSTRAINT fk_execution_processes_session
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE;

-- Drop old column and indexes
DROP INDEX IF EXISTS idx_execution_processes_task_attempt_created_at;
DROP INDEX IF EXISTS idx_execution_processes_task_attempt_type_created;
ALTER TABLE execution_processes DROP COLUMN task_attempt_id;

-- Create new indexes
CREATE INDEX idx_execution_processes_session_id ON execution_processes(session_id);
CREATE INDEX idx_execution_processes_session_status_run_reason
    ON execution_processes (session_id, status, run_reason);
CREATE INDEX idx_execution_processes_session_run_reason_created
    ON execution_processes (session_id, run_reason, created_at DESC);

-- 8. Rename executor_sessions to coding_agent_turns
ALTER TABLE executor_sessions RENAME TO coding_agent_turns;
ALTER TABLE coding_agent_turns RENAME COLUMN session_id TO agent_session_id;
ALTER TABLE coding_agent_turns DROP COLUMN IF EXISTS task_attempt_id;

-- Recreate indexes with new names
DROP INDEX IF EXISTS idx_executor_sessions_execution_process_id;
DROP INDEX IF EXISTS idx_executor_sessions_session_id;
CREATE INDEX idx_coding_agent_turns_execution_process_id ON coding_agent_turns(execution_process_id);
CREATE INDEX idx_coding_agent_turns_agent_session_id ON coding_agent_turns(agent_session_id);

-- 9. Rename attempt_repos to workspace_repos
ALTER TABLE attempt_repos RENAME TO workspace_repos;
ALTER TABLE workspace_repos RENAME COLUMN attempt_id TO workspace_id;
DROP INDEX IF EXISTS idx_attempt_repos_attempt_id;
DROP INDEX IF EXISTS idx_attempt_repos_repo_id;
CREATE INDEX idx_workspace_repos_workspace_id ON workspace_repos(workspace_id);
CREATE INDEX idx_workspace_repos_repo_id ON workspace_repos(repo_id);

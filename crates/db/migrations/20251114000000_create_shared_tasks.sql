

CREATE TABLE IF NOT EXISTS shared_tasks (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    remote_project_id   UUID NOT NULL,
    title               TEXT NOT NULL,
    description         TEXT,
    status              TEXT NOT NULL DEFAULT 'todo'
                        CHECK (status IN ('todo','inprogress','done','cancelled','inreview')),
    assignee_user_id    UUID,
    assignee_first_name TEXT,
    assignee_last_name  TEXT,
    assignee_username   TEXT,
    version             INTEGER NOT NULL DEFAULT 1,
    last_event_seq      INTEGER,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_shared_tasks_remote_project
    ON shared_tasks (remote_project_id);

CREATE INDEX IF NOT EXISTS idx_shared_tasks_status
    ON shared_tasks (status);

CREATE TABLE IF NOT EXISTS shared_activity_cursors (
    remote_project_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    last_seq          INTEGER NOT NULL CHECK (last_seq >= 0),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE tasks
    ADD COLUMN shared_task_id UUID REFERENCES shared_tasks(id) ON DELETE SET NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_shared_task_unique
    ON tasks(shared_task_id)
    WHERE shared_task_id IS NOT NULL;

ALTER TABLE projects
    ADD COLUMN remote_project_id UUID;

CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_remote_project_id
    ON projects(remote_project_id)
    WHERE remote_project_id IS NOT NULL;

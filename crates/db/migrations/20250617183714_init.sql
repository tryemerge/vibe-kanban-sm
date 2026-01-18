

CREATE TABLE projects (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name          TEXT NOT NULL,
    git_repo_path TEXT NOT NULL DEFAULT '' UNIQUE,
    setup_script  TEXT DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE tasks (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id  UUID NOT NULL,
    title       TEXT NOT NULL,
    description TEXT,
    status      TEXT NOT NULL DEFAULT 'todo'
                   CHECK (status IN ('todo','inprogress','done','cancelled','inreview')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE task_attempts (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id       UUID NOT NULL,
    worktree_path TEXT NOT NULL,
    merge_commit  TEXT,
    executor      TEXT,
    stdout        TEXT,
    stderr        TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE TABLE task_attempt_activities (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_attempt_id UUID NOT NULL,
    status          TEXT NOT NULL DEFAULT 'init'
                       CHECK (status IN ('init','setuprunning','setupcomplete','setupfailed','executorrunning','executorcomplete','executorfailed','paused')),    note            TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (task_attempt_id) REFERENCES task_attempts(id) ON DELETE CASCADE
);

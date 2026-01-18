-- PostgreSQL version of project repositories migration

-- Step 1: Create global repos registry
CREATE TABLE repos (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    path         TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    display_name TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Step 2: Create project_repos junction with per-repo script fields
CREATE TABLE project_repos (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id              UUID NOT NULL,
    repo_id                 UUID NOT NULL,
    setup_script            TEXT,
    cleanup_script          TEXT,
    copy_files              TEXT,
    parallel_setup_script   INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE CASCADE,
    UNIQUE (project_id, repo_id)
);
CREATE INDEX idx_project_repos_project_id ON project_repos(project_id);
CREATE INDEX idx_project_repos_repo_id ON project_repos(repo_id);

-- Step 3: Create attempt_repos
CREATE TABLE attempt_repos (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    attempt_id    UUID NOT NULL,
    repo_id       UUID NOT NULL,
    target_branch TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (attempt_id) REFERENCES task_attempts(id) ON DELETE CASCADE,
    FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE CASCADE,
    UNIQUE (attempt_id, repo_id)
);
CREATE INDEX idx_attempt_repos_attempt_id ON attempt_repos(attempt_id);
CREATE INDEX idx_attempt_repos_repo_id ON attempt_repos(repo_id);

-- Step 4: Execution process repo states
CREATE TABLE execution_process_repo_states (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    execution_process_id UUID NOT NULL,
    repo_id              UUID NOT NULL,
    before_head_commit   TEXT,
    after_head_commit    TEXT,
    merge_commit         TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (execution_process_id) REFERENCES execution_processes(id) ON DELETE CASCADE,
    FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE CASCADE,
    UNIQUE (execution_process_id, repo_id)
);
CREATE INDEX idx_eprs_process_id ON execution_process_repo_states(execution_process_id);
CREATE INDEX idx_eprs_repo_id ON execution_process_repo_states(repo_id);

-- Step 5: Add repo_id to merges table for multi-repo support
ALTER TABLE merges ADD COLUMN repo_id UUID REFERENCES repos(id);
CREATE INDEX idx_merges_repo_id ON merges(repo_id);

-- Step 6: Migrate existing projects to repos
INSERT INTO repos (id, path, name, display_name)
SELECT
    gen_random_uuid(),
    git_repo_path,
    '__NEEDS_BACKFILL__',
    '__NEEDS_BACKFILL__'
FROM projects
WHERE git_repo_path IS NOT NULL AND git_repo_path != '';

INSERT INTO project_repos (id, project_id, repo_id, setup_script, cleanup_script, copy_files, parallel_setup_script)
SELECT
    gen_random_uuid(),
    p.id,
    r.id,
    p.setup_script,
    p.cleanup_script,
    p.copy_files,
    p.parallel_setup_script
FROM projects p
JOIN repos r ON r.path = p.git_repo_path
WHERE p.git_repo_path IS NOT NULL AND p.git_repo_path != '';

-- Step 7: Migrate task_attempt.target_branch
INSERT INTO attempt_repos (id, attempt_id, repo_id, target_branch, created_at, updated_at)
SELECT
    gen_random_uuid(),
    ta.id,
    r.id,
    ta.target_branch,
    ta.created_at,
    ta.updated_at
FROM task_attempts ta
JOIN tasks t ON t.id = ta.task_id
JOIN project_repos pr ON pr.project_id = t.project_id
JOIN repos r ON r.id = pr.repo_id;

-- Step 8: Backfill merges.repo_id from attempt_repos
UPDATE merges
SET repo_id = (
    SELECT ar.repo_id
    FROM attempt_repos ar
    WHERE ar.attempt_id = merges.task_attempt_id
    LIMIT 1
);

-- Step 9: Make merges.repo_id NOT NULL (PostgreSQL can do this directly)
-- First set a default for any NULL values
UPDATE merges SET repo_id = '00000000-0000-0000-0000-000000000000' WHERE repo_id IS NULL;
ALTER TABLE merges ALTER COLUMN repo_id SET NOT NULL;

-- Step 10: Backfill per-repo state
INSERT INTO execution_process_repo_states (
    id, execution_process_id, repo_id, before_head_commit, after_head_commit
)
SELECT
    gen_random_uuid(),
    ep.id,
    r.id,
    ep.before_head_commit,
    ep.after_head_commit
FROM execution_processes ep
JOIN task_attempts ta ON ta.id = ep.task_attempt_id
JOIN tasks t ON t.id = ta.task_id
JOIN project_repos pr ON pr.project_id = t.project_id
JOIN repos r ON r.id = pr.repo_id;

-- Step 11: Cleanup old columns (PostgreSQL supports DROP COLUMN directly)
ALTER TABLE execution_processes DROP COLUMN IF EXISTS before_head_commit;
ALTER TABLE execution_processes DROP COLUMN IF EXISTS after_head_commit;
ALTER TABLE task_attempts DROP COLUMN IF EXISTS target_branch;

-- Step 12: Remove git_repo_path and legacy columns from projects
-- PostgreSQL can drop columns directly
ALTER TABLE projects DROP COLUMN IF EXISTS git_repo_path;
ALTER TABLE projects DROP COLUMN IF EXISTS setup_script;
ALTER TABLE projects DROP COLUMN IF EXISTS cleanup_script;
ALTER TABLE projects DROP COLUMN IF EXISTS copy_files;
ALTER TABLE projects DROP COLUMN IF EXISTS parallel_setup_script;

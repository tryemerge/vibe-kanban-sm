-- Task groups: project-scoped grouping of related tasks (ADR-012)
-- Tasks within a group execute sequentially via auto-created dependencies.
-- Groups become immutable once started (started_at set).

CREATE TABLE task_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    -- NULL = mutable (draft), set = frozen (executing)
    started_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, name)
);

CREATE INDEX idx_task_groups_project ON task_groups(project_id);

-- Inter-group dependency DAG
-- Defines which groups must complete before others can start.
CREATE TABLE task_group_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The blocked group
    task_group_id UUID NOT NULL REFERENCES task_groups(id) ON DELETE CASCADE,
    -- The prerequisite group that must complete first
    depends_on_group_id UUID NOT NULL REFERENCES task_groups(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- When prerequisite group completed (all tasks terminal)
    satisfied_at TIMESTAMPTZ,
    UNIQUE(task_group_id, depends_on_group_id)
);

CREATE INDEX idx_task_group_deps_group ON task_group_dependencies(task_group_id);
CREATE INDEX idx_task_group_deps_prereq ON task_group_dependencies(depends_on_group_id);

-- A task belongs to zero or one group
ALTER TABLE tasks ADD COLUMN task_group_id UUID REFERENCES task_groups(id) ON DELETE SET NULL;
CREATE INDEX idx_tasks_task_group ON tasks(task_group_id);

-- Distinguish auto-created group dependencies from manual ones
ALTER TABLE task_dependencies ADD COLUMN is_auto_group BOOLEAN NOT NULL DEFAULT FALSE;

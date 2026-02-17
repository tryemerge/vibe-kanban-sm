-- Task dependencies (hard blocking constraints)
-- Task B (task_id) CANNOT start until Task A (depends_on_task_id) is complete.
-- Dependencies use ALL semantics: blocked until every dependency is satisfied.
-- This is separate from triggers (auto-start). Dependencies block, triggers automate.

CREATE TABLE task_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The blocked task
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    -- The prerequisite task that must complete first
    depends_on_task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- When the dependency became satisfied (prerequisite reached a done terminal column)
    satisfied_at TIMESTAMPTZ,
    UNIQUE(task_id, depends_on_task_id)
);

CREATE INDEX idx_task_deps_task ON task_dependencies(task_id);
CREATE INDEX idx_task_deps_prereq ON task_dependencies(depends_on_task_id);
CREATE INDEX idx_task_deps_unsatisfied ON task_dependencies(task_id) WHERE satisfied_at IS NULL;

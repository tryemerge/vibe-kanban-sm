-- Enable hierarchical state transitions: board -> project -> task
--
-- Resolution order:
-- 1. Task-level (most specific) - task_id is set
-- 2. Project-level - project_id is set, task_id is NULL
-- 3. Board-level (default) - board_id is set, project_id and task_id are NULL

-- SQLite doesn't support ALTER COLUMN, so we need to recreate the table
-- to make project_id nullable

-- Step 1: Create new table with correct schema
CREATE TABLE state_transitions_new (
    id                    BLOB PRIMARY KEY,
    board_id              BLOB REFERENCES boards(id) ON DELETE CASCADE,
    project_id            BLOB REFERENCES projects(id) ON DELETE CASCADE,  -- Now nullable
    task_id               BLOB REFERENCES tasks(id) ON DELETE CASCADE,
    from_column_id        BLOB NOT NULL,
    to_column_id          BLOB NOT NULL,
    name                  TEXT,
    requires_confirmation INTEGER NOT NULL DEFAULT 0,
    condition_key         TEXT,
    condition_value       TEXT,
    max_iterations        INTEGER,
    created_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (from_column_id) REFERENCES kanban_columns(id) ON DELETE CASCADE,
    FOREIGN KEY (to_column_id) REFERENCES kanban_columns(id) ON DELETE CASCADE
);

-- Step 2: Copy existing data (project-level transitions remain project-level)
INSERT INTO state_transitions_new (
    id, project_id, from_column_id, to_column_id, name,
    requires_confirmation, condition_key, condition_value, max_iterations, created_at
)
SELECT
    id, project_id, from_column_id, to_column_id, name,
    requires_confirmation, condition_key, condition_value, max_iterations, created_at
FROM state_transitions;

-- Step 3: Drop old table and rename new one
DROP TABLE state_transitions;
ALTER TABLE state_transitions_new RENAME TO state_transitions;

-- Step 4: Recreate indexes
CREATE INDEX idx_state_transitions_from ON state_transitions(from_column_id);
CREATE INDEX idx_state_transitions_project ON state_transitions(project_id);
CREATE INDEX idx_state_transitions_board ON state_transitions(board_id);
CREATE INDEX idx_state_transitions_task ON state_transitions(task_id);

-- Step 5: Create unique constraints to prevent duplicate transitions at the same scope
-- (from_column, to_column, condition) must be unique within each scope level
CREATE UNIQUE INDEX idx_state_transitions_unique_board
    ON state_transitions(board_id, from_column_id, to_column_id, COALESCE(condition_key, ''), COALESCE(condition_value, ''))
    WHERE board_id IS NOT NULL AND project_id IS NULL AND task_id IS NULL;

CREATE UNIQUE INDEX idx_state_transitions_unique_project
    ON state_transitions(project_id, from_column_id, to_column_id, COALESCE(condition_key, ''), COALESCE(condition_value, ''))
    WHERE project_id IS NOT NULL AND task_id IS NULL;

CREATE UNIQUE INDEX idx_state_transitions_unique_task
    ON state_transitions(task_id, from_column_id, to_column_id, COALESCE(condition_key, ''), COALESCE(condition_value, ''))
    WHERE task_id IS NOT NULL;

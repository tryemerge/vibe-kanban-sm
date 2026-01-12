-- Add else_column_id and escalation_column_id to state_transitions
-- Rename max_iterations to max_failures for clarity
--
-- New semantics:
-- - to_column_id: where to go when condition matches (success path)
-- - else_column_id: where to go when condition doesn't match (normal failure)
-- - escalation_column_id: where to go when max_failures is reached (emergency)
-- - max_failures: count of times the else path was taken (not total visits)

-- SQLite doesn't support ALTER COLUMN or RENAME COLUMN in older versions,
-- so we recreate the table

-- Step 1: Create new table with updated schema
CREATE TABLE state_transitions_new (
    id                    BLOB PRIMARY KEY,
    board_id              BLOB REFERENCES boards(id) ON DELETE CASCADE,
    project_id            BLOB REFERENCES projects(id) ON DELETE CASCADE,
    task_id               BLOB REFERENCES tasks(id) ON DELETE CASCADE,
    from_column_id        BLOB NOT NULL,
    to_column_id          BLOB NOT NULL,
    else_column_id        BLOB REFERENCES kanban_columns(id) ON DELETE CASCADE,
    escalation_column_id  BLOB REFERENCES kanban_columns(id) ON DELETE CASCADE,
    name                  TEXT,
    requires_confirmation INTEGER NOT NULL DEFAULT 0,
    condition_key         TEXT,
    condition_value       TEXT,
    max_failures          INTEGER,  -- renamed from max_iterations
    created_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (from_column_id) REFERENCES kanban_columns(id) ON DELETE CASCADE,
    FOREIGN KEY (to_column_id) REFERENCES kanban_columns(id) ON DELETE CASCADE
);

-- Step 2: Copy existing data (max_iterations becomes max_failures, else/escalation are NULL)
INSERT INTO state_transitions_new (
    id, board_id, project_id, task_id, from_column_id, to_column_id,
    else_column_id, escalation_column_id, name, requires_confirmation,
    condition_key, condition_value, max_failures, created_at
)
SELECT
    id, board_id, project_id, task_id, from_column_id, to_column_id,
    NULL, NULL, name, requires_confirmation,
    condition_key, condition_value, max_iterations, created_at
FROM state_transitions;

-- Step 3: Drop old table and rename new one
DROP TABLE state_transitions;
ALTER TABLE state_transitions_new RENAME TO state_transitions;

-- Step 4: Recreate indexes
CREATE INDEX idx_state_transitions_from ON state_transitions(from_column_id);
CREATE INDEX idx_state_transitions_project ON state_transitions(project_id);
CREATE INDEX idx_state_transitions_board ON state_transitions(board_id);
CREATE INDEX idx_state_transitions_task ON state_transitions(task_id);

-- Step 5: Recreate unique constraints per scope level
CREATE UNIQUE INDEX idx_state_transitions_unique_board
    ON state_transitions(board_id, from_column_id, to_column_id, COALESCE(condition_key, ''), COALESCE(condition_value, ''))
    WHERE board_id IS NOT NULL AND project_id IS NULL AND task_id IS NULL;

CREATE UNIQUE INDEX idx_state_transitions_unique_project
    ON state_transitions(project_id, from_column_id, to_column_id, COALESCE(condition_key, ''), COALESCE(condition_value, ''))
    WHERE project_id IS NOT NULL AND task_id IS NULL;

CREATE UNIQUE INDEX idx_state_transitions_unique_task
    ON state_transitions(task_id, from_column_id, to_column_id, COALESCE(condition_key, ''), COALESCE(condition_value, ''))
    WHERE task_id IS NOT NULL;

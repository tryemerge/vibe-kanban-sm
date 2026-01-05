-- Fix automation_executions: Remove invalid foreign key to task_attempts
-- The task_attempts table doesn't exist in vanilla vibe-kanban

-- SQLite doesn't support DROP CONSTRAINT, so we need to recreate the table
-- Step 1: Create new table without the invalid FK
CREATE TABLE automation_executions_new (
    id              BLOB PRIMARY KEY,
    rule_id         BLOB NOT NULL,
    task_id         BLOB NOT NULL,
    attempt_id      BLOB,                     -- Optional: will store workspace session ID instead
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'running', 'completed', 'failed', 'skipped')),
    trigger_context TEXT,                     -- JSON: what triggered this (transition details)
    result          TEXT,                     -- JSON: output or error message
    started_at      TEXT,
    completed_at    TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
    -- Note: attempt_id is intentionally NOT a FK since we may link to different tables
);

-- Step 2: Copy data
INSERT INTO automation_executions_new SELECT * FROM automation_executions;

-- Step 3: Drop old table
DROP TABLE automation_executions;

-- Step 4: Rename new table
ALTER TABLE automation_executions_new RENAME TO automation_executions;

-- Step 5: Recreate indexes
CREATE INDEX idx_automation_executions_task ON automation_executions(task_id, created_at);
CREATE INDEX idx_automation_executions_rule ON automation_executions(rule_id, status);
CREATE INDEX idx_automation_executions_status ON automation_executions(status) WHERE status IN ('pending', 'running');

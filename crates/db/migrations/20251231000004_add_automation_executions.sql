-- Automation Executions: Log of all automation runs
-- Provides audit trail and debugging for automation system

CREATE TABLE automation_executions (
    id              BLOB PRIMARY KEY,
    rule_id         BLOB NOT NULL,
    task_id         BLOB NOT NULL,
    attempt_id      BLOB,                     -- Optional: linked task attempt if agent was run
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'running', 'completed', 'failed', 'skipped')),
    trigger_context TEXT,                     -- JSON: what triggered this (transition details)
    result          TEXT,                     -- JSON: output or error message
    started_at      TEXT,
    completed_at    TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (attempt_id) REFERENCES task_attempts(id) ON DELETE SET NULL
);

-- Indexes for efficient queries
CREATE INDEX idx_automation_executions_task ON automation_executions(task_id, created_at);
CREATE INDEX idx_automation_executions_rule ON automation_executions(rule_id, status);
CREATE INDEX idx_automation_executions_status ON automation_executions(status) WHERE status IN ('pending', 'running');

-- Automation Executions: Log of all automation runs
-- Provides audit trail and debugging for automation system

CREATE TABLE automation_executions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id         UUID NOT NULL,
    task_id         UUID NOT NULL,
    workspace_id    UUID,                     -- Optional: linked workspace if agent was run
    status          TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'running', 'completed', 'failed', 'skipped')),
    trigger_context TEXT,                     -- JSON: what triggered this (transition details)
    result          TEXT,                     -- JSON: output or error message
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

-- Indexes for efficient queries
CREATE INDEX idx_automation_executions_task ON automation_executions(task_id, created_at);
CREATE INDEX idx_automation_executions_rule ON automation_executions(rule_id, status);
CREATE INDEX idx_automation_executions_status ON automation_executions(status) WHERE status IN ('pending', 'running');

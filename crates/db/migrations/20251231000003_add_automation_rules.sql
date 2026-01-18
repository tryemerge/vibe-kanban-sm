-- Automation Rules: Triggers that run when tasks enter/exit columns
-- This is where the magic happens - automations fire on state changes

CREATE TABLE automation_rules (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id    UUID NOT NULL,
    column_id     UUID NOT NULL,
    trigger_type  TEXT NOT NULL CHECK (trigger_type IN ('on_enter', 'on_exit')),
    action_type   TEXT NOT NULL CHECK (action_type IN ('run_agent', 'create_pr', 'merge_pr', 'webhook', 'notify')),
    action_config TEXT NOT NULL,              -- JSON configuration for the action
    enabled       BOOLEAN NOT NULL DEFAULT TRUE, -- Is this rule active?
    priority      INTEGER NOT NULL DEFAULT 0, -- Execution order for multiple rules on same trigger
    name          TEXT,                       -- Human-readable rule name
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (column_id) REFERENCES kanban_columns(id) ON DELETE CASCADE
);

-- Index for efficient rule lookups on column changes
CREATE INDEX idx_automation_rules_column ON automation_rules(column_id, trigger_type, enabled);
CREATE INDEX idx_automation_rules_project ON automation_rules(project_id);

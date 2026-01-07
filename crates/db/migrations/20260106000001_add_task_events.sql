-- Task Events: Track workflow history for tasks
-- Events include column transitions, agent executions, commits, and manual actions

CREATE TABLE task_events (
    id              BLOB PRIMARY KEY,
    task_id         BLOB NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    event_type      TEXT NOT NULL,  -- 'column_enter', 'column_exit', 'agent_start', 'agent_complete', 'agent_failed', 'commit', 'manual_action'

    -- Column transition context (for column_enter/column_exit events)
    from_column_id  BLOB REFERENCES kanban_columns(id) ON DELETE SET NULL,
    to_column_id    BLOB REFERENCES kanban_columns(id) ON DELETE SET NULL,

    -- Agent/execution context (for agent_* events)
    workspace_id    BLOB REFERENCES workspaces(id) ON DELETE SET NULL,
    session_id      BLOB REFERENCES sessions(id) ON DELETE SET NULL,
    executor        TEXT,           -- Agent type (e.g., 'CLAUDE_CODE')

    -- Automation context
    automation_rule_id BLOB REFERENCES automation_rules(id) ON DELETE SET NULL,
    trigger_type    TEXT,           -- 'manual', 'automation', 'drag_drop'

    -- Commit context (for commit events)
    commit_hash     TEXT,
    commit_message  TEXT,

    -- Generic metadata (JSON for flexibility)
    metadata        TEXT,           -- JSON blob for additional context

    -- Actor info
    actor_type      TEXT NOT NULL DEFAULT 'system',  -- 'user', 'agent', 'system'
    actor_id        TEXT,           -- user_id or agent identifier

    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Indexes for efficient queries
CREATE INDEX idx_task_events_task_id ON task_events(task_id, created_at DESC);
CREATE INDEX idx_task_events_workspace ON task_events(workspace_id) WHERE workspace_id IS NOT NULL;
CREATE INDEX idx_task_events_type ON task_events(event_type, created_at DESC);

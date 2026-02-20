-- Orchestration event system for group-level decisions and state changes
-- Separate from task_events, focuses on group lifecycle and coordination

CREATE TABLE group_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_group_id UUID NOT NULL REFERENCES task_groups(id) ON DELETE CASCADE,
    -- Optional: specific task if event is task-related within a group context
    task_id UUID REFERENCES tasks(id) ON DELETE SET NULL,

    -- Event type (string for extensibility)
    -- Examples: 'group_state_change', 'group_analysis_start', 'group_execution_start',
    -- 'backlog_promoted', 'dag_task_added', 'group_task_started', etc.
    event_type TEXT NOT NULL,

    -- Who/what caused the event ('user', 'agent', 'system')
    actor_type TEXT NOT NULL DEFAULT 'system',

    -- Human-readable one-line summary: "What happened and why"
    summary TEXT NOT NULL,

    -- Structured JSON payload with event-specific data
    -- Example for group_state_change: {"from": "draft", "to": "analyzing", "reason": "dependencies_satisfied"}
    -- Example for group_analysis_complete: {"tasks_added": [...], "tasks_moved": [...], "dag": {...}}
    payload TEXT, -- JSON

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for querying events by group (newest first)
CREATE INDEX idx_group_events_group ON group_events(task_group_id, created_at DESC);

-- Index for filtering by event type
CREATE INDEX idx_group_events_type ON group_events(event_type);

-- Index for project-level orchestration feed (via group's project_id)
CREATE INDEX idx_group_events_project ON group_events(task_group_id);

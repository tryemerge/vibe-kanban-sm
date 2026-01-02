-- State Transitions: Define allowed movements between columns
-- This is the state machine that controls workflow

CREATE TABLE state_transitions (
    id                    BLOB PRIMARY KEY,
    project_id            BLOB NOT NULL,
    from_column_id        BLOB NOT NULL,
    to_column_id          BLOB NOT NULL,
    name                  TEXT,                       -- Optional: "Start Work", "Request Review"
    requires_confirmation INTEGER NOT NULL DEFAULT 0, -- Show confirmation dialog (boolean)
    created_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (from_column_id) REFERENCES kanban_columns(id) ON DELETE CASCADE,
    FOREIGN KEY (to_column_id) REFERENCES kanban_columns(id) ON DELETE CASCADE,
    UNIQUE(from_column_id, to_column_id)
);

-- Index for efficient transition lookups
CREATE INDEX idx_state_transitions_from ON state_transitions(from_column_id);
CREATE INDEX idx_state_transitions_project ON state_transitions(project_id);

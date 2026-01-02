-- Kanban Columns: Customizable task states per project
-- Replaces hardcoded status enum with flexible, per-project columns

CREATE TABLE kanban_columns (
    id          BLOB PRIMARY KEY,
    project_id  BLOB NOT NULL,
    name        TEXT NOT NULL,              -- "Todo", "In Progress", "Code Review", "Done"
    slug        TEXT NOT NULL,              -- "todo", "in_progress", "code_review", "done"
    position    INTEGER NOT NULL,           -- For ordering columns left-to-right
    color       TEXT,                       -- Optional: hex color for UI (e.g., "#3b82f6")
    is_initial  INTEGER NOT NULL DEFAULT 0, -- Tasks start here when created (boolean)
    is_terminal INTEGER NOT NULL DEFAULT 0, -- Tasks end here - done/cancelled states (boolean)
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE(project_id, slug)
);

-- Add column_id to tasks table (nullable initially for migration)
ALTER TABLE tasks ADD COLUMN column_id BLOB REFERENCES kanban_columns(id) ON DELETE SET NULL;

-- Index for efficient queries
CREATE INDEX idx_kanban_columns_project ON kanban_columns(project_id, position);
CREATE INDEX idx_tasks_column ON tasks(column_id);

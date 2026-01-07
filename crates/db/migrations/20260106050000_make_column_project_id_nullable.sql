-- Remove project_id from kanban_columns
-- Columns belong to boards, not projects

CREATE TABLE kanban_columns_new (
    id          BLOB PRIMARY KEY,
    board_id    BLOB NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL,
    position    INTEGER NOT NULL,
    color       TEXT,
    is_initial  INTEGER NOT NULL DEFAULT 0,
    is_terminal INTEGER NOT NULL DEFAULT 0,
    agent_id    BLOB REFERENCES agents(id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE(board_id, slug)
);

-- Copy data (drop project_id)
INSERT INTO kanban_columns_new
SELECT id, board_id, name, slug, position, color, is_initial, is_terminal, agent_id, created_at, updated_at
FROM kanban_columns;

-- Drop old table and rename
DROP TABLE kanban_columns;
ALTER TABLE kanban_columns_new RENAME TO kanban_columns;

-- Recreate indexes
CREATE INDEX idx_kanban_columns_board ON kanban_columns(board_id, position);
CREATE INDEX idx_kanban_columns_agent ON kanban_columns(agent_id);

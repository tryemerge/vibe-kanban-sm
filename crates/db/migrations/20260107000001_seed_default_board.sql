-- Seed a default board if none exist
-- This ensures new installations always have a usable board

-- Create default board if no boards exist
INSERT OR IGNORE INTO boards (id, name, description)
VALUES (
    X'00000000000000000000000000000001',
    'Default Board',
    'Standard Kanban workflow with Backlog, In Progress, Review, and Done columns'
);

-- Create default columns for the default board
-- Use INSERT OR IGNORE to avoid duplicates
INSERT OR IGNORE INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal)
VALUES (X'00000000000000000000000000000010', X'00000000000000000000000000000001', 'Backlog', 'backlog', 0, '#6b7280', 1, 0);

INSERT OR IGNORE INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal)
VALUES (X'00000000000000000000000000000011', X'00000000000000000000000000000001', 'In Progress', 'in_progress', 1, '#3b82f6', 0, 0);

INSERT OR IGNORE INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal)
VALUES (X'00000000000000000000000000000012', X'00000000000000000000000000000001', 'In Review', 'in_review', 2, '#8b5cf6', 0, 0);

INSERT OR IGNORE INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal)
VALUES (X'00000000000000000000000000000013', X'00000000000000000000000000000001', 'Done', 'done', 3, '#22c55e', 0, 1);

INSERT OR IGNORE INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal)
VALUES (X'00000000000000000000000000000014', X'00000000000000000000000000000001', 'Cancelled', 'cancelled', 4, '#ef4444', 0, 1);

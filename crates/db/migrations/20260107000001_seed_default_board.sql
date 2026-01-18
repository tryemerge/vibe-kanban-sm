-- Seed a default board if none exist
-- This ensures new installations always have a usable board

-- Create default board if no boards exist
INSERT INTO boards (id, name, description)
VALUES (
    '00000000-0000-0000-0000-000000000001'::uuid,
    'Default Board',
    'Standard Kanban workflow with Backlog, In Progress, Review, and Done columns'
)
ON CONFLICT DO NOTHING;

-- Create default columns for the default board
INSERT INTO kanban_columns (id, board_id, name, slug, position, color, is_initial, is_terminal)
VALUES
    ('00000000-0000-0000-0000-000000000010'::uuid, '00000000-0000-0000-0000-000000000001'::uuid, 'Backlog', 'backlog', 0, '#6b7280', TRUE, FALSE),
    ('00000000-0000-0000-0000-000000000011'::uuid, '00000000-0000-0000-0000-000000000001'::uuid, 'In Progress', 'in_progress', 1, '#3b82f6', FALSE, FALSE),
    ('00000000-0000-0000-0000-000000000012'::uuid, '00000000-0000-0000-0000-000000000001'::uuid, 'In Review', 'in_review', 2, '#8b5cf6', FALSE, FALSE),
    ('00000000-0000-0000-0000-000000000013'::uuid, '00000000-0000-0000-0000-000000000001'::uuid, 'Done', 'done', 3, '#22c55e', FALSE, TRUE),
    ('00000000-0000-0000-0000-000000000014'::uuid, '00000000-0000-0000-0000-000000000001'::uuid, 'Cancelled', 'cancelled', 4, '#ef4444', FALSE, TRUE)
ON CONFLICT DO NOTHING;

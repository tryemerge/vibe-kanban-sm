-- Seed default columns for existing projects
-- Creates standard Kanban columns and migrates existing task statuses

-- For each existing project, create default columns
-- We use a CTE to generate UUIDs via randomblob
INSERT INTO kanban_columns (id, project_id, name, slug, position, color, is_initial, is_terminal)
SELECT
    randomblob(16),
    p.id,
    'Backlog',
    'backlog',
    0,
    '#6b7280',  -- gray
    1,          -- is_initial
    0
FROM projects p
WHERE NOT EXISTS (SELECT 1 FROM kanban_columns kc WHERE kc.project_id = p.id);

INSERT INTO kanban_columns (id, project_id, name, slug, position, color, is_initial, is_terminal)
SELECT
    randomblob(16),
    p.id,
    'In Progress',
    'in_progress',
    1,
    '#3b82f6',  -- blue
    0,
    0
FROM projects p
WHERE EXISTS (SELECT 1 FROM kanban_columns kc WHERE kc.project_id = p.id AND kc.slug = 'backlog');

INSERT INTO kanban_columns (id, project_id, name, slug, position, color, is_initial, is_terminal)
SELECT
    randomblob(16),
    p.id,
    'In Review',
    'in_review',
    2,
    '#8b5cf6',  -- purple
    0,
    0
FROM projects p
WHERE EXISTS (SELECT 1 FROM kanban_columns kc WHERE kc.project_id = p.id AND kc.slug = 'backlog');

INSERT INTO kanban_columns (id, project_id, name, slug, position, color, is_initial, is_terminal)
SELECT
    randomblob(16),
    p.id,
    'Done',
    'done',
    3,
    '#22c55e',  -- green
    0,
    1           -- is_terminal
FROM projects p
WHERE EXISTS (SELECT 1 FROM kanban_columns kc WHERE kc.project_id = p.id AND kc.slug = 'backlog');

INSERT INTO kanban_columns (id, project_id, name, slug, position, color, is_initial, is_terminal)
SELECT
    randomblob(16),
    p.id,
    'Cancelled',
    'cancelled',
    4,
    '#ef4444',  -- red
    0,
    1           -- is_terminal
FROM projects p
WHERE EXISTS (SELECT 1 FROM kanban_columns kc WHERE kc.project_id = p.id AND kc.slug = 'backlog');

-- Migrate existing tasks to use column_id based on their status
-- Map: todo -> backlog, inprogress -> in_progress, inreview -> in_review, done -> done, cancelled -> cancelled
UPDATE tasks
SET column_id = (
    SELECT kc.id
    FROM kanban_columns kc
    WHERE kc.project_id = tasks.project_id
    AND kc.slug = CASE tasks.status
        WHEN 'todo' THEN 'backlog'
        WHEN 'inprogress' THEN 'in_progress'
        WHEN 'inreview' THEN 'in_review'
        WHEN 'done' THEN 'done'
        WHEN 'cancelled' THEN 'cancelled'
    END
)
WHERE tasks.column_id IS NULL;

-- Create default transitions (all moves allowed except from terminal states)
-- From Backlog
INSERT INTO state_transitions (id, project_id, from_column_id, to_column_id, name)
SELECT randomblob(16), p.id, f.id, t.id, 'Start Work'
FROM projects p
JOIN kanban_columns f ON f.project_id = p.id AND f.slug = 'backlog'
JOIN kanban_columns t ON t.project_id = p.id AND t.slug = 'in_progress';

-- From In Progress
INSERT INTO state_transitions (id, project_id, from_column_id, to_column_id, name)
SELECT randomblob(16), p.id, f.id, t.id, 'Request Review'
FROM projects p
JOIN kanban_columns f ON f.project_id = p.id AND f.slug = 'in_progress'
JOIN kanban_columns t ON t.project_id = p.id AND t.slug = 'in_review';

INSERT INTO state_transitions (id, project_id, from_column_id, to_column_id, name)
SELECT randomblob(16), p.id, f.id, t.id, 'Cancel'
FROM projects p
JOIN kanban_columns f ON f.project_id = p.id AND f.slug = 'in_progress'
JOIN kanban_columns t ON t.project_id = p.id AND t.slug = 'cancelled';

-- From In Review
INSERT INTO state_transitions (id, project_id, from_column_id, to_column_id, name)
SELECT randomblob(16), p.id, f.id, t.id, 'Needs Changes'
FROM projects p
JOIN kanban_columns f ON f.project_id = p.id AND f.slug = 'in_review'
JOIN kanban_columns t ON t.project_id = p.id AND t.slug = 'in_progress';

INSERT INTO state_transitions (id, project_id, from_column_id, to_column_id, name)
SELECT randomblob(16), p.id, f.id, t.id, 'Approve'
FROM projects p
JOIN kanban_columns f ON f.project_id = p.id AND f.slug = 'in_review'
JOIN kanban_columns t ON t.project_id = p.id AND t.slug = 'done';

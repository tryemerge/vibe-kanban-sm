-- Fix column slugs to match task status enum values
-- This allows proper fallback matching when tasks don't have explicit column_id

UPDATE kanban_columns SET slug = 'todo' WHERE slug = 'backlog';
UPDATE kanban_columns SET slug = 'inprogress' WHERE slug = 'in_progress';
UPDATE kanban_columns SET slug = 'inreview' WHERE slug = 'in_review';
-- 'done' and 'cancelled' already match

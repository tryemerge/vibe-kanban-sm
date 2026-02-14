-- Fix status enum values in workflow templates
-- The TaskStatus enum uses 'inprogress' (no underscore) and 'inreview' (no underscore)
-- But some template columns were seeded with 'in_progress' and 'in_review' (with underscore)

-- Fix all columns with incorrect status values
UPDATE kanban_columns SET status = 'inprogress' WHERE status = 'in_progress';
UPDATE kanban_columns SET status = 'inreview' WHERE status = 'in_review';

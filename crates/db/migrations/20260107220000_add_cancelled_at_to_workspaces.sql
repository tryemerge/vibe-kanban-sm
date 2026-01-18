-- Add cancelled_at field to workspaces to track cancelled attempts
-- When set, the attempt is cancelled but history is preserved
ALTER TABLE workspaces ADD COLUMN cancelled_at TIMESTAMPTZ;

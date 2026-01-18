-- Boards: Top-level kanban board configurations
-- Boards contain columns and can be shared across projects

-- Create boards table
CREATE TABLE boards (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,                -- "Default Kanban", "Agile Sprint", etc.
    description TEXT,                         -- Optional description
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add board_id to kanban_columns (nullable initially for migration)
ALTER TABLE kanban_columns ADD COLUMN board_id UUID REFERENCES boards(id) ON DELETE CASCADE;

-- Add board_id to projects (nullable initially, then we'll set defaults)
ALTER TABLE projects ADD COLUMN board_id UUID REFERENCES boards(id) ON DELETE SET NULL;

-- Create a default board for each existing project
-- First, we need to create boards and link columns to them

-- Create default boards for existing projects (one board per project)
INSERT INTO boards (id, name, description)
SELECT
    -- Generate a deterministic UUID from project_id for the board
    project_id,  -- Use project_id as board_id for simplicity in migration
    'Default Board',
    'Auto-migrated from project columns'
FROM (SELECT DISTINCT project_id FROM kanban_columns);

-- Update kanban_columns to reference their new board
UPDATE kanban_columns SET board_id = project_id;

-- Update projects to reference their board
UPDATE projects SET board_id = id WHERE id IN (SELECT DISTINCT project_id FROM kanban_columns);

-- Create boards and columns for projects that don't have columns yet
-- (They'll get a default board assigned when columns are created)

-- Create index for efficient queries
CREATE INDEX idx_kanban_columns_board ON kanban_columns(board_id, position);
CREATE INDEX idx_projects_board ON projects(board_id);

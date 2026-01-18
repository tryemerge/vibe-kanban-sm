

-- Add dev_script column to projects table
ALTER TABLE projects ADD COLUMN dev_script TEXT DEFAULT '';

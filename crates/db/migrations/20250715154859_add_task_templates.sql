-- Add task templates tables
CREATE TABLE task_templates (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id    UUID,  -- NULL for global templates
    title         TEXT NOT NULL,
    description   TEXT,
    template_name TEXT NOT NULL,  -- Display name for the template
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Add index for faster queries
CREATE INDEX idx_task_templates_project_id ON task_templates(project_id);

-- Add unique constraints to prevent duplicate template names within same scope
-- For project-specific templates: unique within each project
CREATE UNIQUE INDEX idx_task_templates_unique_name_project 
ON task_templates(project_id, template_name) 
WHERE project_id IS NOT NULL;

-- For global templates: unique across all global templates
CREATE UNIQUE INDEX idx_task_templates_unique_name_global 
ON task_templates(template_name) 
WHERE project_id IS NULL;
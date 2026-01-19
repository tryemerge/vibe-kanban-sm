-- Task Labels: Project-scoped labels for organizing tasks
-- Part of ADR 2026-01-18-004: Swim Lanes and Task Labels

-- Labels are defined per-project with a name and optional color
CREATE TABLE task_labels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT,  -- Hex color for visual distinction (e.g., '#3b82f6')
    position INTEGER NOT NULL DEFAULT 0,  -- For ordering in UI
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, name)
);

-- Many-to-many relationship: tasks can have multiple labels
CREATE TABLE task_label_assignments (
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES task_labels(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (task_id, label_id)
);

-- Indexes for efficient queries
CREATE INDEX idx_task_labels_project ON task_labels(project_id);
CREATE INDEX idx_task_label_assignments_task ON task_label_assignments(task_id);
CREATE INDEX idx_task_label_assignments_label ON task_label_assignments(label_id);

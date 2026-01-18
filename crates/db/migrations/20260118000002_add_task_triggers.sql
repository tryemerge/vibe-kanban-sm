-- Task auto-start triggers (soft dependencies)
-- When trigger_task_id completes, automatically start task_id

CREATE TABLE task_triggers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    trigger_task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    trigger_on TEXT NOT NULL DEFAULT 'completed',
    is_persistent BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(task_id, trigger_task_id)
);

CREATE INDEX idx_task_triggers_task_id ON task_triggers(task_id);
CREATE INDEX idx_task_triggers_trigger_task_id ON task_triggers(trigger_task_id);

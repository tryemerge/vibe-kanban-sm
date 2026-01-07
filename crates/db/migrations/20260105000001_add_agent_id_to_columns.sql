-- Add agent_id to kanban_columns for direct column-to-agent assignment
-- Each column can have one agent that handles tasks in that column

ALTER TABLE kanban_columns ADD COLUMN agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL;

-- Index for efficient agent lookups
CREATE INDEX idx_kanban_columns_agent ON kanban_columns(agent_id);

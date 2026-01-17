-- Add template fields to agents
ALTER TABLE agents ADD COLUMN is_template BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE agents ADD COLUMN template_group_id TEXT;

-- Add template fields to boards
ALTER TABLE boards ADD COLUMN is_template BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE boards ADD COLUMN template_group_id TEXT;
ALTER TABLE boards ADD COLUMN template_name TEXT;
ALTER TABLE boards ADD COLUMN template_description TEXT;
ALTER TABLE boards ADD COLUMN template_icon TEXT;

-- Add template fields to kanban_columns
ALTER TABLE kanban_columns ADD COLUMN is_template BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE kanban_columns ADD COLUMN template_group_id TEXT;

-- Add template fields to state_transitions
ALTER TABLE state_transitions ADD COLUMN is_template BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE state_transitions ADD COLUMN template_group_id TEXT;

-- Create indexes for efficient template queries
CREATE INDEX idx_agents_template ON agents(is_template, template_group_id);
CREATE INDEX idx_boards_template ON boards(is_template, template_group_id);
CREATE INDEX idx_columns_template ON kanban_columns(is_template, template_group_id);
CREATE INDEX idx_transitions_template ON state_transitions(is_template, template_group_id);

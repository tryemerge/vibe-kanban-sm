-- Add start_command field to agents for specifying the initial instruction
-- when auto-starting execution in a column
ALTER TABLE agents ADD COLUMN start_command TEXT;

-- Add deliverable field to agents table
-- This describes what the agent should produce before handing off to the next stage
ALTER TABLE agents ADD COLUMN deliverable TEXT;

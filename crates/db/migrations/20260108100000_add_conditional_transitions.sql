-- Add conditional transition support
-- condition_key: The JSON key to check in .vibe/decision.json (e.g., "decision")
-- condition_value: The value to match (e.g., "approve" or "reject")
-- max_iterations: Optional loop prevention - after N transitions from this column, use this path

ALTER TABLE state_transitions ADD COLUMN condition_key TEXT;
ALTER TABLE state_transitions ADD COLUMN condition_value TEXT;
ALTER TABLE state_transitions ADD COLUMN max_iterations INTEGER;

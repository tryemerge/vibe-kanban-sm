-- Fix Setup Board: Add structured deliverables to Research column
-- and conditional routing to transitions
--
-- The Research Bot agent outputs {"decision": "complete"} or {"decision": "ready"}
-- but the column had no deliverable_variable/deliverable_options defined,
-- and the transitions had no condition_key/condition_value for routing.

-- Add structured deliverable to Research column
UPDATE kanban_columns
SET deliverable_variable = 'decision',
    deliverable_options = '["complete", "ready"]'
WHERE id = '00000000-0000-0000-0002-000000000002';

-- Research → Complete: route when decision = "complete"
UPDATE state_transitions
SET condition_key = 'decision',
    condition_value = 'complete'
WHERE id = '00000000-0000-0000-0002-100000000003';

-- Research → Backlog: route when decision = "ready" (needs implementation work)
UPDATE state_transitions
SET condition_key = 'decision',
    condition_value = 'ready'
WHERE id = '00000000-0000-0000-0002-100000000004';

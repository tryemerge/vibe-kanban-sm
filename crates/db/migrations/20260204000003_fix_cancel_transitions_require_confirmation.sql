-- Fix Cancel transitions to require confirmation
-- Without this, the auto-transition logic could randomly pick Cancel
-- when the agent is waiting for user input (no decision.json exists).
-- Setting requires_confirmation = 1 (true) ensures Cancel transitions
-- only happen via explicit user action, not automatic workflow progression.

-- Update Cancel transitions in the Setup Board template to require confirmation
UPDATE state_transitions
SET requires_confirmation = 1
WHERE name = 'Cancel'
  AND is_template = TRUE
  AND template_group_id = '33333333-3333-3333-3333-333333333333';

-- Also update any non-template Cancel transitions that were created from the template
-- (for projects that already applied the template)
UPDATE state_transitions
SET requires_confirmation = 1
WHERE name = 'Cancel'
  AND is_template = FALSE;

-- Additionally, update any transition that goes to a terminal "cancelled" status column
-- These should all require explicit user confirmation
UPDATE state_transitions st
SET requires_confirmation = 1
FROM kanban_columns kc
WHERE st.to_column_id = kc.id
  AND kc.is_terminal = TRUE
  AND kc.status = 'cancelled'
  AND st.requires_confirmation = 0;

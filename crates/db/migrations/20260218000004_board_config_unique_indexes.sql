-- Board-level column configuration enforcement.
-- is_initial and starts_workflow should each be set on at most ONE column per board.
-- This migration cleans up any existing violations and adds partial unique indexes.

-- Fix duplicate is_initial: keep only the lowest-position column per board
UPDATE kanban_columns SET is_initial = FALSE
WHERE is_initial = TRUE AND is_template = FALSE
  AND id NOT IN (
    SELECT id FROM kanban_columns kc
    WHERE kc.is_initial = TRUE AND kc.is_template = FALSE
      AND kc.position = (
        SELECT MIN(kc2.position) FROM kanban_columns kc2
        WHERE kc2.board_id = kc.board_id AND kc2.is_initial = TRUE AND kc2.is_template = FALSE
      )
  );

-- Fix duplicate starts_workflow: keep only the lowest-position column per board
UPDATE kanban_columns SET starts_workflow = FALSE
WHERE starts_workflow = TRUE AND is_template = FALSE
  AND id NOT IN (
    SELECT id FROM kanban_columns kc
    WHERE kc.starts_workflow = TRUE AND kc.is_template = FALSE
      AND kc.position = (
        SELECT MIN(kc2.position) FROM kanban_columns kc2
        WHERE kc2.board_id = kc.board_id AND kc2.starts_workflow = TRUE AND kc2.is_template = FALSE
      )
  );

-- Enforce: at most one initial column per board
CREATE UNIQUE INDEX idx_one_initial_per_board
  ON kanban_columns (board_id) WHERE is_initial = TRUE AND is_template = FALSE;

-- Enforce: at most one starts_workflow column per board
CREATE UNIQUE INDEX idx_one_workflow_per_board
  ON kanban_columns (board_id) WHERE starts_workflow = TRUE AND is_template = FALSE;

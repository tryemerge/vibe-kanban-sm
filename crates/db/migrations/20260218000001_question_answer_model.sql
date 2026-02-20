-- Question/Answer model for column decisions + workflow decision history
--
-- Changes:
-- 1. Rename kanban_columns.deliverable_variable → question
-- 2. Rename kanban_columns.deliverable_options → answer_options
-- 3. Add tasks.workflow_decisions (JSONB) for accumulated decision history
-- 4. Drop state_transitions.condition_key (column's question is the key now)

-- 1. Rename column fields
ALTER TABLE kanban_columns RENAME COLUMN deliverable_variable TO question;
ALTER TABLE kanban_columns RENAME COLUMN deliverable_options TO answer_options;

-- 2. Add workflow_decisions to tasks
ALTER TABLE tasks ADD COLUMN workflow_decisions JSONB DEFAULT NULL;

-- 3. Drop condition_key from state_transitions (condition_value stays for answer routing)
ALTER TABLE state_transitions DROP COLUMN condition_key;

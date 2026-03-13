-- Persistent workspaces for column-level agents (lazy creation on first use).
-- One workspace per agent type per project — agents accumulate session history
-- rather than spinning up a new workspace per group.
--
-- grouper_workspace_id    → Task Grouper agent
-- group_evaluator_workspace_id → Group Evaluator agent (analyzing column)
-- prereq_eval_workspace_id     → PreReq Evaluator agent (prereq_eval column)
ALTER TABLE projects ADD COLUMN grouper_workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL;
ALTER TABLE projects ADD COLUMN group_evaluator_workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL;
ALTER TABLE projects ADD COLUMN prereq_eval_workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL;

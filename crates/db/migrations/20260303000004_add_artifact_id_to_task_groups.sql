-- Link task groups to their defining IMPL doc (iplan artifact).
-- Groups derive from design docs in the IMPL-doc-first model.
-- ON DELETE SET NULL: deleting an artifact does not cascade-delete the group.
ALTER TABLE task_groups
  ADD COLUMN artifact_id UUID REFERENCES context_artifacts(id) ON DELETE SET NULL;

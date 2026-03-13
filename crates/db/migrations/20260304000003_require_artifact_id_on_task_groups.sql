-- Enforce Plan-first workflow: non-backlog groups must have an iplan artifact.
-- Backlog groups are internal Task Grouper containers and are exempt.

-- Step 1: Backfill stub iplan artifacts for existing non-backlog groups that lack one.
-- Each stub becomes a real Plan card in the Plans panel (visible, editable by Project Agent).
WITH inserted AS (
    INSERT INTO context_artifacts (id, project_id, artifact_type, title, content, scope, created_at, updated_at)
    SELECT
        gen_random_uuid(),
        tg.project_id,
        'iplan',
        'Stub Plan for: ' || tg.name,
        E'## What\n' || tg.name || E'\n\n## Tasks\n(Auto-generated stub — update this plan via the Project Agent)',
        'global',
        NOW(),
        NOW()
    FROM task_groups tg
    WHERE tg.artifact_id IS NULL
      AND tg.is_backlog = FALSE
    RETURNING id, project_id, title
)
UPDATE task_groups tg
SET artifact_id = inserted.id
FROM inserted
WHERE tg.artifact_id IS NULL
  AND tg.is_backlog = FALSE
  AND inserted.title = 'Stub Plan for: ' || tg.name
  AND inserted.project_id = tg.project_id;

-- Step 2: Enforce the constraint going forward.
-- is_backlog groups (internal containers) are exempt.
ALTER TABLE task_groups
    ADD CONSTRAINT artifact_required_unless_backlog
    CHECK (is_backlog = TRUE OR artifact_id IS NOT NULL);

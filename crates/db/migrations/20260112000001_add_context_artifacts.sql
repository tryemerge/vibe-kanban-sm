-- Context artifacts for AI agent memory
-- Stores module memories, ADRs, decisions, patterns extracted from task work

CREATE TABLE context_artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Type of artifact: 'module_memory', 'adr', 'decision', 'pattern', 'dependency'
    artifact_type TEXT NOT NULL,

    -- File/module path this relates to (for module memories)
    path TEXT,

    -- Human-readable title
    title TEXT NOT NULL,

    -- The actual context content (markdown)
    content TEXT NOT NULL,

    -- Additional structured metadata (JSON)
    metadata TEXT,

    -- Provenance tracking
    source_task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    source_commit_hash TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Index for querying artifacts by project
CREATE INDEX idx_context_artifacts_project_id ON context_artifacts(project_id);

-- Index for querying artifacts by type within a project
CREATE INDEX idx_context_artifacts_project_type ON context_artifacts(project_id, artifact_type);

-- Index for querying module memories by path
CREATE INDEX idx_context_artifacts_path ON context_artifacts(path) WHERE path IS NOT NULL;

-- Index for finding artifacts created by a specific task
CREATE INDEX idx_context_artifacts_source_task ON context_artifacts(source_task_id) WHERE source_task_id IS NOT NULL;

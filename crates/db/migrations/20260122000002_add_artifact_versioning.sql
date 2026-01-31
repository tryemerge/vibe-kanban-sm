-- Add versioning and file path support for context artifacts
-- This enables hybrid storage: files as source of truth, DB for mutation history

-- File path on disk (relative to project root)
-- e.g., 'docs/adr/0001-use-postgres.md' or 'docs/plans/0001-feature-x.md'
ALTER TABLE context_artifacts ADD COLUMN file_path TEXT;

-- For version tracking: points to the artifact this one supersedes
-- NULL = this is the first version or a standalone artifact
ALTER TABLE context_artifacts ADD COLUMN supersedes_id UUID REFERENCES context_artifacts(id) ON DELETE SET NULL;

-- Chain ID groups all versions of the same logical document
-- All versions of 'ADR-0001' share the same chain_id
ALTER TABLE context_artifacts ADD COLUMN chain_id UUID;

-- Version number within a chain (1, 2, 3...)
ALTER TABLE context_artifacts ADD COLUMN version INTEGER NOT NULL DEFAULT 1;

-- Index for efficient chain queries (find all versions of a document)
CREATE INDEX idx_context_artifacts_chain_id ON context_artifacts(chain_id) WHERE chain_id IS NOT NULL;

-- Index for finding the latest version (highest version in chain)
CREATE INDEX idx_context_artifacts_chain_version ON context_artifacts(chain_id, version DESC) WHERE chain_id IS NOT NULL;

-- Index for file path lookups
CREATE INDEX idx_context_artifacts_file_path ON context_artifacts(file_path) WHERE file_path IS NOT NULL;

-- Rename artifact type: implementation_plan -> iplan
-- This is a data migration for existing records (if any)
UPDATE context_artifacts SET artifact_type = 'iplan' WHERE artifact_type = 'implementation_plan';

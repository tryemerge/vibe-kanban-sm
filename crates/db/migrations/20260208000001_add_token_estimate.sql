-- Add token_estimate column to context_artifacts
-- Stores approximate token count (content length / 4) for budget-aware context injection
-- See ADR-007: Context Budget & Relevance System

ALTER TABLE context_artifacts ADD COLUMN token_estimate INTEGER NOT NULL DEFAULT 0;

-- Backfill existing artifacts with estimated token counts
UPDATE context_artifacts SET token_estimate = LENGTH(content) / 4;

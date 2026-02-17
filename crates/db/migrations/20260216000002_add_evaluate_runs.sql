-- Persistent storage for test/evaluate run results
-- Each run captures a snapshot of what happened: task outcomes, artifacts produced,
-- context stats, and events. Tied to a git commit for tracking improvements over time.

CREATE TABLE evaluate_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    commit_hash TEXT,
    commit_message TEXT,
    project_name TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    summary JSONB NOT NULL,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

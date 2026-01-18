-- Follow-up drafts per task attempt
-- Stores a single draft prompt that can be queued for the next available run

CREATE TABLE IF NOT EXISTS follow_up_drafts (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_attempt_id  UUID NOT NULL UNIQUE,
    prompt           TEXT NOT NULL DEFAULT '',
    queued           INTEGER NOT NULL DEFAULT 0,
    sending          INTEGER NOT NULL DEFAULT 0,
    version          INTEGER NOT NULL DEFAULT 0,
    variant          TEXT NULL,
    image_ids        TEXT NULL, -- JSON array of UUID strings
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY(task_attempt_id) REFERENCES task_attempts(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_follow_up_drafts_task_attempt_id
    ON follow_up_drafts(task_attempt_id);

-- PostgreSQL trigger function for updated_at
CREATE OR REPLACE FUNCTION update_follow_up_drafts_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to keep updated_at current
DROP TRIGGER IF EXISTS trg_follow_up_drafts_updated_at ON follow_up_drafts;
CREATE TRIGGER trg_follow_up_drafts_updated_at
    BEFORE UPDATE ON follow_up_drafts
    FOR EACH ROW
    EXECUTE FUNCTION update_follow_up_drafts_updated_at();

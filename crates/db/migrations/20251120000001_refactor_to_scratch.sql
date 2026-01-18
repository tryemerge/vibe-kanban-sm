CREATE TABLE scratch (
    id           UUID NOT NULL,
    scratch_type TEXT NOT NULL,
    payload      TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, scratch_type)
);

CREATE INDEX idx_scratch_created_at ON scratch(created_at);

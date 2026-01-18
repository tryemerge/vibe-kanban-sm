

CREATE TABLE execution_process_logs (
    execution_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    logs              TEXT NOT NULL,      -- JSONL format (one LogMsg per line)
    byte_size         INTEGER NOT NULL,
    inserted_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (execution_id) REFERENCES execution_processes(id) ON DELETE CASCADE
);

CREATE INDEX idx_execution_process_logs_inserted_at ON execution_process_logs(inserted_at);

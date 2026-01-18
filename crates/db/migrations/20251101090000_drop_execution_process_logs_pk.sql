-- PostgreSQL version: Simply drop the primary key constraint
-- (PostgreSQL allows direct ALTER TABLE operations)

-- Drop the primary key if it exists (name may vary)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_type = 'PRIMARY KEY'
        AND table_name = 'execution_process_logs'
    ) THEN
        ALTER TABLE execution_process_logs DROP CONSTRAINT execution_process_logs_pkey;
    END IF;
END $$;

-- Ensure index exists for performance
CREATE INDEX IF NOT EXISTS idx_execution_process_logs_execution_id_inserted_at
    ON execution_process_logs (execution_id, inserted_at);

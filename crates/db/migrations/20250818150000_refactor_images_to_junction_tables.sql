

-- Refactor images table to use junction tables for many-to-many relationships
-- This allows images to be associated with multiple tasks and execution processes
-- No data migration needed as there are no existing users of the image system

CREATE TABLE images (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_path             TEXT NOT NULL,  -- relative path within cache/images/
    original_name         TEXT NOT NULL,
    mime_type             TEXT,
    size_bytes            INTEGER,
    hash                  TEXT NOT NULL UNIQUE,  -- SHA256 for deduplication
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create junction table for task-image associations
CREATE TABLE task_images (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id               UUID NOT NULL,
    image_id              UUID NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (image_id) REFERENCES images(id) ON DELETE CASCADE,
    UNIQUE(task_id, image_id)  -- Prevent duplicate associations
);


-- Create indexes for efficient querying
CREATE INDEX idx_images_hash ON images(hash);
CREATE INDEX idx_task_images_task_id ON task_images(task_id);
CREATE INDEX idx_task_images_image_id ON task_images(image_id);

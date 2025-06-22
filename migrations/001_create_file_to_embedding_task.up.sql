CREATE TABLE file_to_embedding_task (
    id SERIAL PRIMARY KEY,
    file_name VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    error_message TEXT,
    embedding_count INTEGER
);

-- Create indexes for efficient querying
CREATE INDEX idx_file_to_embedding_task_status ON file_to_embedding_task(status);
CREATE INDEX idx_file_to_embedding_task_created_at ON file_to_embedding_task(created_at);
CREATE INDEX idx_file_to_embedding_task_file_name ON file_to_embedding_task(file_name);

-- Add check constraint for valid status values
ALTER TABLE file_to_embedding_task 
ADD CONSTRAINT chk_file_to_embedding_task_status 
CHECK (status IN ('pending', 'processing', 'completed', 'failed'));
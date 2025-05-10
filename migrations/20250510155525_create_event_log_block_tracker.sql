-- Create a table to track event log processing
CREATE TABLE IF NOT EXISTS event_log_block_tracker (
    id BOOLEAN PRIMARY KEY DEFAULT TRUE,
    last_block BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT single_row CHECK (id)
);

-- Insert the single row that will be updated
INSERT INTO event_log_block_tracker (id) VALUES (TRUE);

-- Add table documentation
COMMENT ON TABLE event_log_block_tracker IS 'Tracks the last processed event log block number';
COMMENT ON COLUMN event_log_block_tracker.last_block IS 'Last block number that was successfully processed';

-- Down migration
DROP TABLE IF EXISTS event_log_block_tracker;
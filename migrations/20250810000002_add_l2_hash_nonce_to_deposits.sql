-- Add l2_hash and nonce fields to deposits table
ALTER TABLE deposits ADD COLUMN IF NOT EXISTS l2_hash TEXT;
ALTER TABLE deposits ADD COLUMN IF NOT EXISTS nonce BIGINT DEFAULT 0;

-- Create index on l2_hash for faster lookups
CREATE INDEX IF NOT EXISTS deposits_l2_hash_idx ON deposits (l2_hash);
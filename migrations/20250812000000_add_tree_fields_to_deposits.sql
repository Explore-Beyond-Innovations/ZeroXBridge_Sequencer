-- Add tree-related fields to deposits table for L1 tree builder
BEGIN;

-- Add leaf_index field for ordering deposits in the tree
ALTER TABLE deposits 
ADD COLUMN IF NOT EXISTS leaf_index BIGINT;

-- Add proof field to store Merkle proof data
ALTER TABLE deposits 
ADD COLUMN IF NOT EXISTS proof JSONB;

-- Add included flag to track deposits already in the tree
ALTER TABLE deposits 
ADD COLUMN IF NOT EXISTS included BOOLEAN DEFAULT FALSE;

-- Add merkle_root field to track the root at time of inclusion
ALTER TABLE deposits 
ADD COLUMN IF NOT EXISTS merkle_root TEXT;

-- Create index on status and included for efficient querying
CREATE INDEX IF NOT EXISTS deposits_status_included_idx ON deposits (status, included);

-- Create index on leaf_index for ordered processing
CREATE INDEX IF NOT EXISTS deposits_leaf_index_idx ON deposits (leaf_index);

-- Create partial unique index for leaf_index when included = true
CREATE UNIQUE INDEX IF NOT EXISTS uq_deposits_leaf_index_included ON deposits(leaf_index) WHERE included = true;

-- Add constraint to ensure rows with included = true have non-null leaf_index
ALTER TABLE deposits ADD CONSTRAINT IF NOT EXISTS chk_deposits_leaf_index_required_if_included 
CHECK (NOT included OR leaf_index IS NOT NULL);

-- Update existing deposits to have PENDING_TREE_INCLUSION status if they are pending
UPDATE deposits 
SET status = 'PENDING_TREE_INCLUSION' 
WHERE status = 'pending' AND included = FALSE;

COMMIT;
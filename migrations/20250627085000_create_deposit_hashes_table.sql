-- Create deposit_hashes table for DepositHashAppended events
CREATE TABLE IF NOT EXISTS deposit_hashes (
    id SERIAL PRIMARY KEY,
    index BIGINT NOT NULL,
    commitment_hash BYTEA NOT NULL,
    root_hash BYTEA NOT NULL,
    elements_count BIGINT NOT NULL,
    block_number BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on block_number for faster queries
CREATE INDEX IF NOT EXISTS deposit_hashes_block_number_idx ON deposit_hashes (block_number);

-- Add comments for documentation
COMMENT ON TABLE deposit_hashes IS 'Stores DepositHashAppended events from the ZeroXBridge L1 MerkleManager contract';
COMMENT ON COLUMN deposit_hashes.index IS 'Merkle tree index of the appended commitment';
COMMENT ON COLUMN deposit_hashes.commitment_hash IS 'Commitment hash of the deposit';
COMMENT ON COLUMN deposit_hashes.root_hash IS 'Updated Merkle root hash';
COMMENT ON COLUMN deposit_hashes.elements_count IS 'Total number of elements in the Merkle tree';
COMMENT ON COLUMN deposit_hashes.block_number IS 'Block number where the event was emitted';
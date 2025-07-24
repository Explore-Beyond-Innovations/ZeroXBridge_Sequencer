-- Make commitment_hash unique in deposits table
CREATE UNIQUE INDEX IF NOT EXISTS idx_deposits_commitment_hash ON deposits (commitment_hash);

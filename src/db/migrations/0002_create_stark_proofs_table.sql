CREATE TABLE IF NOT EXISTS stark_proofs (
    id SERIAL PRIMARY KEY,
    merkle_tree_root TEXT NOT NULL,
    commitment_hash TEXT NOT NULL,
    proof TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_stark_proofs_commitment_hash ON stark_proofs(commitment_hash);

-- Create withdrawals handler table
CREATE TABLE IF NOT EXISTS withdrawals (
    id SERIAL PRIMARY KEY,
    stark_pub_key TEXT NOT NULL,
    amount BIGINT NOT NULL,
    l1_token TEXT NOT NULL,
    l2_tx_id INTEGER,
    commitment_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    retry_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create deposit_proofs table
CREATE TABLE IF NOT EXISTS withdrawal_proofs (
    id SERIAL PRIMARY KEY,
    withdrawal_id INTEGER REFERENCES withdrawals(id),
    proof_params BYTEA,
    proof_data BYTEA,
    status TEXT NOT NULL DEFAULT 'ready',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

ALTER TABLE withdrawals
    ADD COLUMN IF NOT EXISTS l1_hash TEXT,
    ADD COLUMN IF NOT EXISTS nonce BIGINT;

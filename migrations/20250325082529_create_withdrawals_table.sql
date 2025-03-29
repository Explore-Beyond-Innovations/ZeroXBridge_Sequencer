-- 1️⃣ First, create the ENUM type
CREATE TYPE withdrawal_status AS ENUM ('pending', 'processed', 'failed');

-- 2️⃣ Then, create the table
CREATE TABLE IF NOT EXISTS withdrawals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stark_pub_key TEXT NOT NULL,
    amount TEXT NOT NULL,
    commitment_hash TEXT NOT NULL,
    status withdrawal_status NOT NULL DEFAULT 'pending', -- Use ENUM type directly
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

-- 3️⃣ Add constraints & indexes
ALTER TABLE withdrawals
ADD CONSTRAINT unique_commitment_hash UNIQUE (commitment_hash);

CREATE INDEX idx_withdrawals_status ON withdrawals(status);
CREATE INDEX idx_withdrawals_stark_pub_key ON withdrawals(stark_pub_key);

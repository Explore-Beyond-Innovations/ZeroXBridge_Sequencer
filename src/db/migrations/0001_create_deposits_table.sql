CREATE TYPE deposit_status AS ENUM ('pending', 'processed');

CREATE TABLE IF NOT EXISTS deposits (
    id SERIAL PRIMARY KEY,
    user_address TEXT NOT NULL,
    amount BIGINT NOT NULL CHECK (amount > 0),
    commitment_hash TEXT NOT NULL UNIQUE,
    status deposit_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_deposits_status ON deposits(status);
CREATE INDEX idx_deposits_user_address ON deposits(user_address);
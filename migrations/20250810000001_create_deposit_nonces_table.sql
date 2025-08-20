-- Create deposit_nonces table for tracking nonces per user Starknet pubkey
CREATE TABLE IF NOT EXISTS deposit_nonces (
    id SERIAL PRIMARY KEY,
    stark_pubkey TEXT NOT NULL UNIQUE,
    current_nonce BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on stark_pubkey for faster lookups
CREATE INDEX IF NOT EXISTS deposit_nonces_stark_pubkey_idx ON deposit_nonces (stark_pubkey);
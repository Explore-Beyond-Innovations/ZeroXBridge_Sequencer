-- Create withdrawal_nonces table to track per-user withdrawal nonces
CREATE TABLE IF NOT EXISTS withdrawal_nonces (
    stark_pub_key TEXT PRIMARY KEY,
    nonce BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);


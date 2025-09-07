CREATE TABLE mpc_keyshares (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    mpc_node_id INTEGER NOT NULL, -- Which MPC node (1, 2, 3, etc.)
    private_key_share TEXT NOT NULL, -- Encrypted private key share
    public_key TEXT NOT NULL,
    threshold INTEGER NOT NULL DEFAULT 2, -- Threshold for signing
    total_shares INTEGER NOT NULL DEFAULT 3, -- Total number of shares
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id, mpc_node_id)
);

-- Index for faster user keyshare lookups
CREATE INDEX idx_mpc_keyshares_user_id ON mpc_keyshares(user_id);
CREATE INDEX idx_mpc_keyshares_node_id ON mpc_keyshares(mpc_node_id);

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    agg_pubkey TEXT, -- Aggregated public key from MPC
    balance DECIMAL(20, 8) DEFAULT 0.0, -- SOL balance with 8 decimal precision
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Index for faster email lookups
CREATE INDEX idx_users_email ON users(email);
-- Index for faster balance queries
CREATE INDEX idx_users_balance ON users(balance);

CREATE TYPE transaction_status AS ENUM ('pending', 'confirmed', 'failed');
CREATE TYPE transaction_type AS ENUM ('deposit', 'withdrawal', 'transfer');

CREATE TABLE transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    tx_signature VARCHAR(88) UNIQUE, -- Solana transaction signature
    transaction_type transaction_type NOT NULL,
    status transaction_status DEFAULT 'pending',
    amount DECIMAL(20, 8) NOT NULL,
    token_mint VARCHAR(44), -- NULL for SOL transactions
    from_address VARCHAR(44),
    to_address VARCHAR(44),
    fee DECIMAL(20, 8) DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Index for faster transaction lookups
CREATE INDEX idx_transactions_user_id ON transactions(user_id);
CREATE INDEX idx_transactions_signature ON transactions(tx_signature);
CREATE INDEX idx_transactions_status ON transactions(status);

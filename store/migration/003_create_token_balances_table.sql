CREATE TABLE token_balances (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_mint VARCHAR(44) NOT NULL, -- Solana token mint address (base58)
    token_symbol VARCHAR(10) NOT NULL, -- Token symbol (USDC, USDT, etc.)
    balance DECIMAL(20, 8) DEFAULT 0.0,
    decimals INTEGER NOT NULL DEFAULT 6, -- Token decimals
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id, token_mint)
);

CREATE INDEX idx_token_balances_user_id ON token_balances(user_id);
CREATE INDEX idx_token_balances_mint ON token_balances(token_mint);

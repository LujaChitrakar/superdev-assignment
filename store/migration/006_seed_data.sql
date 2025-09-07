INSERT INTO token_balances (user_id, token_mint, token_symbol, balance, decimals) 
SELECT 
    u.id,
    'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v' as token_mint, -- USDC
    'USDC' as token_symbol,
    0.0 as balance,
    6 as decimals
FROM users u
ON CONFLICT (user_id, token_mint) DO NOTHING;

-- Function to get user with aggregated data
CREATE OR REPLACE FUNCTION get_user_with_balances(p_user_id UUID)
RETURNS TABLE(
    id UUID,
    email VARCHAR,
    agg_pubkey TEXT,
    sol_balance DECIMAL,
    created_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        u.id,
        u.email,
        u.agg_pubkey,
        u.balance as sol_balance,
        u.created_at,
        u.updated_at
    FROM users u
    WHERE u.id = p_user_id;
END;
$$ LANGUAGE plpgsql;
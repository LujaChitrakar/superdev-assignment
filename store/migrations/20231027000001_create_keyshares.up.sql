-- store/migrations/20231027000001_create_keyshares.up.sql
CREATE TABLE keyshares (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    share TEXT NOT NULL, -- Or BYTEA if the share is binary
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

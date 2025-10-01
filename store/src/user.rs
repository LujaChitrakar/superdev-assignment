use crate::Store;
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub agg_pubkey: Option<String>, // Aggregated public key from MPC
    pub balance: Decimal,           // SOL balance
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MpcKeyshare {
    pub id: Uuid,
    pub user_id: Uuid,
    pub mpc_node_id: i32,
    pub private_key_share: String,
    pub public_key: String,
    pub threshold: i32,
    pub total_shares: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKeyshareRequest {
    pub user_id: Uuid,
    pub mpc_node_id: i32,
    pub private_key_share: String,
    pub public_key: String,
    pub threshold: Option<i32>,
    pub total_shares: Option<i32>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TokenBalance {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_mint: String, // Solana mint address
    pub token_symbol: String,
    pub balance: Decimal,
    pub decimals: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBalanceRequest {
    pub user_id: Uuid,
    pub amount: Decimal,
    pub token_mint: Option<String>, // None for SOL
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tx_signature: Option<String>,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub amount: Decimal,
    pub token_mint: Option<String>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub fee: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_type", rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Transfer,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_status", rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBalanceResponse {
    pub user_id: Uuid,
    pub sol_balance: Decimal,
    pub token_balances: Vec<TokenBalance>,
}

#[derive(Debug)]
pub enum UserError {
    UserExists,
    InvalidInput(String),
    DatabaseError(String),
}

#[derive(Debug)]
pub enum StoreError {
    UserExists,
    UserNotFound,
    KeyshareExists,
    KeyshareNotFound,
    InsufficientBalance,
    InvalidInput(String),
    // DatabaseError(#[from] sqlx::Error),
    EncryptionError(String),
    PasswordError(String),
    DatabaseError(sqlx::Error),
}

impl From<sqlx::Error> for StoreError {
    fn from(err: sqlx::Error) -> Self {
        StoreError::DatabaseError(err)
    }
}

// Helper structs for aggregated queries
#[derive(Debug, Serialize)]
pub struct UserSummary {
    pub user: User,
    pub keyshare_count: i64,
    pub total_token_types: i64,
}

#[derive(Debug, Serialize)]
pub struct BalanceSummary {
    pub total_users: i64,
    pub total_sol_locked: Decimal,
    pub total_transactions: i64,
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::UserExists => write!(f, "User already exists"),
            UserError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            UserError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for UserError {}

impl Store {
    //DONE TILL TOKEN balance store impl

    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, StoreError> {
        // Validate email format
        if !request.email.contains('@') || request.email.len() < 5 {
            return Err(StoreError::InvalidInput("Invalid email format".to_string()));
        }

        // Validate password length
        if request.password.len() < 8 {
            return Err(StoreError::InvalidInput(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        // Check if user already exists
        let existing_user = sqlx::query!("SELECT id FROM users WHERE email = $1", request.email)
            .fetch_optional(&self.pool)
            .await?;

        if existing_user.is_some() {
            return Err(StoreError::UserExists);
        }

        // Hash the password
        let password_hash = hash(&request.password, DEFAULT_COST)
            .map_err(|e| StoreError::PasswordError(e.to_string()))?;

        // Insert user into database
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (email, password_hash, balance, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $4)
            RETURNING id, email, agg_pubkey, balance, created_at, updated_at
            "#,
            request.email,
            password_hash,
            Decimal::ZERO,
            Utc::now()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user(&self, user_id: Uuid) -> Result<User, StoreError> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, email, agg_pubkey, balance, created_at, updated_at FROM users WHERE id = $1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::UserNotFound)?;

        Ok(user)
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<User, StoreError> {
        let user = sqlx::query_as!(
            User,
            "SELECT id, email, agg_pubkey, balance, created_at, updated_at FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::UserNotFound)?;

        Ok(user)
    }

    /// Authenticate user with email and password
    pub async fn authenticate_user(&self, email: &str, password: &str) -> Result<User, StoreError> {
        let user_with_password = sqlx::query_as!(
            UserWithPassword,
            "SELECT id, email, password_hash, agg_pubkey, balance, created_at, updated_at FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::UserNotFound)?;

        // Verify password
        let is_valid = verify(password, &user_with_password.password_hash)
            .map_err(|e| StoreError::PasswordError(e.to_string()))?;

        if !is_valid {
            return Err(StoreError::InvalidInput("Invalid password".to_string()));
        }

        Ok(User {
            id: user_with_password.id,
            email: user_with_password.email,
            agg_pubkey: user_with_password.agg_pubkey,
            balance: user_with_password.balance,
            created_at: user_with_password.created_at,
            updated_at: user_with_password.updated_at,
        })
    }

    /// Update user's aggregated public key (after MPC key generation)
    pub async fn update_user_agg_pubkey(
        &self,
        user_id: Uuid,
        agg_pubkey: &str,
    ) -> Result<(), StoreError> {
        sqlx::query!(
            "UPDATE users SET agg_pubkey = $1, updated_at = $2 WHERE id = $3",
            agg_pubkey,
            Utc::now(),
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get user balance (SOL only)
    pub async fn get_user_balance(&self, user_id: Uuid) -> Result<Decimal, StoreError> {
        let balance = sqlx::query_scalar!("SELECT balance FROM users WHERE id = $1", user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StoreError::UserNotFound)?;

        Ok(balance)
    }

    /// Update user SOL balance
    pub async fn update_user_balance(
        &self,
        user_id: Uuid,
        new_balance: Decimal,
    ) -> Result<(), StoreError> {
        let updated_rows = sqlx::query!(
            "UPDATE users SET balance = $1, updated_at = $2 WHERE id = $3",
            new_balance,
            Utc::now(),
            user_id
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        if updated_rows == 0 {
            return Err(StoreError::UserNotFound);
        }

        Ok(())
    }

    /// Add to user SOL balance (for deposits)
    pub async fn add_user_balance(
        &self,
        user_id: Uuid,
        amount: Decimal,
    ) -> Result<Decimal, StoreError> {
        if amount <= Decimal::ZERO {
            return Err(StoreError::InvalidInput(
                "Amount must be positive".to_string(),
            ));
        }

        let new_balance = sqlx::query_scalar!(
            "UPDATE users SET balance = balance + $1, updated_at = $2 WHERE id = $3 RETURNING balance",
            amount,
            Utc::now(),
            user_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::UserNotFound)?;

        Ok(new_balance)
    }

    /// Subtract from user SOL balance (for withdrawals)
    pub async fn subtract_user_balance(
        &self,
        user_id: Uuid,
        amount: Decimal,
    ) -> Result<Decimal, StoreError> {
        if amount <= Decimal::ZERO {
            return Err(StoreError::InvalidInput(
                "Amount must be positive".to_string(),
            ));
        }

        // Check current balance first
        let current_balance = self.get_user_balance(user_id).await?;
        if current_balance < amount {
            return Err(StoreError::InsufficientBalance);
        }

        let new_balance = sqlx::query_scalar!(
            "UPDATE users SET balance = balance - $1, updated_at = $2 WHERE id = $3 RETURNING balance",
            amount,
            Utc::now(),
            user_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::UserNotFound)?;

        Ok(new_balance)
    }

    /// Get user with summary information
    pub async fn get_user_summary(&self, user_id: Uuid) -> Result<UserSummary, StoreError> {
        let user = self.get_user(user_id).await?;

        let keyshare_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM mpc_keyshares WHERE user_id = $1",
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        let total_token_types = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM token_balances WHERE user_id = $1",
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(UserSummary {
            user,
            keyshare_count,
            total_token_types,
        })
    }

    /// Get complete user balance information (SOL + all tokens)
    pub async fn get_user_complete_balance(
        &self,
        user_id: Uuid,
    ) -> Result<UserBalanceResponse, StoreError> {
        let sol_balance = self.get_user_balance(user_id).await?;

        let token_balances = sqlx::query_as!(
            TokenBalance,
            "SELECT id, user_id, token_mint, token_symbol, balance, decimals, created_at, updated_at 
             FROM token_balances WHERE user_id = $1 ORDER BY token_symbol",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(UserBalanceResponse {
            user_id,
            sol_balance,
            token_balances,
        })
    }

    /// List all users (for admin purposes)
    pub async fn list_users(&self, limit: i64, offset: i64) -> Result<Vec<User>, StoreError> {
        let users = sqlx::query_as!(
            User,
            "SELECT id, email, agg_pubkey, balance, created_at, updated_at 
             FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    /// Get total number of users
    pub async fn count_users(&self) -> Result<i64, StoreError> {
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0);

        Ok(count)
    }

    // MPC

    pub async fn create_keyshare(
        &self,
        request: CreateKeyshareRequest,
    ) -> Result<MpcKeyshare, StoreError> {
        // Validate MPC node ID (assuming nodes 1-5)
        if request.mpc_node_id < 1 || request.mpc_node_id > 5 {
            return Err(StoreError::InvalidInput(
                "MPC node ID must be between 1 and 5".to_string(),
            ));
        }

        // Validate that user exists
        sqlx::query!("SELECT id FROM users WHERE id = $1", request.user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StoreError::UserNotFound)?;

        // Check if keyshare already exists for this user and node
        let existing_keyshare = sqlx::query!(
            "SELECT id FROM mpc_keyshares WHERE user_id = $1 AND mpc_node_id = $2",
            request.user_id,
            request.mpc_node_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if existing_keyshare.is_some() {
            return Err(StoreError::KeyshareExists);
        }

        // Insert keyshare
        let keyshare = sqlx::query_as!(
            MpcKeyshare,
            r#"
            INSERT INTO mpc_keyshares (user_id, mpc_node_id, private_key_share, public_key, threshold, total_shares, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
            RETURNING id, user_id, mpc_node_id, private_key_share, public_key, threshold, total_shares, created_at, updated_at
            "#,
            request.user_id,
            request.mpc_node_id,
            request.private_key_share,
            request.public_key,
            request.threshold.unwrap_or(2),
            request.total_shares.unwrap_or(3),
            Utc::now()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(keyshare)
    }

    /// Get a specific keyshare by user ID and MPC node ID
    pub async fn get_keyshare(
        &self,
        user_id: Uuid,
        mpc_node_id: i32,
    ) -> Result<MpcKeyshare, StoreError> {
        let keyshare = sqlx::query_as!(
            MpcKeyshare,
            "SELECT id, user_id, mpc_node_id, private_key_share, public_key, threshold, total_shares, created_at, updated_at
             FROM mpc_keyshares WHERE user_id = $1 AND mpc_node_id = $2",
            user_id,
            mpc_node_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::KeyshareNotFound)?;

        Ok(keyshare)
    }

    /// Get all keyshares for a specific user
    pub async fn get_user_keyshares(&self, user_id: Uuid) -> Result<Vec<MpcKeyshare>, StoreError> {
        let keyshares = sqlx::query_as!(
            MpcKeyshare,
            "SELECT id, user_id, mpc_node_id, private_key_share, public_key, threshold, total_shares, created_at, updated_at
             FROM mpc_keyshares WHERE user_id = $1 ORDER BY mpc_node_id",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(keyshares)
    }

    /// Get all keyshares for a specific MPC node (for node operators)
    pub async fn get_node_keyshares(
        &self,
        mpc_node_id: i32,
    ) -> Result<Vec<MpcKeyshare>, StoreError> {
        if mpc_node_id < 1 || mpc_node_id > 5 {
            return Err(StoreError::InvalidInput("Invalid MPC node ID".to_string()));
        }

        let keyshares = sqlx::query_as!(
            MpcKeyshare,
            "SELECT id, user_id, mpc_node_id, private_key_share, public_key, threshold, total_shares, created_at, updated_at
             FROM mpc_keyshares WHERE mpc_node_id = $1 ORDER BY created_at",
            mpc_node_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(keyshares)
    }

    /// Update keyshare private key (for key refresh operations)
    pub async fn update_keyshare(
        &self,
        user_id: Uuid,
        mpc_node_id: i32,
        new_private_key_share: &str,
    ) -> Result<(), StoreError> {
        let updated_rows = sqlx::query!(
            "UPDATE mpc_keyshares SET private_key_share = $1, updated_at = $2 
             WHERE user_id = $3 AND mpc_node_id = $4",
            new_private_key_share,
            Utc::now(),
            user_id,
            mpc_node_id
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        if updated_rows == 0 {
            return Err(StoreError::KeyshareNotFound);
        }

        Ok(())
    }

    /// Check if user has minimum required keyshares for operations
    pub async fn has_sufficient_keyshares(
        &self,
        user_id: Uuid,
        required_threshold: Option<i32>,
    ) -> Result<bool, StoreError> {
        let threshold = required_threshold.unwrap_or(2);

        let keyshare_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM mpc_keyshares WHERE user_id = $1",
            user_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        Ok(keyshare_count >= threshold as i64)
    }

    /// Get keyshare statistics for monitoring
    pub async fn get_keyshare_stats(&self) -> Result<(i64, i64, i64), StoreError> {
        // Total keyshares, unique users with keyshares, active nodes
        let total_keyshares = sqlx::query_scalar!("SELECT COUNT(*) FROM mpc_keyshares")
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0);

        let unique_users = sqlx::query_scalar!("SELECT COUNT(DISTINCT user_id) FROM mpc_keyshares")
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0);

        let active_nodes =
            sqlx::query_scalar!("SELECT COUNT(DISTINCT mpc_node_id) FROM mpc_keyshares")
                .fetch_one(&self.pool)
                .await?
                .unwrap_or(0);

        Ok((total_keyshares, unique_users, active_nodes))
    }

    /// Batch create keyshares for a user across multiple nodes (for initial setup)
    pub async fn create_user_keyshares_batch(
        &self,
        user_id: Uuid,
        keyshares: Vec<(i32, String, String)>,
    ) -> Result<Vec<MpcKeyshare>, StoreError> {
        // keyshares format: (mpc_node_id, private_key_share, public_key)

        // Validate that user exists
        sqlx::query!("SELECT id FROM users WHERE id = $1", user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StoreError::UserNotFound)?;

        let mut created_keyshares = Vec::new();

        // Use transaction for atomic batch creation
        let mut tx = self.pool.begin().await?;

        for (mpc_node_id, private_key_share, public_key) in keyshares {
            // Validate MPC node ID
            if mpc_node_id < 1 || mpc_node_id > 5 {
                return Err(StoreError::InvalidInput(format!(
                    "Invalid MPC node ID: {}",
                    mpc_node_id
                )));
            }

            let keyshare = sqlx::query_as!(
                MpcKeyshare,
                r#"
                INSERT INTO mpc_keyshares (user_id, mpc_node_id, private_key_share, public_key, threshold, total_shares, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
                RETURNING id, user_id, mpc_node_id, private_key_share, public_key, threshold, total_shares, created_at, updated_at
                "#,
                user_id,
                mpc_node_id,
                private_key_share,
                public_key,
                2, // Default threshold
                3, // Default total shares
                Utc::now()
            )
            .fetch_one(&mut *tx)
            .await?;

            created_keyshares.push(keyshare);
        }

        tx.commit().await?;
        Ok(created_keyshares)
    }

    // Token balance

    /// Get token balance for a specific user and token
    pub async fn get_token_balance(
        &self,
        user_id: Uuid,
        token_mint: &str,
    ) -> Result<Decimal, StoreError> {
        let balance = sqlx::query_scalar!(
            "SELECT balance FROM token_balances WHERE user_id = $1 AND token_mint = $2",
            user_id,
            token_mint
        )
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(Decimal::ZERO); // Return 0 if no balance record exists

        Ok(balance)
    }

    /// Get all token balances for a user
    pub async fn get_user_token_balances(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<TokenBalance>, StoreError> {
        let token_balances = sqlx::query_as!(
            TokenBalance,
            "SELECT id, user_id, token_mint, token_symbol, balance, decimals, created_at, updated_at
             FROM token_balances WHERE user_id = $1 ORDER BY token_symbol",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(token_balances)
    }

    /// Create or update token balance for a user
    pub async fn update_token_balance(
        &self,
        user_id: Uuid,
        token_mint: &str,
        token_symbol: &str,
        balance: Decimal,
        decimals: i32,
    ) -> Result<TokenBalance, StoreError> {
        // Validate that user exists
        sqlx::query!("SELECT id FROM users WHERE id = $1", user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StoreError::UserNotFound)?;

        let token_balance = sqlx::query_as!(
            TokenBalance,
            r#"
            INSERT INTO token_balances (user_id, token_mint, token_symbol, balance, decimals, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            ON CONFLICT (user_id, token_mint) 
            DO UPDATE SET 
                balance = EXCLUDED.balance,
                token_symbol = EXCLUDED.token_symbol,
                decimals = EXCLUDED.decimals,
                updated_at = EXCLUDED.updated_at
            RETURNING id, user_id, token_mint, token_symbol, balance, decimals, created_at, updated_at
            "#,
            user_id,
            token_mint,
            token_symbol,
            balance,
            decimals,
            Utc::now()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(token_balance)
    }

    /// Add to token balance (for deposits)
    pub async fn add_token_balance(
        &self,
        user_id: Uuid,
        token_mint: &str,
        amount: Decimal,
    ) -> Result<Decimal, StoreError> {
        if amount <= Decimal::ZERO {
            return Err(StoreError::InvalidInput(
                "Amount must be positive".to_string(),
            ));
        }

        // Check if token balance record exists
        let existing_balance = sqlx::query!(
            "SELECT balance FROM token_balances WHERE user_id = $1 AND token_mint = $2",
            user_id,
            token_mint
        )
        .fetch_optional(&self.pool)
        .await?;

        if existing_balance.is_none() {
            return Err(StoreError::InvalidInput(
                "Token balance record not found. Create it first with update_token_balance"
                    .to_string(),
            ));
        }

        let new_balance = sqlx::query_scalar!(
            "UPDATE token_balances SET balance = balance + $1, updated_at = $2 
             WHERE user_id = $3 AND token_mint = $4 
             RETURNING balance",
            amount,
            Utc::now(),
            user_id,
            token_mint
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(new_balance)
    }

    /// Subtract from token balance (for withdrawals)
    pub async fn subtract_token_balance(
        &self,
        user_id: Uuid,
        token_mint: &str,
        amount: Decimal,
    ) -> Result<Decimal, StoreError> {
        if amount <= Decimal::ZERO {
            return Err(StoreError::InvalidInput(
                "Amount must be positive".to_string(),
            ));
        }

        // Check current balance first
        let current_balance = self.get_token_balance(user_id, token_mint).await?;
        if current_balance < amount {
            return Err(StoreError::InsufficientBalance);
        }

        let new_balance = sqlx::query_scalar!(
            "UPDATE token_balances SET balance = balance - $1, updated_at = $2 
             WHERE user_id = $3 AND token_mint = $4 
             RETURNING balance",
            amount,
            Utc::now(),
            user_id,
            token_mint
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::InvalidInput(
            "Token balance record not found".to_string(),
        ))?;

        Ok(new_balance)
    }

    /// Get token balance with full token information
    pub async fn get_token_balance_info(
        &self,
        user_id: Uuid,
        token_mint: &str,
    ) -> Result<TokenBalance, StoreError> {
        let token_balance = sqlx::query_as!(
            TokenBalance,
            "SELECT id, user_id, token_mint, token_symbol, balance, decimals, created_at, updated_at
             FROM token_balances WHERE user_id = $1 AND token_mint = $2",
            user_id,
            token_mint
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::InvalidInput(
            "Token balance not found".to_string(),
        ))?;

        Ok(token_balance)
    }

    /// Transfer tokens between users (internal transfer)
    pub async fn transfer_tokens(
        &self,
        from_user_id: Uuid,
        to_user_id: Uuid,
        token_mint: &str,
        amount: Decimal,
    ) -> Result<(Decimal, Decimal), StoreError> {
        if amount <= Decimal::ZERO {
            return Err(StoreError::InvalidInput(
                "Transfer amount must be positive".to_string(),
            ));
        }

        // Use transaction for atomic transfer
        let mut tx = self.pool.begin().await?;

        // Check sender balance
        let sender_balance: Decimal = sqlx::query_scalar!(
            "SELECT balance FROM token_balances WHERE user_id = $1 AND token_mint = $2",
            from_user_id,
            token_mint
        )
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(Decimal::ZERO);

        if sender_balance < amount {
            return Err(StoreError::InsufficientBalance);
        }

        // Subtract from sender
        let new_sender_balance = sqlx::query_scalar!(
            "UPDATE token_balances SET balance = balance - $1, updated_at = $2 
             WHERE user_id = $3 AND token_mint = $4 
             RETURNING balance",
            amount,
            Utc::now(),
            from_user_id,
            token_mint
        )
        .fetch_one(&mut *tx)
        .await?;

        // Add to receiver (create record if doesn't exist)
        let new_receiver_balance = sqlx::query_scalar!(
            r#"
            INSERT INTO token_balances (user_id, token_mint, token_symbol, balance, decimals, created_at, updated_at)
            VALUES ($1, $2, 'UNKNOWN', $3, 6, $4, $4)
            ON CONFLICT (user_id, token_mint) 
            DO UPDATE SET 
                balance = token_balances.balance + EXCLUDED.balance,
                updated_at = EXCLUDED.updated_at
            RETURNING balance
            "#,
            to_user_id,
            token_mint,
            amount,
            Utc::now()
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((new_sender_balance, new_receiver_balance))
    }

    /// Delete zero balance token records (cleanup)
    pub async fn cleanup_zero_balances(&self, user_id: Option<Uuid>) -> Result<u64, StoreError> {
        let deleted_count = if let Some(user_id) = user_id {
            sqlx::query!(
                "DELETE FROM token_balances WHERE user_id = $1 AND balance = 0",
                user_id
            )
            .execute(&self.pool)
            .await?
            .rows_affected()
        } else {
            sqlx::query!("DELETE FROM token_balances WHERE balance = 0")
                .execute(&self.pool)
                .await?
                .rows_affected()
        };

        Ok(deleted_count)
    }
}

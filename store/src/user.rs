use crate::Store;
use serde::Serialize;
use sqlx::prelude::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub agg_pubkey: Option<String>, // Aggregated public key from MPC
    pub balance: Decimal, // SOL balance
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
    // pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, UserError> {
    //     // Validate email format
    //     if !request.email.contains('@') {
    //         return Err(UserError::InvalidInput("Invalid email format".to_string()));
    //     }

    //     // Validate password length
    //     if request.password.len() < 6 {
    //         return Err(UserError::InvalidInput("Password must be at least 6 characters".to_string()));
    //     }

    //     // Check if user already exists
    //     let existing_user = sqlx::query!(
    //         "SELECT id FROM users WHERE email = $1",
    //         request.email
    //     )
    //     .fetch_optional(&self.pool)
    //     .await
    //     .map_err(|e| UserError::DatabaseError(e.to_string()))?;

    //     if existing_user.is_some() {
    //         return Err(UserError::UserExists);
    //     }

    //     // Hash the password
    //     let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)
    //         .map_err(|e| UserError::DatabaseError(format!("Password hashing failed: {}", e)))?;

    //     // Generate user ID and timestamp
    //     let user_id = Uuid::new_v4().to_string();
    //     let created_at = Utc::now();

    //     // Insert user into database
    //     sqlx::query!(
    //         "INSERT INTO users (id, email, password_hash, created_at) VALUES ($1, $2, $3, $4)",
    //         user_id,
    //         request.email,
    //         password_hash,
    //         created_at
    //     )
    //     .execute(&self.pool)
    //     .await
    //     .map_err(|e| UserError::DatabaseError(e.to_string()))?;

    //     // Return the created user
    //     let user = User {
    //         id: user_id,
    //         email: request.email,
    //         created_at: created_at.to_rfc3339(),
    //     };

    //     Ok(user)
    // }


}

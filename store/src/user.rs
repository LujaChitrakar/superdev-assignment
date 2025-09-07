use crate::Store;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    #[sqlx(rename = "password_hash")]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Keyshare {
    pub id: Uuid,
    pub user_id: Uuid,
    pub share: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Balance {
    pub id: Uuid,
    pub user_id: Uuid,
    pub mint: String,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug)]
pub struct CreateKeyshareRequest {
    pub user_id: Uuid,
    pub share: String,
}

#[derive(Debug)]
pub enum StoreError {
    UserExists,
    UserNotFound,
    KeyshareNotFound,
    BalanceNotFound,
    InvalidInput(String),
    DatabaseError(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::UserExists => write!(f, "User already exists"),
            StoreError::UserNotFound => write!(f, "User not found"),
            StoreError::KeyshareNotFound => write!(f, "Keyshare not found"),
            StoreError::BalanceNotFound => write!(f, "Balance not found"),
            StoreError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            StoreError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for StoreError {}

impl Store {
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, StoreError> {
        if request.username.is_empty() {
            return Err(StoreError::InvalidInput("Username cannot be empty".to_string()));
        }

        if request.password.len() < 8 {
            return Err(StoreError::InvalidInput("Password must be at least 8 characters".to_string()));
        }

        let existing_user = sqlx::query("SELECT id FROM users WHERE username = $1")
            .bind(&request.username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StoreError::DatabaseError(e.to_string()))?;

        if existing_user.is_some() {
            return Err(StoreError::UserExists);
        }

        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)
            .map_err(|e| StoreError::DatabaseError(format!("Password hashing failed: {}", e)))?;

        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING *",
        )
        .bind(&request.username)
        .bind(&password_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StoreError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    pub async fn get_user(&self, user_id: Uuid) -> Result<User, StoreError> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StoreError::DatabaseError(e.to_string()))?
            .ok_or(StoreError::UserNotFound)?;

        Ok(user)
    }

    pub async fn create_keyshare(&self, request: CreateKeyshareRequest) -> Result<Keyshare, StoreError> {
        let keyshare = sqlx::query_as::<_, Keyshare>(
            "INSERT INTO keyshares (user_id, share) VALUES ($1, $2) RETURNING *",
        )
        .bind(request.user_id)
        .bind(request.share)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StoreError::DatabaseError(e.to_string()))?;

        Ok(keyshare)
    }

    pub async fn get_keyshare(&self, user_id: Uuid) -> Result<Keyshare, StoreError> {
        let keyshare = sqlx::query_as::<_, Keyshare>("SELECT * FROM keyshares WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StoreError::DatabaseError(e.to_string()))?
            .ok_or(StoreError::KeyshareNotFound)?;

        Ok(keyshare)
    }

    pub async fn get_user_balance(&self, user_id: Uuid) -> Result<Vec<Balance>, StoreError> {
        let balances = sqlx::query_as::<_, Balance>("SELECT * FROM balances WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StoreError::DatabaseError(e.to_string()))?;

        Ok(balances)
    }

    pub async fn get_token_balance(&self, user_id: Uuid, mint: String) -> Result<Balance, StoreError> {
        let balance = sqlx::query_as::<_, Balance>("SELECT * FROM balances WHERE user_id = $1 AND mint = $2")
            .bind(user_id)
            .bind(mint)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StoreError::DatabaseError(e.to_string()))?
            .ok_or(StoreError::BalanceNotFound)?;

        Ok(balance)
    }
}
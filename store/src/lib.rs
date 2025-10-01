pub mod transaction;
pub mod user;
use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub struct Store {
    pub pool: PgPool,
}

impl Store {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("./migrations").run(&self.pool).await
    }

    /// Close the database connection pool
    pub async fn close(&self) {
        self.pool.close().await;
    }

    // Check if the database connection is healthy
    pub async fn health_check(&self) -> Result<bool, sqlx::Error> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map(|_| true)
            .or(Ok(false))
    }
}

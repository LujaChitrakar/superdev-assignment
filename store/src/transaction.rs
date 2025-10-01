use crate::Store;
use crate::user::{StoreError, Transaction, TransactionStatus, TransactionType};
use chrono::Utc;
use rust_decimal::Decimal;
use uuid::Uuid;

impl Store {
    /// Create a new transaction record
    pub async fn create_transaction(
        &self,
        user_id: Uuid,
        transaction_type: TransactionType,
        amount: Decimal,
        token_mint: Option<String>,
        from_address: Option<String>,
        to_address: Option<String>,
        fee: Option<Decimal>,
    ) -> Result<Transaction, StoreError> {
        // Validate that user exists
        sqlx::query!("SELECT id FROM users WHERE id = $1", user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StoreError::UserNotFound)?;

        if amount <= Decimal::ZERO {
            return Err(StoreError::InvalidInput(
                "Amount must be positive".to_string(),
            ));
        }

        let transaction = sqlx::query_as!(
            Transaction,
            r#"
            INSERT INTO transactions (user_id, transaction_type, status, amount, token_mint, from_address, to_address, fee, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
            RETURNING id, user_id, tx_signature, transaction_type as "transaction_type: TransactionType", 
                      status as "status: TransactionStatus", amount, token_mint, from_address, to_address, fee, created_at, updated_at
            "#,
            user_id,
            transaction_type as TransactionType,
            TransactionStatus::Pending as TransactionStatus,
            amount,
            token_mint,
            from_address,
            to_address,
            fee.unwrap_or(Decimal::ZERO),
            Utc::now()
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(transaction)
    }

    /// Update transaction status and signature
    pub async fn update_transaction_status(
        &self,
        transaction_id: Uuid,
        status: TransactionStatus,
        tx_signature: Option<String>,
    ) -> Result<(), StoreError> {
        let updated_rows = sqlx::query!(
            "UPDATE transactions SET status = $1, tx_signature = $2, updated_at = $3 WHERE id = $4",
            status as TransactionStatus,
            tx_signature,
            Utc::now(),
            transaction_id
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        if updated_rows == 0 {
            return Err(StoreError::InvalidInput(
                "Transaction not found".to_string(),
            ));
        }

        Ok(())
    }

    pub async fn get_transaction(&self, transaction_id: Uuid) -> Result<Transaction, StoreError> {
        let transaction = sqlx::query_as!(
            Transaction,
            r#"
            SELECT id, user_id, tx_signature, transaction_type as "transaction_type: TransactionType",
                   status as "status: TransactionStatus", amount, token_mint, from_address, to_address, fee, created_at, updated_at
            FROM transactions WHERE id = $1
            "#,
            transaction_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::InvalidInput("Transaction not found".to_string()))?;

        Ok(transaction)
    }

    /// Get transaction by signature
    pub async fn get_transaction_by_signature(
        &self,
        tx_signature: &str,
    ) -> Result<Transaction, StoreError> {
        let transaction = sqlx::query_as!(
            Transaction,
            r#"
            SELECT id, user_id, tx_signature, transaction_type as "transaction_type: TransactionType",
                   status as "status: TransactionStatus", amount, token_mint, from_address, to_address, fee, created_at, updated_at
            FROM transactions WHERE tx_signature = $1
            "#,
            tx_signature
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::InvalidInput("Transaction not found".to_string()))?;

        Ok(transaction)
    }

    /// Get user transactions with pagination
    pub async fn get_user_transactions(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
        status_filter: Option<TransactionStatus>,
        transaction_type_filter: Option<TransactionType>,
    ) -> Result<Vec<Transaction>, StoreError> {
        let transactions = match (status_filter, transaction_type_filter) {
            (Some(status), Some(tx_type)) => {
                sqlx::query_as!(
                    Transaction,
                    r#"
                    SELECT id, user_id, tx_signature, transaction_type as "transaction_type: TransactionType",
                           status as "status: TransactionStatus", amount, token_mint, from_address, to_address, fee, created_at, updated_at
                    FROM transactions 
                    WHERE user_id = $1 AND status = $2 AND transaction_type = $3
                    ORDER BY created_at DESC 
                    LIMIT $4 OFFSET $5
                    "#,
                    user_id,
                    status as TransactionStatus,
                    tx_type as TransactionType,
                    limit,
                    offset
                )
                .fetch_all(&self.pool)
                .await?
            }
            (Some(status), None) => {
                sqlx::query_as!(
                    Transaction,
                    r#"
                    SELECT id, user_id, tx_signature, transaction_type as "transaction_type: TransactionType",
                           status as "status: TransactionStatus", amount, token_mint, from_address, to_address, fee, created_at, updated_at
                    FROM transactions 
                    WHERE user_id = $1 AND status = $2
                    ORDER BY created_at DESC 
                    LIMIT $3 OFFSET $4
                    "#,
                    user_id,
                    status as TransactionStatus,
                    limit,
                    offset
                )
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(tx_type)) => {
                sqlx::query_as!(
                    Transaction,
                    r#"
                    SELECT id, user_id, tx_signature, transaction_type as "transaction_type: TransactionType",
                           status as "status: TransactionStatus", amount, token_mint, from_address, to_address, fee, created_at, updated_at
                    FROM transactions 
                    WHERE user_id = $1 AND transaction_type = $2
                    ORDER BY created_at DESC 
                    LIMIT $3 OFFSET $4
                    "#,
                    user_id,
                    tx_type as TransactionType,
                    limit,
                    offset
                )
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as!(
                    Transaction,
                    r#"
                    SELECT id, user_id, tx_signature, transaction_type as "transaction_type: TransactionType",
                           status as "status: TransactionStatus", amount, token_mint, from_address, to_address, fee, created_at, updated_at
                    FROM transactions 
                    WHERE user_id = $1
                    ORDER BY created_at DESC 
                    LIMIT $2 OFFSET $3
                    "#,
                    user_id,
                    limit,
                    offset
                )
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(transactions)
    }

    /// Get pending transactions (for processing)
    pub async fn get_pending_transactions(
        &self,
        limit: i64,
    ) -> Result<Vec<Transaction>, StoreError> {
        let transactions = sqlx::query_as!(
            Transaction,
            r#"
            SELECT id, user_id, tx_signature, transaction_type as "transaction_type: TransactionType",
                   status as "status: TransactionStatus", amount, token_mint, from_address, to_address, fee, created_at, updated_at
            FROM transactions 
            WHERE status = $1
            ORDER BY created_at ASC 
            LIMIT $2
            "#,
            TransactionStatus::Pending as TransactionStatus,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(transactions)
    }

    /// Count user transactions
    pub async fn count_user_transactions(
        &self,
        user_id: Uuid,
        status_filter: Option<TransactionStatus>,
        transaction_type_filter: Option<TransactionType>,
    ) -> Result<i64, StoreError> {
        let count = match (status_filter, transaction_type_filter) {
            (Some(status), Some(tx_type)) => {
                sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM transactions WHERE user_id = $1 AND status = $2 AND transaction_type = $3",
                    user_id,
                    status as TransactionStatus,
                    tx_type as TransactionType
                )
                .fetch_one(&self.pool)
                .await?
                .unwrap_or(0)
            }
            (Some(status), None) => {
                sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM transactions WHERE user_id = $1 AND status = $2",
                    user_id,
                    status as TransactionStatus
                )
                .fetch_one(&self.pool)
                .await?
                .unwrap_or(0)
            }
            (None, Some(tx_type)) => {
                sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM transactions WHERE user_id = $1 AND transaction_type = $2",
                    user_id,
                    tx_type as TransactionType
                )
                .fetch_one(&self.pool)
                .await?
                .unwrap_or(0)
            }
            (None, None) => {
                sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM transactions WHERE user_id = $1",
                    user_id
                )
                .fetch_one(&self.pool)
                .await?
                .unwrap_or(0)
            }
        };

        Ok(count)
    }

    /// Get transaction statistics
    pub async fn get_transaction_stats(&self) -> Result<(i64, i64, i64, Decimal), StoreError> {
        // Total transactions, pending, failed, total volume
        let total_transactions = sqlx::query_scalar!("SELECT COUNT(*) FROM transactions")
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0);

        let pending_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM transactions WHERE status = $1",
            TransactionStatus::Pending as TransactionStatus
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        let failed_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM transactions WHERE status = $1",
            TransactionStatus::Failed as TransactionStatus
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);

        let total_volume = sqlx::query_scalar!(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE status = $1",
            TransactionStatus::Confirmed as TransactionStatus
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(Decimal::ZERO);

        Ok((
            total_transactions,
            pending_count,
            failed_count,
            total_volume,
        ))
    }

    /// Process a deposit transaction (updates balance and transaction status)
    pub async fn process_deposit(
        &self,
        transaction_id: Uuid,
        tx_signature: String,
    ) -> Result<(), StoreError> {
        // Use transaction for atomic operation
        let mut tx = self.pool.begin().await?;

        // Get transaction details
        let transaction = sqlx::query!(
            r#"
            SELECT user_id, amount, token_mint, transaction_type as "transaction_type: TransactionType"
            FROM transactions WHERE id = $1 AND status = $2
            "#,
            transaction_id,
            TransactionStatus::Pending as TransactionStatus
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(StoreError::InvalidInput("Pending transaction not found".to_string()))?;

        // Verify it's a deposit transaction
        if !matches!(transaction.transaction_type, TransactionType::Deposit) {
            return Err(StoreError::InvalidInput(
                "Transaction is not a deposit".to_string(),
            ));
        }

        // Update balances
        if let Some(token_mint) = transaction.token_mint {
            // Token deposit - update token balance
            sqlx::query!(
                r#"
                INSERT INTO token_balances (user_id, token_mint, token_symbol, balance, decimals, created_at, updated_at)
                VALUES ($1, $2, 'UNKNOWN', $3, 6, $4, $4)
                ON CONFLICT (user_id, token_mint) 
                DO UPDATE SET 
                    balance = token_balances.balance + EXCLUDED.balance,
                    updated_at = EXCLUDED.updated_at
                "#,
                transaction.user_id,
                token_mint,
                transaction.amount,
                Utc::now()
            )
            .execute(&mut *tx)
            .await?;
        } else {
            // SOL deposit - update user balance
            sqlx::query!(
                "UPDATE users SET balance = balance + $1, updated_at = $2 WHERE id = $3",
                transaction.amount,
                Utc::now(),
                transaction.user_id
            )
            .execute(&mut *tx)
            .await?;
        }

        // Update transaction status
        sqlx::query!(
            "UPDATE transactions SET status = $1, tx_signature = $2, updated_at = $3 WHERE id = $4",
            TransactionStatus::Confirmed as TransactionStatus,
            tx_signature,
            Utc::now(),
            transaction_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Process a withdrawal transaction (updates balance and transaction status)
    pub async fn process_withdrawal(
        &self,
        transaction_id: Uuid,
        tx_signature: String,
    ) -> Result<(), StoreError> {
        // Use transaction for atomic operation
        let mut tx = self.pool.begin().await?;

        // Get transaction details
        let transaction = sqlx::query!(
            r#"
            SELECT user_id, amount, token_mint, transaction_type as "transaction_type: TransactionType"
            FROM transactions WHERE id = $1 AND status = $2
            "#,
            transaction_id,
            TransactionStatus::Pending as TransactionStatus
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(StoreError::InvalidInput("Pending transaction not found".to_string()))?;

        // Verify it's a withdrawal transaction
        if !matches!(transaction.transaction_type, TransactionType::Withdrawal) {
            return Err(StoreError::InvalidInput(
                "Transaction is not a withdrawal".to_string(),
            ));
        }

        // Check and update balances
        if let Some(token_mint) = transaction.token_mint {
            // Token withdrawal - check and update token balance
            let current_balance = sqlx::query_scalar!(
                "SELECT balance FROM token_balances WHERE user_id = $1 AND token_mint = $2",
                transaction.user_id,
                token_mint
            )
            .fetch_optional(&mut *tx)
            .await?
            .unwrap_or(Decimal::ZERO);

            if current_balance < transaction.amount {
                return Err(StoreError::InsufficientBalance);
            }

            sqlx::query!(
                "UPDATE token_balances SET balance = balance - $1, updated_at = $2 WHERE user_id = $3 AND token_mint = $4",
                transaction.amount,
                Utc::now(),
                transaction.user_id,
                token_mint
            )
            .execute(&mut *tx)
            .await?;
        } else {
            // SOL withdrawal - check and update user balance
            let current_balance = sqlx::query_scalar!(
                "SELECT balance FROM users WHERE id = $1",
                transaction.user_id
            )
            .fetch_one(&mut *tx)
            .await?;

            if current_balance < transaction.amount {
                return Err(StoreError::InsufficientBalance);
            }

            sqlx::query!(
                "UPDATE users SET balance = balance - $1, updated_at = $2 WHERE id = $3",
                transaction.amount,
                Utc::now(),
                transaction.user_id
            )
            .execute(&mut *tx)
            .await?;
        }

        // Update transaction status
        sqlx::query!(
            "UPDATE transactions SET status = $1, tx_signature = $2, updated_at = $3 WHERE id = $4",
            TransactionStatus::Confirmed as TransactionStatus,
            tx_signature,
            Utc::now(),
            transaction_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Mark transaction as failed
    pub async fn fail_transaction(
        &self,
        transaction_id: Uuid,
        reason: Option<String>,
    ) -> Result<(), StoreError> {
        // For failed transactions, we might want to store the failure reason
        // For now, we'll just update the status
        let updated_rows = sqlx::query!(
            "UPDATE transactions SET status = $1, updated_at = $2 WHERE id = $3",
            TransactionStatus::Failed as TransactionStatus,
            Utc::now(),
            transaction_id
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        if updated_rows == 0 {
            return Err(StoreError::InvalidInput(
                "Transaction not found".to_string(),
            ));
        }

        Ok(())
    }

    /// Calculate user's total transaction fees
    pub async fn get_user_total_fees(&self, user_id: Uuid) -> Result<Decimal, StoreError> {
        let total_fees = sqlx::query_scalar!(
            "SELECT COALESCE(SUM(fee), 0) FROM transactions WHERE user_id = $1 AND status = $2",
            user_id,
            TransactionStatus::Confirmed as TransactionStatus
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(Decimal::ZERO);

        Ok(total_fees)
    }
}

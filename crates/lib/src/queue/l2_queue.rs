use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Transaction {
    pub id: i64,
    pub stark_pub_key: String,
    pub amount: i64,
    pub token_address: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tx_hash: Option<String>,
    pub error: Option<String>,
    pub proof_data: Option<String>,
    pub retry_count: i32,
}

#[derive(Debug, Error)]
pub enum L2QueueError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Transaction not found: {0}")]
    TransactionNotFound(i64),

    #[error("Invalid transaction status: {0}")]
    InvalidStatus(String),

    #[error("Missing proof data")]
    MissingProofData,

    #[error("Commitment not yet found on L2")]
    CommitmentPending,

    #[error("Max retries exceeded")]
    MaxRetriesExceeded,
}

pub struct QueueConfig {
    pub process_interval_sec: u64,
    pub initial_retry_delay_sec: u64,
    pub max_retries: u32,
    pub batch_size: i64,
}

pub struct L2Queue {
    db_pool: Pool<Postgres>,
    config: QueueConfig,
}

impl L2Queue {
    pub fn new(db_pool: Pool<Postgres>, config: QueueConfig) -> Self {
        Self { db_pool, config }
    }

    pub async fn run(&self) {
        loop {
            match self.process_transactions().await {
                Ok(_) => info!("Processing cycle completed."),
                Err(e) => error!("Processing failed: {:?}", e),
            }
            sleep(Duration::from_secs(self.config.process_interval_sec)).await;
        }
    }

    async fn process_transactions(&self) -> Result<(), L2QueueError> {
        let transactions = self
            .get_pending_transactions_for_proof(self.config.batch_size)
            .await?;

        for tx in transactions {
            let tx_handle = self.db_pool.begin().await?;

            // Optional delay
            sleep(Duration::from_secs(self.config.initial_retry_delay_sec)).await;

            match self.validate_transaction(&tx).await {
                Ok(proof) => {
                    self.mark_transaction_ready_for_relay(tx.id, &proof).await?;
                    tx_handle.commit().await?;
                }
                Err(L2QueueError::CommitmentPending) => {
                    warn!("Commitment pending for tx {}", tx.id);
                    self.increment_retry_count(tx.id).await?;
                    tx_handle.commit().await?;
                }
                Err(L2QueueError::MaxRetriesExceeded) => {
                    error!("Max retries hit for tx {}. Marking failed.", tx.id);
                    self.update_transaction_status(tx.id, "failed", Some("Max retries exceeded"))
                        .await?;
                    tx_handle.commit().await?;
                }
                Err(e) => {
                    error!("Unexpected error for tx {}: {:?}", tx.id, e);
                    self.update_transaction_status(tx.id, "failed", Some(&e.to_string()))
                        .await?;
                    tx_handle.commit().await?;
                }
            }
        }

        Ok(())
    }

    async fn validate_transaction(&self, tx: &L2Transaction) -> Result<String, L2QueueError> {
        trace!("Validating tx: {}", tx.id);

        let proof_data = self.check_l2_commitment(tx).await?;

        if let Some(proof) = proof_data {
            Ok(proof)
        } else {
            let retry_count = tx.retry_count;
            if retry_count + 1 >= self.config.max_retries as i32 {
                Err(L2QueueError::MaxRetriesExceeded)
            } else {
                Err(L2QueueError::CommitmentPending)
            }
        }
    }

    async fn check_l2_commitment(
        &self,
        tx: &L2Transaction,
    ) -> Result<Option<String>, L2QueueError> {
        // Mocked success
        trace!("Checking commitment for tx {}", tx.id);

        // Example logic - replace with real L2 check
        if tx.id % 2 == 0 {
            Ok(Some("mocked_proof_data".into()))
        } else {
            Ok(None)
        }
    }

    async fn increment_retry_count(&self, id: i64) -> Result<(), L2QueueError> {
        sqlx::query!(
            r#"
            UPDATE l2_transactions
            SET retry_count = COALESCE(retry_count, 0) + 1, updated_at = NOW()
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.db_pool)
        .await
        .map_err(L2QueueError::Database)?;

        Ok(())
    }

    async fn mark_transaction_ready_for_relay(
        &self,
        id: i64,
        proof_data: &str,
    ) -> Result<(), L2QueueError> {
        let result = sqlx::query!(
            r#"
            UPDATE l2_transactions
            SET status = 'ready_for_relay', proof_data = $1, updated_at = NOW()
            WHERE id = $2 AND status = 'pending'
            "#,
            proof_data,
            id
        )
        .execute(&self.db_pool)
        .await
        .map_err(L2QueueError::Database)?;

        if result.rows_affected() == 0 {
            return Err(L2QueueError::TransactionNotFound(id));
        }

        Ok(())
    }

    async fn update_transaction_status(
        &self,
        id: i64,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), L2QueueError> {
        let result = match error {
            Some(err) => {
                sqlx::query!(
                    r#"
                UPDATE l2_transactions
                SET status = $1, error = $2, updated_at = NOW()
                WHERE id = $3
                "#,
                    status,
                    err,
                    id
                )
                .execute(&self.db_pool)
                .await
            }
            None => {
                sqlx::query!(
                    r#"
                UPDATE l2_transactions
                SET status = $1, updated_at = NOW()
                WHERE id = $2
                "#,
                    status,
                    id
                )
                .execute(&self.db_pool)
                .await
            }
        };

        if result.map_err(L2QueueError::Database)?.rows_affected() == 0 {
            return Err(L2QueueError::TransactionNotFound(id));
        }

        Ok(())
    }

    async fn get_pending_transactions_for_proof(
        &self,
        limit: i64,
    ) -> Result<Vec<L2Transaction>, L2QueueError> {
        let transactions = sqlx::query_as!(
            L2Transaction,
            r#"
            SELECT * FROM l2_transactions
            WHERE status = 'pending'
            ORDER BY created_at ASC
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(L2QueueError::Database)?;

        Ok(transactions)
    }
}

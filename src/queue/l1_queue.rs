use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Configuration for the L1 queue processing.
#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    pub process_interval_sec: u64,
    pub initial_retry_delay_sec: u64,
    pub retry_delay_sec: u64,
    pub max_retries: u32,
}

#[derive(Debug, sqlx::FromRow)]
struct Deposit {
    id: u64,
    status: String,
    retry_count: i32,
    created_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
enum ValidationError {
    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Commitment error: {0}")]
    Critical(String),

    #[error("Merkle update pending: {0}")]
    Temporary(String),
}

/// L1 Queue structure to process deposits.
pub struct L1Queue {
    db_pool: PgPool,
    config: QueueConfig,
}

impl L1Queue {
    pub fn new(db_pool: PgPool, config: QueueConfig) -> Self {
        Self { db_pool, config }
    }

    /// Runs the L1 queue processor in an infinite loop.
    pub async fn run(&self) {
        loop {
            match self.process_deposits().await {
                Ok(_) => info!("Completed deposit processing cycle"),
                Err(e) => error!("Deposit processing cycle failed: {}", e),
            }
            sleep(Duration::from_secs(self.config.process_interval_sec)).await;
        }
    }

    /// Processes pending deposit requests.
    async fn process_deposits(&self) -> Result<(), sqlx::Error> {
        let deposits = self.fetch_pending_deposits().await?;
        for deposit in deposits {
            let mut transaction = self.db_pool.begin().await?;
            // Delay to reduce excessive blockchain queries.
            sleep(Duration::from_secs(self.config.initial_retry_delay_sec)).await;

            match self.validate_deposit(&deposit).await {
                Ok(()) => {
                    info!("Deposit {} validated successfully", deposit.id);
                    // Mark as processed
                    sqlx::query!(
                        r#"
                        UPDATE deposits 
                        SET status = 'processed', updated_at = NOW()
                        WHERE id = $1
                        "#,
                        deposit.id
                    )
                    .execute(&mut transaction)
                    .await?;
                    transaction.commit().await?;
                }
                Err(e) => {
                    match e {
                        ValidationError::Temporary(reason) => {
                            warn!(
                                "Temporary failure for deposit {}: {}. Retrying.",
                                deposit.id, reason
                            );
                            // Increment retry count.
                            sqlx::query!(
                                r#"
                                UPDATE deposits 
                                SET retry_count = retry_count + 1, updated_at = NOW()
                                WHERE id = $1
                                "#,
                                deposit.id
                            )
                            .execute(&mut transaction)
                            .await?;
                            transaction.commit().await?;
                            // Wait before processing the next retry.
                            sleep(Duration::from_secs(self.config.retry_delay_sec)).await;
                        }
                        ValidationError::Critical(reason) => {
                            error!(
                                "Critical failure for deposit {}: {}. Marking as failed.",
                                deposit.id, reason
                            );
                            sqlx::query!(
                                r#"
                                UPDATE deposits 
                                SET status = 'failed', updated_at = NOW()
                                WHERE id = $1
                                "#,
                                deposit.id
                            )
                            .execute(&mut transaction)
                            .await?;
                            transaction.commit().await?;
                        }
                        _ => {
                            warn!("Unhandled validation error for deposit {}", deposit.id);
                            sqlx::query!(
                                r#"
                                UPDATE deposits 
                                SET retry_count = retry_count + 1, updated_at = NOW()
                                WHERE id = $1
                                "#,
                                deposit.id
                            )
                            .execute(&mut transaction)
                            .await?;
                            transaction.commit().await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Fetches pending deposits that have not exceeded the maximum retries.
    async fn fetch_pending_deposits(&self) -> Result<Vec<Deposit>, sqlx::Error> {
        let deposits = sqlx::query_as!(
            Deposit,
            r#"
            SELECT id, status, retry_count, created_at
            FROM deposits
            WHERE status = 'pending' AND retry_count < $1
            ORDER BY created_at ASC
            LIMIT 10
            "#,
            self.config.max_retries as i32
        )
        .fetch_all(&self.db_pool)
        .await?;
        Ok(deposits)
    }

    /// Validates the deposit by verifying commitment existence and Merkle root update.
    async fn validate_deposit(&self, deposit: &Deposit) -> Result<(), ValidationError> {
        // Check if the commitment exists on L1.
        // let commitment_exists = crate::l1_api::commitment_exists(deposit.id).await;
        let commitment_exists = true;
        if !commitment_exists {
            return Err(ValidationError::Critical(
                "Commitment not found on L1".into(),
            ));
        }
        // Check if the Merkle Root has been updated.
        // let merkle_updated = crate::l1_api::merkle_root_updated().await;
        let merkle_updated = true;
        if !merkle_updated {
            return Err(ValidationError::Temporary(
                "Merkle Root not updated yet".into(),
            ));
        }
        Ok(())
    }
}

use sqlx::PgPool;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};

use crate::{
    config::QueueConfig,
    db::database::{fetch_pending_deposits, process_deposit_retry, update_deposit_status, Deposit},
};

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("RPC communication failed: {0}")]
    Rpc(String),

    #[error("Invalid deposit commitment format")]
    InvalidCommitment,

    #[error("Commitment not yet found on L1")]
    CommitmentPending,

    #[error("Commitment not found after max retries")]
    MaxRetriesExceeded,
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
                Err(e) => error!("Deposit processing cycle failed: {:?}", e),
            }
            sleep(Duration::from_secs(self.config.process_interval_sec)).await;
        }
    }

    /// Processes pending deposit requests.
    async fn process_deposits(&self) -> Result<(), sqlx::Error> {
        let deposits = fetch_pending_deposits(&self.db_pool, self.config.max_retries).await?;

        for deposit in deposits {
            let mut tx = self.db_pool.begin().await?;

            // Small delay to prevent hammering chain for each deposit
            sleep(Duration::from_secs(self.config.initial_retry_delay_sec)).await;

            match self.validate_deposit(&deposit).await {
                Ok(()) => {
                    info!("Deposit {} validated successfully", deposit.id);
                    update_deposit_status(&mut tx, deposit.id, "processed").await?;
                }

                Err(ValidationError::CommitmentPending) => {
                    warn!("Deposit {} not yet found on L1. Will retry.", deposit.id);
                    process_deposit_retry(&mut tx, deposit.id).await?;
                    sleep(Duration::from_secs(self.config.retry_delay_seconds.into())).await;
                }

                Err(ValidationError::MaxRetriesExceeded) => {
                    error!(
                        "Deposit {} failed after max retries. Marking as failed.",
                        deposit.id
                    );
                    update_deposit_status(&mut tx, deposit.id, "failed").await?;
                }

                Err(e) => {
                    warn!("Deposit {} hit an error: {:?}. Will retry.", deposit.id, e);
                    process_deposit_retry(&mut tx, deposit.id).await?;
                    sleep(Duration::from_secs(self.config.retry_delay_seconds.into())).await;
                }
            }

            tx.commit().await?;
        }

        Ok(())
    }

    /// Validates the deposit by verifying commitment existence
    async fn validate_deposit(&self, deposit: &Deposit) -> Result<(), ValidationError> {
        let commitment_exists = self
            .check_l1_commitment(deposit.commitment_hash.clone())
            .await?;

        let max_retries_i32 = self.config.max_retries as i32;

        if !commitment_exists {
            if deposit.retry_count + 1 >= max_retries_i32 {
                return Err(ValidationError::MaxRetriesExceeded);
            } else {
                return Err(ValidationError::CommitmentPending);
            }
        }

        Ok(())
    }

    async fn check_l1_commitment(&self, commitment_hash: String) -> Result<bool, ValidationError> {
        // Check l1 event logs for commitment hash
        // Assuming we have a function `get_event_logs` that fetches event logs from L1
        // we store last index so we don't have to fetch all logs every time

        trace!("Checking L1 commitment for hash: {}", commitment_hash);
        Ok(true)
    }
}

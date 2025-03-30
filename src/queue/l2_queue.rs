use serde::Deserialize;
use sqlx::PgPool;
use starknet::core::types::Felt;
use starknet::providers::{AnyProvider, Provider};
use tracing::{info, warn, error};
use std::time::Duration;
use tokio::time::sleep;
use chrono;

// Configuration structure (loaded from config.toml)
#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    pub process_interval_sec: u64,
    pub initial_retry_delay_sec: u64,
    pub max_retries: u32,
    pub merkle_update_confirmations: usize,
}


#[derive(Debug, sqlx::FromRow)]
struct Withdrawal {
    id: i32,
    user_address: String,
    l2_token: String,
    amount: String,
    status: String,
    commitment_hash: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    retry_count: i32,
}

#[derive(Debug, thiserror::Error)]
enum ValidationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("RPC error: {0}")]
    RpcError(String),
    
    #[error("Invalid commitment format")]
    InvalidCommitment,
    
    #[error("Merkle root not updated")]
    MerkleRootNotUpdated,
}

pub struct L2Queue {
    db_pool: PgPool,
    starknet_client: AnyProvider,
    config: QueueConfig,
}

impl L2Queue {
    pub fn new(db_pool: PgPool, starknet_client: AnyProvider, config: QueueConfig) -> Self {
        Self {
            db_pool,
            starknet_client,
            config,
        }
    }

    pub async fn run(&self) {
        loop {
            match self.process_withdrawals().await {
                Ok(_) => info!("Completed processing cycle"),
                Err(e) => error!("Processing cycle failed: {}", e),
            }
            sleep(Duration::from_secs(self.config.process_interval_sec)).await;
        }
    }

    async fn process_withdrawals(&self) -> Result<(), sqlx::Error> {
        let withdrawals = self.fetch_pending_withdrawals().await?;
        
        for withdrawal in withdrawals {
            let mut transaction = self.db_pool.begin().await?;
            
            // Apply processing delay
            sleep(Duration::from_secs(self.config.initial_retry_delay_sec)).await;

            match self.validate_withdrawal(&withdrawal).await {
                Ok(true) => {
                    self.mark_for_proof_generation(&mut transaction, &withdrawal).await?;
                    transaction.commit().await?;
                    info!("Withdrawal {} validated successfully", withdrawal.id);
                }
                Ok(false) => {
                    self.handle_retry(&mut transaction, &withdrawal, "Validation failed").await?;
                    transaction.commit().await?;
                }
                Err(e) => {
                    self.handle_critical_error(&mut transaction, &withdrawal, &e.to_string()).await?;
                    transaction.commit().await?;
                }
            }
        }
        
        Ok(())
    }

    async fn fetch_pending_withdrawals(&self) -> Result<Vec<Withdrawal>, sqlx::Error> {
        sqlx::query_as!(
            Withdrawal,
            r#"
            SELECT * FROM withdrawals
            WHERE status = 'pending'
            AND retry_count < $1
            ORDER BY created_at ASC
            LIMIT 10
            "#,
            self.config.max_retries as i32
        )
        .fetch_all(&self.db_pool)
        .await
    }

    async fn validate_withdrawal(&self, withdrawal: &Withdrawal) -> Result<bool, ValidationError> {
        // Check commitment exists on L2
        let commitment_exists = self.check_l2_commitment(withdrawal.commitment_hash.clone()).await?;
        // Verify Merkle root updates
        let merkle_updated = self.verify_merkle_update(withdrawal.created_at).await?;

        Ok(commitment_exists && merkle_updated)
    }

    async fn check_l2_commitment(&self, commitment_hash: String) -> Result<bool, ValidationError> {
        let commitment = Felt::from_hex(&commitment_hash)
            .map_err(|_| ValidationError::InvalidCommitment)?;

        // Starknet contract call implementation would go here
        let result = self.starknet_client
            .get_transaction_status(commitment)
            .await
            .map_err(|e| ValidationError::RpcError(e.to_string()))?;

        Ok(result.is_accepted_on_l2())
    }

    async fn verify_merkle_update(&self, created_at: chrono::DateTime<chrono::Utc>) -> Result<bool, ValidationError> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM merkle_roots WHERE chain = 'L2' AND created_at > $1"#,
            created_at
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(count >= self.config.merkle_update_confirmations as i64)
    }

    async fn handle_retry(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        withdrawal: &Withdrawal,
        reason: &str,
    ) -> Result<(), sqlx::Error> {
        warn!("Retrying withdrawal {}: {}", withdrawal.id, reason);
        
        sqlx::query!(
            r#"
            UPDATE withdrawals 
            SET retry_count = retry_count + 1,
                updated_at = NOW()
            WHERE id = $1
            "#,
            withdrawal.id
        )
        .execute(transaction)
        .await?;

        Ok(())
    }

    async fn handle_critical_error(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        withdrawal: &Withdrawal,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        error!("Critical error for withdrawal {}: {}", withdrawal.id, error);
        
        sqlx::query!(
            r#"
            UPDATE withdrawals 
            SET status = 'failed',
                updated_at = NOW(),
                error_message = $2
            WHERE id = $1
            "#,
            withdrawal.id,
            error
        )
        .execute(transaction)
        .await?;

        Ok(())
    }

    async fn mark_for_proof_generation(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        withdrawal: &Withdrawal,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE withdrawals 
            SET status = 'processing',
                updated_at = NOW()
            WHERE id = $1
            "#,
            withdrawal.id
        )
        .execute(transaction)
        .await?;

        Ok(())
    }
}
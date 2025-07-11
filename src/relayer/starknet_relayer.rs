use crate::queue::l2_queue::L2Transaction;
use sqlx::{Pool, Postgres};
use starknet::accounts::ConnectedAccount;
use starknet::accounts::ExecutionEncoding;
use starknet::core::chain_id::MAINNET;
use starknet::core::types::ExecutionResult;
use starknet::core::types::StarknetError;
// or TESTNET
use starknet::core::types::{Felt, TransactionReceipt};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::jsonrpc::JsonRpcClient;
use starknet::providers::Provider;
use starknet::providers::ProviderError;
use starknet::signers::SigningKey;
use starknet::{accounts::SingleOwnerAccount, signers::LocalWallet};
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use url::Url;

// Define custom error types for the Starknet Relayer
#[derive(Error, Debug)]
pub enum StarknetRelayerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Parse error: {0}")]
    ParseError(#[from] starknet::core::types::FromStrError),

    #[error("Transaction not found")]
    TransactionNotFound,

    #[error("Proof data missing")]
    ProofDataMissing,

    #[error("Invalid contract address")]
    InvalidContractAddress,

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Transaction timeout")]
    TransactionTimeout,

    // ✅ Add these if they're used
    #[error("Selector parse failed")]
    SelectorParseFailed,

    #[error("Request timed out")]
    Timeout,

    #[error("Timeout error: {0}")]
    TimeoutError(String),
}

// Configuration for the Starknet Relayer
#[derive(Debug, Clone)]
pub struct StarknetRelayerConfig {
    pub bridge_contract_address: String,
    pub rpc_url: String,
    pub account_address: String,
    pub private_key: String,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub transaction_timeout_ms: u64,
}

// The main Starknet Relayer struct
pub struct StarknetRelayer {
    db_pool: Pool<Postgres>,
    config: StarknetRelayerConfig,
    account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
}

impl StarknetRelayer {
    pub async fn new(
        db_pool: Pool<Postgres>,
        config: StarknetRelayerConfig,
    ) -> Result<Self, StarknetRelayerError> {
        let provider = JsonRpcClient::new(HttpTransport::new(
            Url::parse(&config.rpc_url.clone()).unwrap(),
        ));
        let signer: LocalWallet = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex(&config.private_key).unwrap(),
        ));
        let chain_id = MAINNET;
        let address = Felt::from_hex(&config.account_address).unwrap();
        let account =
            SingleOwnerAccount::new(provider, signer, address, chain_id, ExecutionEncoding::New);
        Ok(Self {
            db_pool,
            config,
            account,
        })
    }

    // Main function to start the relayer process
    pub async fn start(&self) -> Result<(), StarknetRelayerError> {
        info!("Starting Starknet Relayer service");

        loop {
            match self.process_pending_transactions().await {
                Ok(processed) => {
                    if processed > 0 {
                        info!("Successfully processed {} Starknet transactions", processed);
                    } else {
                        debug!("No pending Starknet transactions to process");
                    }
                }
                Err(e) => {
                    error!("Error processing Starknet transactions: {:?}", e);
                }
            }

            // Sleep before the next iteration
            sleep(Duration::from_secs(10)).await;
        }
    }

    // Process all pending transactions
    pub async fn process_pending_transactions(&self) -> Result<usize, StarknetRelayerError> {
        let mut processed_count = 0;

        // Fetch all transactions marked as "ready for relay"
        let transactions = self.fetch_ready_transactions().await?;

        for mut tx in transactions {
            match self.process_transaction(&mut tx).await {
                Ok(_) => {
                    processed_count += 1;
                }
                Err(e) => {
                    error!("Failed to process transaction {}: {:?}", tx.id, e);
                    self.mark_transaction_failed(&tx, &e.to_string()).await?;
                }
            }
        }

        Ok(processed_count)
    }

    // Fetch transactions marked as "ready for relay"
    pub async fn fetch_ready_transactions(
        &self,
    ) -> Result<Vec<L2Transaction>, StarknetRelayerError> {
        let transactions = sqlx::query_as!(
            L2Transaction,
            r#"
                SELECT * FROM l2_transactions
                WHERE status = 'ready_for_relay'
                ORDER BY created_at ASC
                LIMIT 10
                "#
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(StarknetRelayerError::Database)?;

        Ok(transactions)
    }

    // Process a single transaction
    pub async fn process_transaction(
        &self,
        tx: &mut L2Transaction,
    ) -> Result<(), StarknetRelayerError> {
        info!("Processing L2 transaction {}", &tx.id);

        // Mark transaction as processing
        self.mark_transaction_processing(&tx).await?;

        // Extract proof data from the transaction
        let proof_data = tx
            .proof_data
            .clone()
            .ok_or(StarknetRelayerError::ProofDataMissing)?;

        // Attempt to relay the transaction with retries
        let mut attempts = 0;
        let max_retries = self.config.max_retries;

        loop {
            attempts += 1;

            match self.relay_to_starknet(&tx.clone(), &proof_data).await {
                Ok(tx_hash) => {
                    // Wait for transaction confirmation
                    match self.wait_for_transaction_confirmation(tx_hash).await {
                        Ok(_) => {
                            // Mark transaction as completed
                            self.mark_transaction_completed(&tx, &tx_hash.to_string())
                                .await?;
                            info!(
                                "Transaction {} successfully processed on Starknet (hash: {})",
                                tx.id, tx_hash
                            );
                            return Ok(());
                        }
                        Err(e) => {
                            warn!(
                                "Transaction {} submitted but confirmation failed: {:?}",
                                tx.id, e
                            );

                            if attempts >= max_retries {
                                return Err(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to relay transaction {} (attempt {}/{}): {:?}",
                        tx.id, attempts, max_retries, e
                    );

                    if attempts >= max_retries {
                        return Err(e);
                    }
                }
            }

            // Delay before retry
            let retry_delay = Duration::from_millis(self.config.retry_delay_ms);
            sleep(retry_delay).await;
        }
    }

    // Relay transaction to Starknet
    pub async fn relay_to_starknet(
        &self,
        tx: &L2Transaction,
        proof_data: &str,
    ) -> Result<Felt, StarknetRelayerError> {
        // Parse proof data from JSON
        let proof: serde_json::Value = serde_json::from_str(proof_data).map_err(|e| {
            StarknetRelayerError::TransactionFailed(format!("Invalid proof data: {}", e))
        })?;

        // Extract withdrawal ID from transaction
        let withdrawal_id = tx.id.clone();
        
        // Extract proof array and merkle root from proof data
        let proof_array = match proof.get("proof") {
            Some(array) if array.is_array() => {
                let mut felts = Vec::new();
                for item in array.as_array().unwrap() {
                    if let Some(s) = item.as_str() {
                        felts.push(Felt::from_hex(s).map_err(|_| {
                            StarknetRelayerError::TransactionFailed("Invalid proof element".to_string())
                        })?);
                    } else {
                        return Err(StarknetRelayerError::TransactionFailed(
                            "Proof array contains non-string elements".to_string(),
                        ));
                    }
                }
                felts
            }
            _ => return Err(StarknetRelayerError::ProofDataMissing),
        };

        let merkle_root = match proof.get("merkle_root") {
            Some(value) => {
                if let Some(s) = value.as_str() {
                    Felt::from_hex(s).map_err(|_| {
                        StarknetRelayerError::TransactionFailed("Invalid merkle root".to_string())
                    })?
                } else {
                    return Err(StarknetRelayerError::ProofDataMissing);
                }
            }
            _ => return Err(StarknetRelayerError::ProofDataMissing),
        };

        // Initialize calldata with basic fields
        let mut calldata = Vec::new();
        
        // Add withdrawal ID as a felt
        calldata.push(Felt::from_u64(withdrawal_id as u64));
        
        // Add proof array length
        calldata.push(Felt::from_u64(proof_array.len() as u64));

        // Extend calldata with proof array elements
        calldata.extend(proof_array);

        // Add merkle root at the end
        calldata.push(merkle_root);

        // Get the contract address
        let contract_address = Felt::from_hex(&self.config.bridge_contract_address)
            .map_err(|_| StarknetRelayerError::InvalidContractAddress)?;

        // Create the call
        use starknet::core::types::{Call, FunctionCall};
        use starknet::macros::selector;
        
        let calls = vec![Call {
            to: contract_address,
            selector: selector!("process_withdrawal"),
            calldata,
        }];

        // Execute the transaction
        info!("Sending transaction to Starknet contract: {}", &self.config.bridge_contract_address);
        
        // Execute the call and get the transaction hash
        let result = match self.account.execute(calls).send().await {
            Ok(result) => {
                info!("Transaction sent successfully with hash: {}", result.transaction_hash);
                result.transaction_hash
            }
            Err(e) => {
                error!("Failed to send transaction: {:?}", e);
                return Err(StarknetRelayerError::TransactionFailed(format!("Failed to send transaction: {}", e)));
            }
        };

        Ok(result)
    }

    // Wait for transaction confirmation

    pub async fn wait_for_transaction_confirmation(
        &self,
        tx_hash: Felt,
    ) -> Result<(), StarknetRelayerError> {
        let timeout = Duration::from_millis(self.config.transaction_timeout_ms);
        let start_time = std::time::Instant::now();

        loop {
            // Timeout check
            if start_time.elapsed() > timeout {
                return Err(StarknetRelayerError::TimeoutError(
                    "Transaction confirmation timed out.".to_string(),
                ));
            }

            match self
                .account
                .provider()
                .get_transaction_receipt(tx_hash)
                .await
            {
                Ok(receipt) => {
                    match receipt.receipt {
                        TransactionReceipt::Invoke(receipt) => match receipt.execution_result {
                            ExecutionResult::Succeeded => return Ok(()),
                            ExecutionResult::Reverted { reason } => {
                                return Err(StarknetRelayerError::TransactionFailed(
                                    reason.to_string(),
                                ));
                            }
                        },
                        _ => {
                            // Other receipt types — keep polling
                        }
                    }
                }
                Err(ProviderError::StarknetError(StarknetError::TransactionHashNotFound)) => {
                    // Hash not found yet — retry
                }
                Err(e) => return Err(StarknetRelayerError::Provider(e)),
            }

            // Sleep for a short duration before retrying
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    // Mark transaction as processing in the database
    pub async fn mark_transaction_processing(
        &self,
        tx: &L2Transaction,
    ) -> Result<(), StarknetRelayerError> {
        sqlx::query!(
            r#"
                UPDATE l2_transactions
                SET status = 'processing', updated_at = NOW()
                WHERE id = $1
                "#,
            tx.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(StarknetRelayerError::Database)?;

        Ok(())
    }

    // Mark transaction as completed in the database
    pub async fn mark_transaction_completed(
        &self,
        tx: &L2Transaction,
        tx_hash: &str,
    ) -> Result<(), StarknetRelayerError> {
        sqlx::query!(
            r#"
                UPDATE l2_transactions
                SET status = 'completed', tx_hash = $1, updated_at = NOW()
                WHERE id = $2
                "#,
            tx_hash,
            tx.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(StarknetRelayerError::Database)?;

        Ok(())
    }

    // Mark transaction as failed in the database
    pub async fn mark_transaction_failed(
        &self,
        tx: &L2Transaction,
        error_message: &str,
    ) -> Result<(), StarknetRelayerError> {
        sqlx::query!(
            r#"
                UPDATE l2_transactions
                SET status = 'failed', error = $1, updated_at = NOW()
                WHERE id = $2
                "#,
            error_message,
            tx.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(StarknetRelayerError::Database)?;

        Ok(())
    }
}

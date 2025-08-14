use crate::queue::l2_queue::L2Transaction;
use sqlx::{Pool, Postgres};
use starknet::accounts::Account;
use starknet::accounts::ConnectedAccount;
use starknet::accounts::ExecutionEncoding;
use starknet::core::chain_id::MAINNET;
use starknet::core::types::ExecutionResult;
use starknet::core::types::StarknetError;
use starknet::core::types::{Call, Felt, TransactionReceipt};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::jsonrpc::JsonRpcClient;
use starknet::providers::Provider;
use starknet::providers::ProviderError;
use starknet::signers::SigningKey;
use starknet::{accounts::SingleOwnerAccount, signers::LocalWallet};
use std::ops::Deref;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use url::Url;

const MINT_AND_CLAIM_SELECTOR: starknet_crypto::Felt =
    starknet::macros::selector!("mint_and_claim_xzb");
const REGISTER_DEPOSIT_PROOF: starknet_crypto::Felt =
    starknet::macros::selector!("register_deposit_proof");

struct U256(starknet::core::types::U256);

impl Deref for U256 {
    type Target = starknet::core::types::U256;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&str> for U256 {
    fn from(s: &str) -> Self {
        let s = s.strip_prefix("0x").unwrap_or(s);
        if s.len() <= 32 {
            let low = u128::from_str_radix(s, 16).unwrap_or(0);
            let high = 0_u128;
            return U256(starknet::core::types::U256::from_words(
                low, high,
            ));
        } else { 
            let low = u128::from_str_radix(&s[s.len() - 32..], 16).unwrap_or(0);
            let high = u128::from_str_radix(&s[..s.len() - 32], 16).unwrap_or(0);
            return U256(starknet::core::types::U256::from_words(
                low, high,
            ));
        }
    }
}

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
    pub proof_registry_contract_address: String,
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
        let chain_id = provider.chain_id().await.unwrap_or(MAINNET);
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
            StarknetRelayerError::TransactionFailed(format!("Invalid proof data: {e}"))
        })?;

        // Extract proof array and merkle root from proof data
        let proof_array = match proof.get("proof_array") {
            Some(array) if array.is_array() => {
                let mut felts = Vec::new();
                for item in array.as_array().unwrap() {
                    if let Some(s) = item.as_str() {
                        felts.push(Felt::from_hex(s).map_err(|_| {
                            StarknetRelayerError::TransactionFailed(
                                "Invalid proof element".to_string(),
                            )
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

        let commitment_hash = match proof.get("commitment_hash") {
            Some(value) => {
                if let Some(s) = value.as_str() {
                    Felt::from_hex(s).map_err(|_| {
                        StarknetRelayerError::TransactionFailed(
                            "Invalid commitment hash".to_string(),
                        )
                    })?
                } else {
                    return Err(StarknetRelayerError::ProofDataMissing);
                }
            }
            _ => return Err(StarknetRelayerError::ProofDataMissing),
        };

        let eth_address = match proof.get("eth_address") {
            Some(value) => {
                if let Some(s) = value.as_str() {
                    Felt::from_hex(s).map_err(|_| {
                        StarknetRelayerError::TransactionFailed("Invalid ETH address".to_string())
                    })?
                } else {
                    return Err(StarknetRelayerError::ProofDataMissing);
                }
            }
            _ => return Err(StarknetRelayerError::ProofDataMissing),
        };

        let r = match proof.get("r") {
            Some(value) => {
                if let Some(s) = value.as_str() {
                    U256::from(s)
                } else {
                    return Err(StarknetRelayerError::ProofDataMissing);
                }
            }
            _ => return Err(StarknetRelayerError::ProofDataMissing),
        };
        let s = match proof.get("s") {
            Some(value) => {
                if let Some(s) = value.as_str() {
                    U256::from(s)
                } else {
                    return Err(StarknetRelayerError::ProofDataMissing);
                }
            }
            _ => return Err(StarknetRelayerError::ProofDataMissing),
        };
        let y_parity: bool = match proof.get("y_parity") {
            Some(value) => {
                if let Some(b) = value.as_bool() {
                    b
                } else {
                    return Err(StarknetRelayerError::ProofDataMissing);
                }
            }
            _ => return Err(StarknetRelayerError::ProofDataMissing),
        };

        let register_deposit_proof_call = self
            .prepare_register_deposit_proof_call(commitment_hash, merkle_root)
            .await?;

        let mint_and_claim_call = self
            .prepare_mint_and_claim_call(&proof_array, commitment_hash, eth_address, r, s, y_parity)
            .await?;

        let calls = vec![register_deposit_proof_call, mint_and_claim_call];

        // Execute the transaction
        info!(
            "Sending transaction to Starknet contract: {}",
            &self.config.bridge_contract_address
        );

        // Execute the call and get the transaction hash
        let result = match self.account.execute_v3(calls).send().await {
            Ok(result) => {
                info!(
                    "Transaction sent successfully with hash: {}",
                    result.transaction_hash
                );
                result.transaction_hash
            }
            Err(e) => {
                error!("Failed to send transaction: {:?}", e);
                return Err(StarknetRelayerError::TransactionFailed(format!(
                    "Failed to send transaction: {}",
                    e
                )));
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
    async fn mark_transaction_processing(
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
    async fn mark_transaction_completed(
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
    async fn mark_transaction_failed(
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

    async fn prepare_register_deposit_proof_call(
        &self,
        commitment_hash: Felt,
        merkle_root: Felt,
    ) -> Result<Call, StarknetRelayerError> {
        // Get the contract address
        let contract_address = Felt::from_hex(&self.config.proof_registry_contract_address)
            .map_err(|_| StarknetRelayerError::InvalidContractAddress)?;

        // Create the call
        Ok(Call {
            to: contract_address,
            selector: REGISTER_DEPOSIT_PROOF,
            calldata: vec![commitment_hash, merkle_root],
        })
    }

    async fn prepare_mint_and_claim_call(
        &self,
        proof_array: &Vec<Felt>,
        commitment_hash: Felt,
        eth_address: Felt,
        r: U256,
        s: U256,
        y_parity: bool,
    ) -> Result<Call, StarknetRelayerError> {
        // Get the contract address
        let contract_address = Felt::from_hex(&self.config.bridge_contract_address)
            .map_err(|_| StarknetRelayerError::InvalidContractAddress)?;

        let mut calldata: Vec<Felt> = vec![];
        calldata.push(Felt::from(proof_array.len()));
        calldata.extend(proof_array);
        calldata.push(commitment_hash);
        calldata.push(eth_address);
        calldata.push(Felt::from(r.low()));
        calldata.push(Felt::from(r.high()));
        calldata.push(Felt::from(s.low()));
        calldata.push(Felt::from(s.high()));
        calldata.push(Felt::from(y_parity));

        Ok(Call {
            to: contract_address,
            selector: MINT_AND_CLAIM_SELECTOR,
            calldata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing_u256_hex_string_low() {
        let hex_string = "0x1";
        let result = U256::from(hex_string);
        assert_eq!(result.low(), 1);
        assert_eq!(result.high(), 0);
    }

    #[test]
    fn test_parsing_u256_hex_string_high() {
        let hex_string = "0x200000000000000000000000000000000";
        let result = U256::from(hex_string);
        assert_eq!(result.low(), 0);
        assert_eq!(result.high(), 2);
    }

    #[test]
    fn test_parsing_u256_hex_string() {
        let hex_string = "0x0000200000000000000000000000000000001";
        let result = U256::from(hex_string);
        assert_eq!(result.low(), 1);
        assert_eq!(result.high(), 2);
    }

    #[test]
    fn test_parsing_u256_hex_string_without_prefix() {
        let hex_string = "0000200000000000000000000000000000001";
        let result = U256::from(hex_string);
        assert_eq!(result.low(), 1);
        assert_eq!(result.high(), 2);
    }


}
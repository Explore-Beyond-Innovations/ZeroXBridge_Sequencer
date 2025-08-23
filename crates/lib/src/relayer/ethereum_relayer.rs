use crate::config::RelayerConfig;
use alloy_json_rpc::RpcError;
use alloy_primitives::{hex, Address, U256};
use alloy_rpc_client::{ClientBuilder, RpcClient};
use alloy_sol_types::{sol, SolCall};
use sqlx::{PgConnection, PgPool};
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{error, info, trace, warn};
use url::Url;

sol! {
    function unlock_funds_with_proof(
        uint256[] calldata proofParams,
        uint256[] calldata proof,
        uint256 stark_pub_key,
        uint256 amount,
        uint256 l2TxId,
        bytes32 commitmentHash
    ) external;
}

#[derive(Debug, Error)]
pub enum RelayerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Ethereum RPC error: {0}")]
    RpcError(String),

    #[error("Contract error: {0}")]
    ContractError(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Failed to fetch proof: {0}")]
    ProofFetchFailed(String),
}

/// Data structure for withdrawal with proof
#[derive(Debug)]
pub struct WithdrawalWithProof {
    pub withdrawal_id: i32,
    pub stark_pub_key: String,
    pub amount: i64,
    pub l2_tx_id: String,
    pub commitment_hash: String,
    pub proof_params: Vec<u8>,
    pub proof_data: Vec<u8>,
}

/// Relayer for sending L1 transactions to Ethereum
pub struct EthereumRelayer {
    db_pool: PgPool,
    client: RpcClient,
    contract_address: Address,
    config: RelayerConfig,
}

impl EthereumRelayer {
    /// Create a new Ethereum relayer
    pub async fn new(
        db_pool: PgPool,
        ethereum_rpc_url: Url,
        contract_address: &str,
        config: RelayerConfig,
    ) -> Result<Self, RelayerError> {
        let client = ClientBuilder::default().http(ethereum_rpc_url);

        let contract_address = contract_address
            .parse::<Address>()
            .map_err(|e| RelayerError::ContractError(format!("Invalid contract address: {}", e)))?;

        Ok(Self {
            db_pool,
            client,
            contract_address,
            config,
        })
    }

    /// Run the relayer in an infinite loop
    pub async fn run(&self) {
        loop {
            match self.process_relay_transactions().await {
                Ok(_) => info!("Completed relay processing cycle"),
                Err(e) => error!("Relay processing cycle failed: {:?}", e),
            }
            sleep(Duration::from_secs(self.config.retry_delay_seconds.into())).await;
        }
    }

    /// Process transactions that are ready to be relayed
    async fn process_relay_transactions(&self) -> Result<(), RelayerError> {
        let withdrawals_to_relay = self.fetch_ready_for_relay_withdrawals().await?;

        for withdrawal in withdrawals_to_relay {
            let mut tx = self.db_pool.begin().await?;

            match self.relay_transaction(&withdrawal).await {
                Ok(_) => {
                    info!(
                        "Successfully relayed transaction for withdrawal {}",
                        withdrawal.withdrawal_id
                    );
                    self.update_withdrawal_status(&mut tx, withdrawal.withdrawal_id, "relayed")
                        .await?;
                }
                Err(e) => {
                    let retry_count = self.get_retry_count(&withdrawal.withdrawal_id).await?;
                    if retry_count >= self.config.max_retries as i32 - 1 {
                        error!(
                            "Max retries reached for withdrawal {}. Marking as failed: {:?}",
                            withdrawal.withdrawal_id, e
                        );
                        self.update_withdrawal_status(&mut tx, withdrawal.withdrawal_id, "failed")
                            .await?;
                    } else {
                        warn!(
                            "Failed to relay transaction for withdrawal {}. Will retry: {:?}",
                            withdrawal.withdrawal_id, e
                        );
                        self.increment_retry_count(&mut tx, withdrawal.withdrawal_id)
                            .await?;
                    }
                }
            }

            tx.commit().await?;

            // small delay between sending transactions to avoid nonce issues
            sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }

    /// Get the retry count for a withdrawal
    async fn get_retry_count(&self, withdrawal_id: &i32) -> Result<i32, RelayerError> {
        let retry_count = sqlx::query!(
            r#"
            SELECT retry_count FROM withdrawals
            WHERE id = $1
            "#,
            withdrawal_id
        )
        .fetch_one(&self.db_pool)
        .await?
        .retry_count;

        Ok(retry_count)
    }

    /// Fetch withdrawals that are ready to be relayed with their proofs
    async fn fetch_ready_for_relay_withdrawals(
        &self,
    ) -> Result<Vec<WithdrawalWithProof>, RelayerError> {
        let records = sqlx::query!(
            r#"
            SELECT d.id, d.stark_pub_key, d.amount, d.l2_tx_id, d.commitment_hash, 
                  dp.proof_params, dp.proof_data 
            FROM withdrawals d
            JOIN withdrawal_proofs dp ON d.id = dp.withdrawal_id
            WHERE d.status = 'ready_for_relay' AND d.retry_count < $1 AND dp.status = 'ready'
            ORDER BY d.created_at ASC
            LIMIT 10
            "#,
            self.config.max_retries as i32
        )
        .fetch_all(&self.db_pool)
        .await?;

        let withdrawals_with_proofs = records
            .into_iter()
            .map(|row| WithdrawalWithProof {
                withdrawal_id: row.id,
                stark_pub_key: row.stark_pub_key,
                amount: row.amount,
                l2_tx_id: row.l2_tx_id.map_or(String::new(), |id| id.to_string()),
                commitment_hash: row.commitment_hash,
                proof_params: row.proof_params.unwrap_or_default(),
                proof_data: row.proof_data.unwrap_or_default(),
            })
            .collect();

        Ok(withdrawals_with_proofs)
    }

    /// Update the status of a withdrawal
    async fn update_withdrawal_status(
        &self,
        conn: &mut PgConnection,
        id: i32,
        status: &str,
    ) -> Result<(), RelayerError> {
        sqlx::query!(
            r#"
            UPDATE withdrawals 
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            "#,
            id,
            status
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    /// Increment the retry count for a withdrawal
    async fn increment_retry_count(
        &self,
        conn: &mut PgConnection,
        id: i32,
    ) -> Result<(), RelayerError> {
        sqlx::query!(
            r#"
            UPDATE withdrawals 
            SET retry_count = retry_count + 1, updated_at = NOW()
            WHERE id = $1
            "#,
            id
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    /// Relay a transaction to Ethereum
    async fn relay_transaction(
        &self,
        withdrawal: &WithdrawalWithProof,
    ) -> Result<(), RelayerError> {
        // Try to send the transaction with retry logic
        let mut retry_count = 0;
        while retry_count < self.config.max_retries {
            trace!(
                "Attempting to relay transaction for withdrawal {}, attempt {}/{}",
                withdrawal.withdrawal_id,
                retry_count + 1,
                self.config.max_retries
            );

            match self.send_unlock_funds_transaction(withdrawal).await {
                Ok(_) => {
                    info!(
                        "Transaction for withdrawal {} successfully sent",
                        withdrawal.withdrawal_id
                    );
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to send transaction for withdrawal {}: {:?}. Retrying in {} seconds...", 
                          withdrawal.withdrawal_id, e, self.config.retry_delay_seconds);

                    retry_count += 1;
                    if retry_count < self.config.max_retries {
                        sleep(Duration::from_secs(self.config.retry_delay_seconds.into())).await;
                    }
                }
            }
        }

        Err(RelayerError::TransactionFailed(format!(
            "Failed to send transaction after {} retries",
            self.config.max_retries
        )))
    }

    /// Send an Ethereum transaction to the unlock_funds_with_proof function
    async fn send_unlock_funds_transaction(
        &self,
        withdrawal: &WithdrawalWithProof,
    ) -> Result<(), RelayerError> {
        let accounts: Vec<Address> = self
            .client
            .request_noparams("eth_accounts")
            .await
            .map_err(|e| RelayerError::RpcError(e.to_string()))?;

        if accounts.is_empty() {
            return Err(RelayerError::RpcError("No accounts available".to_string()));
        }

        let from = accounts[0];

        let gas_price: U256 = self
            .client
            .request_noparams("eth_gasPrice")
            .await
            .map_err(|e| RelayerError::RpcError(e.to_string()))?;

        let nonce: U256 = self
            .client
            .request("eth_getTransactionCount", (from, "latest"))
            .await
            .map_err(|e| RelayerError::RpcError(e.to_string()))?;

        // Parse user pub key
        let stark_pub_key = withdrawal
            .stark_pub_key
            .parse::<U256>()
            .map_err(|e| RelayerError::ContractError(format!("Invalid stark pub key: {}", e)))?;

        // Parse amount
        let amount = U256::from(withdrawal.amount);

        // Parse L2 TX ID, defaulting to 0 if empty
        let l2_tx_id = if withdrawal.l2_tx_id.is_empty() {
            U256::ZERO
        } else {
            U256::from_str_radix(&withdrawal.l2_tx_id, 10)
                .map_err(|e| RelayerError::ContractError(format!("Invalid L2 TX ID: {}", e)))?
        };

        // Parse commitment hash to bytes32
        let commitment_hash_str = withdrawal
            .commitment_hash
            .strip_prefix("0x")
            .unwrap_or(&withdrawal.commitment_hash);
        let commitment_hash = hex::decode(commitment_hash_str)
            .map_err(|e| RelayerError::ContractError(format!("Invalid commitment hash: {}", e)))?;

        // Pad or truncate to 32 bytes
        let mut commitment_bytes32 = [0u8; 32];
        let len = commitment_hash.len().min(32);
        commitment_bytes32[..len].copy_from_slice(&commitment_hash[..len]);

        // Convert proof_params and proof_data from bytes to Vec<U256>
        let proof_params = self.decode_uint_array_from_bytes(&withdrawal.proof_params)?;
        let proof = self.decode_uint_array_from_bytes(&withdrawal.proof_data)?;

        // Create the function call using alloy_sol_types
        let call = unlock_funds_with_proofCall {
            proofParams: proof_params,
            proof,
            stark_pub_key,
            amount,
            l2TxId: l2_tx_id,
            commitmentHash: alloy_primitives::FixedBytes(commitment_bytes32),
        };

        let call_data = call.abi_encode();

        let tx_params = serde_json::json!({
            "from": from,
            "to": self.contract_address,
            "gas": format!("0x{:x}", self.config.gas_limit),
            "gasPrice": format!("0x{:x}", gas_price),
            "nonce": format!("0x{:x}", nonce),
            "data": format!("0x{}", hex::encode(&call_data)),
        });

        let tx_hash: String = self
            .client
            .request("eth_sendTransaction", [tx_params])
            .await
            .map_err(|e| match e {
                RpcError::ErrorResp(payload) => RelayerError::RpcError(format!(
                    "RPC error {} - {}",
                    payload.code, payload.message
                )),
                RpcError::Transport(e) => RelayerError::RpcError(format!("Transport error: {}", e)),
                _ => RelayerError::RpcError(e.to_string()),
            })?;

        self.wait_for_transaction_receipt(&tx_hash).await?;

        Ok(())
    }

    /// Decode bytes to an array of U256 integers
    fn decode_uint_array_from_bytes(&self, bytes: &[u8]) -> Result<Vec<U256>, RelayerError> {
        let mut result = Vec::new();
        let mut i = 0;

        // Simple decoding
        while i + 32 <= bytes.len() {
            let mut word = [0u8; 32];
            word.copy_from_slice(&bytes[i..i + 32]);
            result.push(U256::from_be_bytes(word));
            i += 32;
        }

        Ok(result)
    }

    /// Wait for a transaction receipt
    async fn wait_for_transaction_receipt(&self, tx_hash: &str) -> Result<(), RelayerError> {
        // Ensure tx_hash starts with 0x
        let tx_hash = if !tx_hash.starts_with("0x") {
            format!("0x{}", tx_hash)
        } else {
            tx_hash.to_string()
        };

        for _ in 0..60 {
            // Try for up to 5 minutes (60 * 5s)
            // Poll for receipt
            let receipt: Option<serde_json::Value> = self
                .client
                .request("eth_getTransactionReceipt", [&tx_hash])
                .await
                .map_err(|e| RelayerError::RpcError(e.to_string()))?;

            if let Some(receipt) = receipt {
                // Check status (1 = success, 0 = failure)
                let status = receipt["status"]
                    .as_str()
                    .unwrap_or("0x0")
                    .trim_start_matches("0x");

                let status_value = u64::from_str_radix(status, 16)
                    .map_err(|_| RelayerError::RpcError("Invalid status format".to_string()))?;

                if status_value == 1 {
                    return Ok(());
                } else {
                    return Err(RelayerError::TransactionFailed(format!(
                        "Transaction failed: {}",
                        tx_hash
                    )));
                }
            }

            sleep(Duration::from_secs(5)).await;
        }

        Err(RelayerError::TransactionFailed(format!(
            "Transaction confirmation timeout: {}",
            tx_hash
        )))
    }
}

#[cfg(test)]
mod tests {}

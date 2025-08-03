use crate::config::AppConfig;
use serde_json::Value;
use sqlx::{Pool, Postgres};
use starknet::accounts::{Account, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount};
use starknet::core::chain_id::MAINNET;
use starknet::core::types::{Call, ExecutionResult, Felt, TransactionReceipt};
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcClient};
use starknet::providers::{Provider, ProviderError};
use starknet::signers::{LocalWallet, SigningKey};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use url::Url;

#[derive(Error, Debug)]
pub enum ProofSubmissionError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    ParseError(#[from] starknet::core::types::FromStrError),

    #[error("Calldata directory not found: {0}")]
    CalldataDirNotFound(String),

    #[error("Required calldata file missing: {0}")]
    CalldataFileMissing(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Transaction timeout")]
    TransactionTimeout,

    #[error("Invalid contract address")]
    InvalidContractAddress,

    #[error("Proof job not found: {0}")]
    ProofJobNotFound(u64),

    #[error("Invalid calldata format: {0}")]
    InvalidCalldataFormat(String),
}

#[derive(Debug, Clone)]
pub struct ProofSubmissionConfig {
    pub contract_address: String,
    pub rpc_url: String,
    pub account_address: String,
    pub private_key: String,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub transaction_timeout_ms: u64,
}

impl From<AppConfig> for ProofSubmissionConfig {
    fn from(config: AppConfig) -> Self {
        Self {
            contract_address: config.starknet.contract_address.clone(),
            rpc_url: config.starknet.get_rpc_url(),
            account_address: config.starknet.account_address.clone(),
            private_key: config.starknet.private_key.clone(),
            max_retries: config.starknet.max_retries.unwrap_or(5),
            retry_delay_ms: config.starknet.retry_delay_ms.unwrap_or(5000),
            transaction_timeout_ms: config.starknet.transaction_timeout_ms.unwrap_or(300000),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProofJob {
    pub id: i64,
    pub job_id: i64,
    pub calldata_dir: String,
    pub layout: String,
    pub hasher: String,
    pub stone_version: String,
    pub memory_verification: String,
    pub status: String,
    pub current_stage: Option<String>,
    pub retry_count: i32,
    pub error_message: Option<String>,
    pub tx_hashes: Value,
}

/// Main struct for handling proof submission to Starknet
pub struct ProofSubmissionRelayer {
    db_pool: Pool<Postgres>,
    config: ProofSubmissionConfig,
    account: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
}

impl ProofSubmissionRelayer {
    /// Create a new ProofSubmissionRelayer instance
    pub async fn new(
        db_pool: Pool<Postgres>,
        config: ProofSubmissionConfig,
    ) -> Result<Self, ProofSubmissionError> {
        let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(&config.rpc_url).unwrap()));
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

    /// Main entry point for submitting proofs from a calldata directory
    pub async fn submit_proof_from_calldata(
        &self,
        calldata_dir: PathBuf,
        job_id: u64,
        layout: String,
        hasher: String,
        stone_version: String,
        memory_verification: String,
    ) -> Result<(), ProofSubmissionError> {
        info!(
            "Starting proof submission for job_id: {}, calldata_dir: {:?}",
            job_id, calldata_dir
        );

        // Validate calldata directory exists
        if !calldata_dir.exists() {
            return Err(ProofSubmissionError::CalldataDirNotFound(
                calldata_dir.display().to_string(),
            ));
        }

        // Create or get existing proof job
        let mut proof_job = self
            .create_or_get_proof_job(
                job_id,
                &calldata_dir,
                &layout,
                &hasher,
                &stone_version,
                &memory_verification,
            )
            .await?;

        info!(
            "Processing proof job {} (DB ID: {}), current status: {}",
            proof_job.job_id, proof_job.id, proof_job.status
        );

        // Resume from current stage if interrupted
        match proof_job.current_stage.as_deref() {
            None | Some("processing") => {
                self.execute_full_proof_flow(&mut proof_job).await?;
            }
            Some("initial_submitted") => {
                self.submit_step_proofs(&mut proof_job).await?;
                self.submit_final_proof(&mut proof_job).await?;
            }
            Some(stage) if stage.starts_with("step") => {
                let step_num: u32 = stage
                    .strip_prefix("step")
                    .and_then(|s| s.strip_suffix("_submitted"))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                self.submit_step_proofs_from(&mut proof_job, step_num + 1)
                    .await?;
                self.submit_final_proof(&mut proof_job).await?;
            }
            Some("final_submitted") => {
                info!("All proofs already submitted, marking as completed");
                self.mark_proof_job_completed(&mut proof_job).await?;
            }
            Some("completed") => {
                info!("Proof job already completed");
                return Ok(());
            }
            Some("failed") => {
                warn!("Proof job previously failed, retrying from beginning");
                proof_job.current_stage = Some("processing".to_string());
                proof_job.retry_count += 1;
                self.update_proof_job_stage(&mut proof_job, "processing")
                    .await?;
                self.execute_full_proof_flow(&mut proof_job).await?;
            }
            _ => {
                warn!(
                    "Unknown stage: {:?}, restarting from beginning",
                    proof_job.current_stage
                );
                self.execute_full_proof_flow(&mut proof_job).await?;
            }
        }

        info!(
            "Proof submission completed successfully for job_id: {}",
            job_id
        );
        Ok(())
    }

    /// Execute the full proof submission flow (initial -> steps -> final)
    async fn execute_full_proof_flow(
        &self,
        proof_job: &mut ProofJob,
    ) -> Result<(), ProofSubmissionError> {
        self.submit_initial_proof(proof_job).await?;
        self.submit_step_proofs(proof_job).await?;
        self.submit_final_proof(proof_job).await?;
        Ok(())
    }

    /// Submit the initial proof
    async fn submit_initial_proof(
        &self,
        proof_job: &mut ProofJob,
    ) -> Result<(), ProofSubmissionError> {
        info!("Submitting initial proof for job_id: {}", proof_job.job_id);

        let calldata_dir = PathBuf::from(&proof_job.calldata_dir);
        let initial_file = calldata_dir.join("initial");

        if !initial_file.exists() {
            return Err(ProofSubmissionError::CalldataFileMissing(
                "initial".to_string(),
            ));
        }

        let initial_calldata = self.read_calldata_file(&initial_file)?;

        // Build calldata for verify_proof_initial
        let mut calldata = vec![Felt::from(proof_job.job_id as u64)];
        calldata.push(self.string_to_felt(&proof_job.layout));
        calldata.push(self.string_to_felt(&proof_job.hasher));
        calldata.push(self.string_to_felt(&proof_job.stone_version));
        calldata.push(self.string_to_felt(&proof_job.memory_verification));
        calldata.extend(initial_calldata);

        let tx_hash = self
            .submit_contract_call("verify_proof_initial", calldata, proof_job)
            .await?;

        self.update_proof_job_stage(proof_job, "initial_submitted")
            .await?;
        self.add_tx_hash(proof_job, "initial", &tx_hash.to_string())
            .await?;

        info!(
            "Initial proof submitted successfully for job_id: {}, tx_hash: {}",
            proof_job.job_id, tx_hash
        );

        Ok(())
    }

    /// Submit all step proofs
    async fn submit_step_proofs(
        &self,
        proof_job: &mut ProofJob,
    ) -> Result<(), ProofSubmissionError> {
        self.submit_step_proofs_from(proof_job, 1).await
    }

    /// Submit step proofs starting from a specific step number
    async fn submit_step_proofs_from(
        &self,
        proof_job: &mut ProofJob,
        start_step: u32,
    ) -> Result<(), ProofSubmissionError> {
        let calldata_dir = PathBuf::from(&proof_job.calldata_dir);
        let mut step_num = start_step;

        loop {
            let step_file = calldata_dir.join(format!("step{}", step_num));

            if !step_file.exists() {
                info!(
                    "No more step files found after step{}, proceeding to final",
                    step_num - 1
                );
                break;
            }

            info!(
                "Submitting step{} proof for job_id: {}",
                step_num, proof_job.job_id
            );

            let step_calldata = self.read_calldata_file(&step_file)?;

            // Build calldata for verify_proof_step
            let mut calldata = vec![Felt::from(proof_job.job_id as u64)];
            calldata.extend(step_calldata);

            let tx_hash = self
                .submit_contract_call("verify_proof_step", calldata, proof_job)
                .await?;

            let stage_name = format!("step{}_submitted", step_num);
            self.update_proof_job_stage(proof_job, &stage_name).await?;
            self.add_tx_hash(
                proof_job,
                &format!("step{}", step_num),
                &tx_hash.to_string(),
            )
            .await?;

            info!(
                "Step{} proof submitted successfully for job_id: {}, tx_hash: {}",
                step_num, proof_job.job_id, tx_hash
            );

            step_num += 1;
        }

        Ok(())
    }

    /// Submit the final proof and register fact
    async fn submit_final_proof(
        &self,
        proof_job: &mut ProofJob,
    ) -> Result<(), ProofSubmissionError> {
        info!("Submitting final proof for job_id: {}", proof_job.job_id);

        let calldata_dir = PathBuf::from(&proof_job.calldata_dir);
        let final_file = calldata_dir.join("final");

        if !final_file.exists() {
            return Err(ProofSubmissionError::CalldataFileMissing(
                "final".to_string(),
            ));
        }

        let final_calldata = self.read_calldata_file(&final_file)?;

        // Build calldata for verify_proof_final_and_register_fact
        let mut calldata = vec![Felt::from(proof_job.job_id as u64)];
        calldata.extend(final_calldata);

        let tx_hash = self
            .submit_contract_call("verify_proof_final_and_register_fact", calldata, proof_job)
            .await?;

        self.update_proof_job_stage(proof_job, "final_submitted")
            .await?;
        self.add_tx_hash(proof_job, "final", &tx_hash.to_string())
            .await?;

        info!(
            "Final proof submitted successfully for job_id: {}, tx_hash: {}",
            proof_job.job_id, tx_hash
        );

        // Mark as completed and update deposits
        self.mark_proof_job_completed(proof_job).await?;

        Ok(())
    }

    /// Submit a contract call with retry logic
    async fn submit_contract_call(
        &self,
        function_name: &str,
        calldata: Vec<Felt>,
        proof_job: &ProofJob,
    ) -> Result<Felt, ProofSubmissionError> {
        let contract_address = Felt::from_hex(&self.config.contract_address)
            .map_err(|_| ProofSubmissionError::InvalidContractAddress)?;

        let selector = match function_name {
            "verify_proof_initial" => starknet::macros::selector!("verify_proof_initial"),
            "verify_proof_step" => starknet::macros::selector!("verify_proof_step"),
            "verify_proof_final_and_register_fact" => {
                starknet::macros::selector!("verify_proof_final_and_register_fact")
            }
            _ => {
                return Err(ProofSubmissionError::TransactionFailed(format!(
                    "Unknown function: {}",
                    function_name
                )))
            }
        };

        let call = Call {
            to: contract_address,
            selector,
            calldata,
        };

        let mut attempts = 0;
        let max_retries = self.config.max_retries;

        loop {
            attempts += 1;

            info!(
                "Submitting {} (attempt {}/{}) for job_id: {}",
                function_name, attempts, max_retries, proof_job.job_id
            );

            match self.account.execute_v3(vec![call.clone()]).send().await {
                Ok(result) => {
                    info!(
                        "Transaction submitted successfully: {} for job_id: {}, tx_hash: {}",
                        function_name, proof_job.job_id, result.transaction_hash
                    );

                    // Wait for confirmation
                    match self
                        .wait_for_transaction_confirmation(result.transaction_hash)
                        .await
                    {
                        Ok(_) => {
                            info!(
                                "Transaction confirmed: {} for job_id: {}, tx_hash: {}",
                                function_name, proof_job.job_id, result.transaction_hash
                            );
                            return Ok(result.transaction_hash);
                        }
                        Err(e) => {
                            error!(
                                "Transaction confirmation failed: {} for job_id: {}, tx_hash: {}, error: {:?}",
                                function_name, proof_job.job_id, result.transaction_hash, e
                            );

                            if attempts >= max_retries {
                                return Err(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Transaction submission failed: {} for job_id: {} (attempt {}/{}), error: {:?}",
                        function_name, proof_job.job_id, attempts, max_retries, e
                    );

                    if attempts >= max_retries {
                        return Err(ProofSubmissionError::TransactionFailed(format!(
                            "Failed after {} attempts: {}",
                            max_retries, e
                        )));
                    }
                }
            }

            // Exponential backoff
            let delay = Duration::from_millis(self.config.retry_delay_ms * attempts as u64);
            warn!(
                "Retrying {} for job_id: {} in {:?}",
                function_name, proof_job.job_id, delay
            );
            sleep(delay).await;
        }
    }

    /// Wait for transaction confirmation
    async fn wait_for_transaction_confirmation(
        &self,
        tx_hash: Felt,
    ) -> Result<(), ProofSubmissionError> {
        let timeout = Duration::from_millis(self.config.transaction_timeout_ms);
        let start_time = std::time::Instant::now();

        loop {
            if start_time.elapsed() > timeout {
                return Err(ProofSubmissionError::TransactionTimeout);
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
                                return Err(ProofSubmissionError::TransactionFailed(
                                    reason.to_string(),
                                ));
                            }
                        },
                        _ => {
                            // Other receipt types, keep polling
                        }
                    }
                }
                Err(ProviderError::StarknetError(
                    starknet::core::types::StarknetError::TransactionHashNotFound,
                )) => {
                    // Transaction not found yet, keep polling
                }
                Err(e) => return Err(ProofSubmissionError::Provider(e)),
            }

            sleep(Duration::from_secs(2)).await;
        }
    }

    /// Read calldata from a file and parse it
    fn read_calldata_file(&self, file_path: &Path) -> Result<Vec<Felt>, ProofSubmissionError> {
        let content = fs::read_to_string(file_path)?;
        let mut calldata = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Split by whitespace and parse each hex value
            for hex_str in line.split_whitespace() {
                let felt = Felt::from_hex(hex_str).map_err(|e| {
                    ProofSubmissionError::InvalidCalldataFormat(format!(
                        "Invalid hex value '{}' in file {:?}: {}",
                        hex_str, file_path, e
                    ))
                })?;
                calldata.push(felt);
            }
        }

        debug!(
            "Read {} calldata elements from {:?}",
            calldata.len(),
            file_path
        );
        Ok(calldata)
    }

    /// Convert string to Felt (hex encoding)
    fn string_to_felt(&self, input: &str) -> Felt {
        let hex_string = self.string_to_hex(input);
        Felt::from_hex(&hex_string).unwrap()
    }

    /// Convert string to hex representation
    fn string_to_hex(&self, input: &str) -> String {
        let mut hex_string = String::from("0x");
        for byte in input.bytes() {
            hex_string.push_str(&format!("{:02x}", byte));
        }
        hex_string
    }

    /// Create or get existing proof job from database
    async fn create_or_get_proof_job(
        &self,
        job_id: u64,
        calldata_dir: &Path,
        layout: &str,
        hasher: &str,
        stone_version: &str,
        memory_verification: &str,
    ) -> Result<ProofJob, ProofSubmissionError> {
        // Try to get existing job
        if let Ok(existing_job) = self.get_proof_job_by_job_id(job_id).await {
            info!("Found existing proof job for job_id: {}", job_id);
            return Ok(existing_job);
        }

        // Create new job
        info!("Creating new proof job for job_id: {}", job_id);
        let row = sqlx::query!(
            r#"
            INSERT INTO proof_jobs (job_id, calldata_dir, layout, hasher, stone_version, memory_verification, status, current_stage, tx_hashes)
            VALUES ($1, $2, $3, $4, $5, $6, 'processing', 'processing', '{}')
            RETURNING id, job_id, calldata_dir, layout, hasher, stone_version, memory_verification, status, current_stage, retry_count, error_message, tx_hashes
            "#,
            job_id as i64,
            calldata_dir.display().to_string(),
            layout,
            hasher,
            stone_version,
            memory_verification
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(ProofJob {
            id: row.id,
            job_id: row.job_id,
            calldata_dir: row.calldata_dir,
            layout: row.layout,
            hasher: row.hasher,
            stone_version: row.stone_version,
            memory_verification: row.memory_verification,
            status: row.status,
            current_stage: row.current_stage,
            retry_count: row.retry_count,
            error_message: row.error_message,
            tx_hashes: row.tx_hashes.unwrap_or_else(|| serde_json::json!({})),
        })
    }

    /// Get proof job by job_id
    async fn get_proof_job_by_job_id(&self, job_id: u64) -> Result<ProofJob, ProofSubmissionError> {
        let row = sqlx::query!(
            r#"
            SELECT id, job_id, calldata_dir, layout, hasher, stone_version, memory_verification, status, current_stage, retry_count, error_message, tx_hashes
            FROM proof_jobs
            WHERE job_id = $1
            "#,
            job_id as i64
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|_| ProofSubmissionError::ProofJobNotFound(job_id))?;

        Ok(ProofJob {
            id: row.id,
            job_id: row.job_id,
            calldata_dir: row.calldata_dir,
            layout: row.layout,
            hasher: row.hasher,
            stone_version: row.stone_version,
            memory_verification: row.memory_verification,
            status: row.status,
            current_stage: row.current_stage,
            retry_count: row.retry_count,
            error_message: row.error_message,
            tx_hashes: row.tx_hashes.unwrap_or_else(|| serde_json::json!({})),
        })
    }

    /// Update proof job stage
    async fn update_proof_job_stage(
        &self,
        proof_job: &mut ProofJob,
        stage: &str,
    ) -> Result<(), ProofSubmissionError> {
        sqlx::query!(
            r#"
            UPDATE proof_jobs
            SET current_stage = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            stage,
            proof_job.id
        )
        .execute(&self.db_pool)
        .await?;

        proof_job.current_stage = Some(stage.to_string());
        info!("Updated proof job {} stage to: {}", proof_job.job_id, stage);
        Ok(())
    }

    /// Add transaction hash to proof job
    async fn add_tx_hash(
        &self,
        proof_job: &mut ProofJob,
        stage: &str,
        tx_hash: &str,
    ) -> Result<(), ProofSubmissionError> {
        let mut tx_hashes: HashMap<String, String> =
            serde_json::from_value(proof_job.tx_hashes.clone())?;
        tx_hashes.insert(stage.to_string(), tx_hash.to_string());

        let updated_tx_hashes = serde_json::to_value(&tx_hashes)?;

        sqlx::query!(
            r#"
            UPDATE proof_jobs
            SET tx_hashes = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            updated_tx_hashes,
            proof_job.id
        )
        .execute(&self.db_pool)
        .await?;

        proof_job.tx_hashes = updated_tx_hashes;
        info!(
            "Added tx_hash for job {} stage {}: {}",
            proof_job.job_id, stage, tx_hash
        );
        Ok(())
    }

    /// Mark proof job as completed and update related deposits
    async fn mark_proof_job_completed(
        &self,
        proof_job: &mut ProofJob,
    ) -> Result<(), ProofSubmissionError> {
        info!("Marking proof job {} as completed", proof_job.job_id);

        // Update proof job status
        sqlx::query!(
            r#"
            UPDATE proof_jobs
            SET status = 'completed', current_stage = 'completed', updated_at = NOW()
            WHERE id = $1
            "#,
            proof_job.id
        )
        .execute(&self.db_pool)
        .await?;

        // Update related deposits to READY_TO_CLAIM
        let updated_deposits = sqlx::query!(
            r#"
            UPDATE deposits
            SET status = 'READY_TO_CLAIM', updated_at = NOW()
            WHERE id IN (
                SELECT DISTINCT d.id
                FROM deposits d
                WHERE d.status = 'pending'
                -- Add additional criteria to match deposits with this proof job
                -- This depends on your specific business logic
            )
            RETURNING id
            "#
        )
        .fetch_all(&self.db_pool)
        .await?;

        info!(
            "Marked {} deposits as READY_TO_CLAIM for proof job {}",
            updated_deposits.len(),
            proof_job.job_id
        );

        proof_job.status = "completed".to_string();
        proof_job.current_stage = Some("completed".to_string());

        info!(
            "Proof job {} marked as completed successfully",
            proof_job.job_id
        );
        Ok(())
    }
}

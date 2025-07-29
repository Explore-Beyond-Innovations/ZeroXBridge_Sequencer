use crate::config::AppConfig;
use crate::relayer::proof_submission::{
    ProofSubmissionConfig, ProofSubmissionError, ProofSubmissionRelayer,
};
use sqlx::{Pool, Postgres};
use std::path::PathBuf;

/// High-level client for proof submission operations
pub struct ProofSubmissionClient {
    relayer: ProofSubmissionRelayer,
}

impl ProofSubmissionClient {
    /// Create a new ProofSubmissionClient
    pub async fn new(
        db_pool: Pool<Postgres>,
        config: AppConfig,
    ) -> Result<Self, ProofSubmissionError> {
        let proof_config = ProofSubmissionConfig::from(config);
        let relayer = ProofSubmissionRelayer::new(db_pool, proof_config).await?;

        Ok(Self { relayer })
    }

    /// Submit a proof from a calldata directory
    ///
    /// This is the main entry point for proof submission. It handles:
    /// - Reading calldata files from the specified directory
    /// - Submitting proofs in the correct order (initial -> steps -> final)
    /// - Updating database records at each stage
    /// - Retrying failed transactions with exponential backoff
    /// - Resuming from interruptions
    pub async fn submit_proof(
        &self,
        calldata_dir: PathBuf,
        job_id: u64,
        layout: String,
        hasher: String,
        stone_version: String,
        memory_verification: String,
    ) -> Result<(), ProofSubmissionError> {
        self.relayer
            .submit_proof_from_calldata(
                calldata_dir,
                job_id,
                layout,
                hasher,
                stone_version,
                memory_verification,
            )
            .await
    }

    /// Get the underlying relayer instance (for advanced usage)
    pub fn relayer(&self) -> &ProofSubmissionRelayer {
        &self.relayer
    }
}

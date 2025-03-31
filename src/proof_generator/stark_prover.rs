use crate::config::MerkleConfig;
use sqlx::PgPool;
use stwo::cairo::stark::{
    ProverParameters,
    StarkProver,
    ProofOptions
};


#[derive(Debug, thiserror::Error)]
enum ProofGenerationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Proof generation failed")]
    GenerationError,
}

pub struct StarkProver {
    db_pool: PgPool,
    merkle_tree_data: MerkleConfig,
    commitment_hash: String,
}

impl StarkProver {
    /// Create a new StarkProver instance
    /// @params db_pool reference to a PgPool instance (PgPool)
    /// @params merkle_tree_data reference to a pre-validated merkle tree data (MerkleConfig)
    /// @params commitment_hash the commitment hash of the transaction (string)
    /// returns Self The instance of StarkProver
    pub fn new(db_pool: PgPool, merkle_tree_data: MerkleConfig, commitment_hash: &str) -> Self {
        Self {
            db_pool,
            merkle_tree_data,
            commitment_hash: commitment_hash.to_owned(),
        }
    }
    /// Generate the STARK Proof
    /// return String The stark proof
    pub fn generate_proof(&self) -> String {
        // Proof logic here
        "proof".to_owned()
    }

    /// Saves the STARK Proof and the metadata to DB
    /// @params merkle_tree_root The root of the merkle tree data
    /// @params proof The generated proof (string)
    /// @params commitment_hash The pre-validated commitment hash of the transaction
    /// returns Result<(), ProofGenerationError>
    async fn save_proof_to_db(
        &self,
        merkle_tree_root: &str,
        proof: &str,
        commitment_hash: &str,
    ) -> Result<(), ProofGenerationError> {
        sqlx::query!(
            r#"
            INSERT INTO stark_proofs (merkle_tree_root, commitment_hash, proof)
            VALUES ($1, $2, $3)
            "#,
            merkle_tree_root,
            commitment_hash,
            proof
        )
        .execute(self.db_pool)
        .await
        .map_err(|e| ProofGenerationError::DatabaseError(e))?;

        // Log the proof save success
        tracing::debug!("STARK proof successfully saved to DB");
        Ok(())
    }

    /// Runs the StarkProver application
    /// generates the proof and save it to db
    async fn run(&self) {
        let proof = self.generate_proof();
        // save the data to database
        self.save_proof_to_db(self.merkle_tree_root, &proof, &self.commitment_hash)
            .await.unwrap();
    }
}

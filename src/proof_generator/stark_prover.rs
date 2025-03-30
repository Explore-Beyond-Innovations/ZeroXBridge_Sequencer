use sqlx::PgPool;
use stwo;
use crate::config::MerkleConfig;

#[derive(Debug, thiserror::Error )]
enum ProofGenerationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Proof generation failed")]
    GenerationError,
}

pub struct StarkProver {
    db_pool: PgPool,
    merkle_tree_data: MerkleConfig,
    commitment_hash: String
}

impl StarkProver {
    pub fn new(db_pool: PgPool, merkle_tree_data: MerkleConfig, commitment_hash: &str) -> Self {
        Self {
            db_pool,
            merkle_tree_data,
            commitment_hash: commitment_hash.to_owned(),
        }
    }

    pub fn generate_proof(&self) -> String {
        
    }

    async  fn save_proof_to_db(&self, merkle_tree_root: &str, commitment_hash: &str) -> Result<(), ProofGenerationError>{
        sqlx::query!(
            r#"
            INSERT INTO stark_proofs (merkle_tree_root, commitment_has, updated_at)
            VALUES ($1, $2, NOW())
            "#,
            merkle_tree_root,
            commitment_hash,
        )
        .execute(self.db_pool)
        .await
        .map_err(|e| ProofGenerationError::DatabaseError(e))?;

        Ok(())
    }

}
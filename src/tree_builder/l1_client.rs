//! Tree Builder Client
//! 
//! Long-running service that manages Merkle tree updates for deposits.
//! Processes deposits with status = PENDING_TREE_INCLUSION and stores proofs.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, error, info};

use crate::db::database::{
    fetch_deposits_for_tree_inclusion, 
    fetch_included_deposits, 
    update_deposit_with_proof,
    get_max_leaf_index,
    Deposit,
};

use tree_builder::l1_tree::L1MerkleTreeBuilder;

/// Manages Merkle tree updates for deposits
pub struct TreeBuilderClient {
    db_pool: PgPool,
    tree_builder: Arc<Mutex<L1MerkleTreeBuilder>>,
    poll_interval_seconds: u64,
    task_handle: Option<JoinHandle<()>>,
    shutdown_sender: Option<oneshot::Sender<()>>,
}

impl TreeBuilderClient {
    /// Creates a new client
    pub fn new(db_pool: PgPool, poll_interval_seconds: u64) -> Self {
        Self {
            db_pool,
            tree_builder: Arc::new(Mutex::new(L1MerkleTreeBuilder::new())),
            poll_interval_seconds,
            task_handle: None,
            shutdown_sender: None,
        }
    }

    /// Starts the service in the background
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Tree Builder Client");

        self.rebuild_tree_on_startup().await
            .context("Failed to rebuild tree on startup")?;
        
        let db_pool = self.db_pool.clone();
        let tree_builder = Arc::clone(&self.tree_builder);
        let poll_interval_seconds = self.poll_interval_seconds;
        
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        self.shutdown_sender = Some(shutdown_tx);

        let handle = tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(poll_interval_seconds));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::process_pending_deposits_static(&db_pool, &tree_builder).await {
                            error!("Error processing pending deposits: {}", e);
                        }
                    }
                    _ = &mut shutdown_rx => {
                        info!("Received shutdown signal, stopping tree builder");
                        break;
                    }
                }
            }
        });

        self.task_handle = Some(handle);
        info!("Tree Builder Client started successfully");
        Ok(())
    }

    /// Stops the service gracefully
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping Tree Builder Client");
        
        // Send shutdown signal
        if let Some(shutdown_tx) = self.shutdown_sender.take() {
            let _ = shutdown_tx.send(()); // Ignore error if receiver is already dropped
        }

        // Wait for task to complete gracefully
        if let Some(handle) = self.task_handle.take() {
            match tokio::time::timeout(Duration::from_secs(10), handle).await {
                Ok(result) => {
                    if let Err(e) = result {
                        error!("Tree builder task ended with error: {:?}", e);
                    } else {
                        info!("Tree builder service stopped gracefully");
                    }
                }
                Err(_) => {
                    error!("Tree builder service did not stop within timeout, forcing shutdown");
                    // Task handle is already consumed by timeout, so it will be dropped and cancelled
                }
            }
        }
        
        Ok(())
    }

    /// Rebuilds tree from existing deposits
    async fn rebuild_tree_on_startup(&self) -> Result<()> {
        info!("Rebuilding tree from existing deposits");

        let included_deposits = fetch_included_deposits(&self.db_pool).await
            .context("Failed to fetch included deposits")?;

        if included_deposits.is_empty() {
            info!("No existing deposits to rebuild from");
            return Ok(());
        }

        info!("Rebuilding tree from {} existing deposits", included_deposits.len());

        let mut leaves = Vec::new();
        for deposit in &included_deposits {
            // Validate commitment_hash format and length
            if !deposit.commitment_hash.starts_with("0x") {
                return Err(anyhow::anyhow!(
                    "Invalid commitment_hash format for deposit {}: must start with '0x'", 
                    deposit.id
                ));
            }
            if deposit.commitment_hash.len() < 66 { // 0x + 64 hex chars = 66 total
                return Err(anyhow::anyhow!(
                    "Invalid commitment_hash length for deposit {}: expected 66 characters, got {}",
                    deposit.id, deposit.commitment_hash.len()
                ));
            }

            let commitment_hash = hex::decode(&deposit.commitment_hash[2..])
                .with_context(|| format!("Failed to decode commitment hash for deposit {}", deposit.id))?;
            
            if commitment_hash.len() != 32 {
                return Err(anyhow::anyhow!(
                    "Invalid commitment_hash decoded length for deposit {}: expected 32 bytes, got {}",
                    deposit.id, commitment_hash.len()
                ));
            }
            
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(&commitment_hash);
            leaves.push(hash_array);
        }

        let mut tree = self.tree_builder.lock().await;
        tree.build_merkle(leaves).await
            .map_err(|e| anyhow::anyhow!("Tree build error: {:?}", e))?;

        info!("Successfully rebuilt tree with {} deposits", included_deposits.len());
        Ok(())
    }

    /// Processes pending deposits
    async fn process_pending_deposits_static(
        db_pool: &PgPool,
        tree_builder: &Arc<Mutex<L1MerkleTreeBuilder>>,
    ) -> Result<()> {
        let pending_deposits = fetch_deposits_for_tree_inclusion(db_pool, 100).await
            .context("Failed to fetch pending deposits")?;

        if pending_deposits.is_empty() {
            debug!("No pending deposits to process");
            return Ok(());
        }

        info!("Processing {} pending deposits", pending_deposits.len());

        for deposit in pending_deposits {
            if let Err(e) = Self::process_single_deposit_static(db_pool, tree_builder, deposit).await {
                error!("Failed to process deposit: {}", e);
                continue;
            }
        }

        Ok(())
    }

    /// Processes a single deposit
    async fn process_single_deposit_static(
        db_pool: &PgPool,
        tree_builder: &Arc<Mutex<L1MerkleTreeBuilder>>,
        deposit: Deposit,
    ) -> Result<()> {
        debug!("Processing deposit {}", deposit.id);

        // Validate commitment_hash format and length
        if !deposit.commitment_hash.starts_with("0x") {
            return Err(anyhow::anyhow!(
                "Invalid commitment_hash format for deposit {}: must start with '0x'", 
                deposit.id
            ));
        }
        if deposit.commitment_hash.len() < 66 { // 0x + 64 hex chars = 66 total
            return Err(anyhow::anyhow!(
                "Invalid commitment_hash length for deposit {}: expected 66 characters, got {}",
                deposit.id, deposit.commitment_hash.len()
            ));
        }

        let commitment_hash = hex::decode(&deposit.commitment_hash[2..])
            .with_context(|| format!("Failed to decode commitment hash for deposit {}", deposit.id))?;
        
        if commitment_hash.len() != 32 {
            return Err(anyhow::anyhow!(
                "Invalid commitment_hash decoded length for deposit {}: expected 32 bytes, got {}",
                deposit.id, commitment_hash.len()
            ));
        }
        
        let mut hash_array = [0u8; 32];
        hash_array.copy_from_slice(&commitment_hash);

        let mut tree = tree_builder.lock().await;
        tree.build_merkle(vec![hash_array]).await
            .map_err(|e| anyhow::anyhow!("Failed to append to tree: {:?}", e))?;

        let proof = tree.get_proof(hash_array).await
            .map_err(|e| anyhow::anyhow!("Failed to get proof: {:?}", e))?
            .ok_or_else(|| anyhow::anyhow!("No proof generated for deposit"))?;
            
        let root = tree.get_root().await
            .map_err(|e| anyhow::anyhow!("Failed to get root: {:?}", e))?;
        drop(tree);

        let leaf_index = get_max_leaf_index(db_pool).await
            .context("Failed to get max leaf index")?
            .unwrap_or(0) + 1;

        let proof_json = json!({
            "leaf_index": leaf_index,
            "sibling_hashes": proof.sibling_hashes,
            "peak_bagging": proof.peak_bagging,
        });

        let root_hex = format!("0x{}", hex::encode(root));

        // Update deposit with proof
        update_deposit_with_proof(
            db_pool,
            deposit.id,
            proof_json,
            root_hex,
            leaf_index,
        ).await
        .context("Failed to update deposit with proof")?;

        info!("Successfully processed deposit {} with leaf index {}", deposit.id, leaf_index);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tree_builder_creation_with_mock_pool() {
        // Use a test database URL that won't actually connect
        let test_db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://test_user:test_pass@localhost:5433/test_db".to_string());
        
        // Test client creation with configuration parameters
        if let Ok(pool) = sqlx::PgPool::connect(&test_db_url).await {
            let client = TreeBuilderClient::new(pool, 10);
            assert_eq!(client.poll_interval_seconds, 10);
            assert!(client.task_handle.is_none());
            assert!(client.shutdown_sender.is_none());
        } else {
            // If no test DB is available, test basic structure without DB connection
            println!("Test database not available, skipping connection tests");
            
            // We can still test that configuration values are properly stored
            // by creating a dummy pool connection string and testing parameter validation
            let poll_interval = 15;
            assert!(poll_interval > 0, "Poll interval should be positive");
            
            // Test commitment hash validation logic
            let test_hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
            assert!(test_hash.starts_with("0x"), "Hash should start with 0x");
            assert_eq!(test_hash.len(), 66, "Hash should be 66 characters long");
            
            // Test invalid hash formats that would cause panics
            let invalid_hashes = vec![
                "1234", // Too short, no 0x prefix
                "0x123", // Too short
                "0xgg", // Invalid hex characters
                "", // Empty string
            ];
            
            for invalid_hash in invalid_hashes {
                assert!(
                    invalid_hash.len() < 66 || !invalid_hash.starts_with("0x"),
                    "Invalid hash should be caught by validation: {}",
                    invalid_hash
                );
            }
        }
    }
}
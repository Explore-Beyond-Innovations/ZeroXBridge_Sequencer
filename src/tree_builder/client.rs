//! Tree Builder Client
//!
//! This module implements a long-running service that uses the existing tree-builder crate
//! to incrementally update our Merkle tree for new deposits.
//!
//! The service periodically queries the database for deposits with status = PENDING_TREE_INCLUSION,
//! processes them by appending their commitment_hash to the tree-builder, and stores the
//! resulting proofs in the database.
//!
//! On startup, it rebuilds the in-memory tree from all deposits already marked INCLUDED
//! to ensure correct state.

use std::time::Duration;
use anyhow::{Context, Result};
use serde_json::json;
use sqlx::PgPool;
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

/// Tree Builder Client that manages Merkle tree updates for deposits
pub struct TreeBuilderClient {
    db_pool: PgPool,
    tree_builder: L1MerkleTreeBuilder,
    poll_interval_seconds: u64,
}

impl TreeBuilderClient {
    /// Create a new TreeBuilderClient
    pub fn new(db_pool: PgPool, poll_interval_seconds: u64) -> Self {
        Self {
            db_pool,
            tree_builder: L1MerkleTreeBuilder::new(),
            poll_interval_seconds,
        }
    }

    /// Start the tree builder client service
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Tree Builder Client");

        // Rebuild tree from existing deposits on startup
        self.rebuild_tree_on_startup().await
            .context("Failed to rebuild tree on startup")?;

        // Start the background processing loop
        let mut interval = time::interval(Duration::from_secs(self.poll_interval_seconds));

        loop {
            interval.tick().await;
            
            if let Err(e) = self.process_pending_deposits().await {
                error!("Error processing pending deposits: {}", e);
                // Continue processing even if there's an error
            }
        }
    }

    /// Rebuild the in-memory tree from existing included deposits
    async fn rebuild_tree_on_startup(&mut self) -> Result<()> {
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
            let commitment_hash = hex::decode(&deposit.commitment_hash[2..])
                .context("Failed to decode commitment hash")?;
            
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(&commitment_hash[..32]);
            leaves.push(hash_array);
        }

        self.tree_builder.build_merkle(leaves).await
            .map_err(|e| anyhow::anyhow!("Tree build error: {:?}", e))?;

        info!("Successfully rebuilt tree with {} deposits", included_deposits.len());
        Ok(())
    }

    /// Process pending deposits
    async fn process_pending_deposits(&mut self) -> Result<()> {
        let pending_deposits = fetch_deposits_for_tree_inclusion(&self.db_pool, 100).await
            .context("Failed to fetch pending deposits")?;

        if pending_deposits.is_empty() {
            debug!("No pending deposits to process");
            return Ok(());
        }

        info!("Processing {} pending deposits", pending_deposits.len());

        for deposit in pending_deposits {
            if let Err(e) = self.process_single_deposit(deposit).await {
                error!("Failed to process deposit: {}", e);
                // Continue processing other deposits even if one fails
                continue;
            }
        }

        Ok(())
    }

    /// Process a single deposit
    async fn process_single_deposit(&mut self, deposit: Deposit) -> Result<()> {
        debug!("Processing deposit {}", deposit.id);

        // Read commitment hash
        let commitment_hash = hex::decode(&deposit.commitment_hash[2..])
            .context("Failed to decode commitment hash")?;
        
        let mut hash_array = [0u8; 32];
        hash_array.copy_from_slice(&commitment_hash[..32]);

        // Append to tree-builder
        self.tree_builder.build_merkle(vec![hash_array]).await
            .map_err(|e| anyhow::anyhow!("Failed to append to tree: {:?}", e))?;

        // Get proof and root
        let proof = self.tree_builder.get_proof(hash_array).await
            .map_err(|e| anyhow::anyhow!("Failed to get proof: {:?}", e))?
            .ok_or_else(|| anyhow::anyhow!("No proof generated for deposit"))?;
            
        let root = self.tree_builder.get_root().await
            .map_err(|e| anyhow::anyhow!("Failed to get root: {:?}", e))?;

        // Get next leaf index
        let leaf_index = get_max_leaf_index(&self.db_pool).await
            .context("Failed to get max leaf index")?
            .unwrap_or(0) + 1;

        // Store proof (sibling paths) in deposit record
        let proof_json = json!({
            "leaf_index": leaf_index,
            "sibling_hashes": proof.sibling_hashes,
            "peak_bagging": proof.peak_bagging,
        });

        let root_hex = format!("0x{}", hex::encode(root));

        // Update deposit with proof and set included = true, status = PENDING_PROOF_GENERATION
        update_deposit_with_proof(
            &self.db_pool,
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
    async fn test_tree_builder_creation() {
        // Mock database pool for testing
        let db_url = "postgresql://test:test@localhost/test";
        if let Ok(pool) = sqlx::PgPool::connect(db_url).await {
            let client = TreeBuilderClient::new(pool, 10);
            assert_eq!(client.poll_interval_seconds, 10);
        }
    }
}
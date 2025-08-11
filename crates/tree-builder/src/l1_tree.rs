use std::collections::HashMap;
use sha3::{Digest, Keccak256};
use serde::{Serialize, Deserialize};

use crate::types::Result;

/// Merkle proof containing sibling hashes and path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    pub leaf_index: usize,
    pub sibling_hashes: Vec<[u8; 32]>,
    pub peak_bagging: Vec<[u8; 32]>,
}

/// Simple Merkle tree implementation for L1 deposits
pub struct L1MerkleTreeBuilder {
    leaves: Vec<[u8; 32]>,
    tree_cache: HashMap<usize, [u8; 32]>,
}

impl L1MerkleTreeBuilder {
    /// Creates a new MerkleTreeBuilder instance
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            tree_cache: HashMap::new(),
        }
    }

    /// Hash two nodes together using Keccak256
    fn hash_pair(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
        let mut hasher = Keccak256::new();
        hasher.update(&left);
        hasher.update(&right);
        hasher.finalize().into()
    }

    /// Hash a single node (for odd number of nodes)
    fn hash_single(node: [u8; 32]) -> [u8; 32] {
        let mut hasher = Keccak256::new();
        hasher.update(&node);
        hasher.finalize().into()
    }

    /// Builds/appends to the Merkle tree from a list of commitment hashes
    pub async fn build_merkle(&mut self, leaves: Vec<[u8; 32]>) -> Result<()> {
        for leaf in leaves {
            self.leaves.push(leaf);
        }
        // Clear cache when new leaves are added
        self.tree_cache.clear();
        Ok(())
    }

    /// Gets the current Merkle root
    pub async fn get_root(&self) -> Result<[u8; 32]> {
        if self.leaves.is_empty() {
            return Ok([0u8; 32]);
        }

        if self.leaves.len() == 1 {
            return Ok(self.leaves[0]);
        }

        let mut current_level = self.leaves.clone();
        
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for i in (0..current_level.len()).step_by(2) {
                if i + 1 < current_level.len() {
                    // Pair exists, hash them together
                    let hash = Self::hash_pair(current_level[i], current_level[i + 1]);
                    next_level.push(hash);
                } else {
                    // Odd number, hash single node
                    let hash = Self::hash_single(current_level[i]);
                    next_level.push(hash);
                }
            }
            
            current_level = next_level;
        }

        Ok(current_level[0])
    }

    /// Generates a Merkle proof for a given leaf
    pub async fn get_proof(&self, leaf: [u8; 32]) -> Result<Option<Proof>> {
        // Find leaf index
        let leaf_index = self.leaves.iter().position(|&x| x == leaf);
        
        if let Some(index) = leaf_index {
            let proof = self.generate_proof(index).await?;
            Ok(Some(proof))
        } else {
            Ok(None)
        }
    }

    /// Generate proof for a leaf at given index
    async fn generate_proof(&self, leaf_index: usize) -> Result<Proof> {
        let mut sibling_hashes = Vec::new();
        let mut current_level = self.leaves.clone();
        let mut current_index = leaf_index;
        
        while current_level.len() > 1 {
            // Find sibling
            let sibling_index = if current_index % 2 == 0 {
                // Current is left child, sibling is right
                if current_index + 1 < current_level.len() {
                    Some(current_index + 1)
                } else {
                    None // No sibling for this node
                }
            } else {
                // Current is right child, sibling is left
                Some(current_index - 1)
            };

            if let Some(sibling_idx) = sibling_index {
                sibling_hashes.push(current_level[sibling_idx]);
            }

            // Move to next level
            let mut next_level = Vec::new();
            for i in (0..current_level.len()).step_by(2) {
                if i + 1 < current_level.len() {
                    let hash = Self::hash_pair(current_level[i], current_level[i + 1]);
                    next_level.push(hash);
                } else {
                    let hash = Self::hash_single(current_level[i]);
                    next_level.push(hash);
                }
            }
            
            current_level = next_level;
            current_index = current_index / 2;
        }

        Ok(Proof {
            leaf_index,
            sibling_hashes,
            peak_bagging: vec![], // Not needed for simple Merkle tree
        })
    }

    /// Verifies a Merkle proof for a given leaf
    pub async fn verify_proof(&self, proof: Proof, leaf: [u8; 32]) -> Result<bool> {
        let root = self.get_root().await?;
        
        let mut current_hash = leaf;
        let mut current_index = proof.leaf_index;
        
        for sibling_hash in proof.sibling_hashes {
            if current_index % 2 == 0 {
                // Current is left child
                current_hash = Self::hash_pair(current_hash, sibling_hash);
            } else {
                // Current is right child
                current_hash = Self::hash_pair(sibling_hash, current_hash);
            }
            current_index = current_index / 2;
        }
        
        Ok(current_hash == root)
    }
}

impl Default for L1MerkleTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_tree_operations() -> Result<()> {
        let mut builder = L1MerkleTreeBuilder::new();

        // Test single leaf
        let leaf1 = [1u8; 32];
        builder.build_merkle(vec![leaf1]).await?;

        // Get proof for leaf1
        let proof1 = builder.get_proof(leaf1).await?;
        assert!(proof1.is_some(), "Should generate proof for existing leaf");
        assert!(
            builder.verify_proof(proof1.unwrap(), leaf1).await?,
            "Proof should be valid"
        );

        // Add second leaf
        let leaf2 = [2u8; 32];
        builder.build_merkle(vec![leaf2]).await?;

        // Verify both leaves have valid proofs
        for leaf in [leaf1, leaf2] {
            let proof = builder.get_proof(leaf).await?.unwrap();
            assert!(
                builder.verify_proof(proof, leaf).await?,
                "Proof should be valid for leaf {:?}",
                leaf
            );
        }

        // Test non-existent leaf
        let fake_leaf = [99u8; 32];
        assert!(
            builder.get_proof(fake_leaf).await?.is_none(),
            "Should not find proof for non-existent leaf"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_tree() -> Result<()> {
        let builder = L1MerkleTreeBuilder::new();
        let root = builder.get_root().await?;
        assert_eq!(root, [0u8; 32]);
        Ok(())
    }

    #[tokio::test]
    async fn test_single_leaf_tree() -> Result<()> {
        let mut builder = L1MerkleTreeBuilder::new();
        let leaf = [42u8; 32];
        builder.build_merkle(vec![leaf]).await?;
        
        let root = builder.get_root().await?;
        assert_eq!(root, leaf);
        Ok(())
    }
}

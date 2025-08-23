use sha3::{Digest, Keccak256};
use crate::{types::Result, l1_tree::Proof};

/// Simple L2 Merkle tree implementation (placeholder - uses same logic as L1 for now)
pub struct L2MerkleTreeBuilder {
    leaves: Vec<[u8; 32]>,
}

impl L2MerkleTreeBuilder {
    /// Creates a new L2MerkleTreeBuilder instance
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
        }
    }

    /// Hash two nodes together using Keccak256 (same as L1)
    fn hash_pair(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
        let mut hasher = Keccak256::new();
        hasher.update(&left);
        hasher.update(&right);
        hasher.finalize().into()
    }

    /// Builds/appends to the Merkle tree from a list of commitment hashes
    pub async fn build_merkle(&mut self, leaves: Vec<[u8; 32]>) -> Result<()> {
        for leaf in leaves {
            self.leaves.push(leaf);
        }
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
            // If odd number of nodes, duplicate the last one (standard Merkle behavior)
            if current_level.len() % 2 == 1 {
                let last_node = current_level[current_level.len() - 1];
                current_level.push(last_node);
            }
            
            let mut next_level = Vec::new();
            for i in (0..current_level.len()).step_by(2) {
                let hash = Self::hash_pair(current_level[i], current_level[i + 1]);
                next_level.push(hash);
            }
            
            current_level = next_level;
        }

        Ok(current_level[0])
    }

    /// Generates a Merkle proof for a given leaf
    pub async fn get_proof(&self, leaf: [u8; 32]) -> Result<Option<Proof>> {
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
        if leaf_index >= self.leaves.len() {
            return Err(crate::error::TreeBuilderError::InvalidIndex.into());
        }
        
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
            
            // If odd number of nodes, duplicate the last one (standard Merkle behavior)
            if current_level.len() % 2 == 1 {
                let last_node = current_level[current_level.len() - 1];
                current_level.push(last_node);
            }
            
            for i in (0..current_level.len()).step_by(2) {
                let hash = Self::hash_pair(current_level[i], current_level[i + 1]);
                next_level.push(hash);
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

impl Default for L2MerkleTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
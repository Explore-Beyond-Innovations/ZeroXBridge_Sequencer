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
        // Simplified - just return first leaf for now
        Ok(self.leaves[0])
    }

    /// Generates a Merkle proof for a given leaf (simplified)
    pub async fn get_proof(&self, leaf: [u8; 32]) -> Result<Option<Proof>> {
        let leaf_index = self.leaves.iter().position(|&x| x == leaf);
        
        if let Some(index) = leaf_index {
            Ok(Some(Proof {
                leaf_index: index,
                sibling_hashes: vec![],
                peak_bagging: vec![],
            }))
        } else {
            Ok(None)
        }
    }

    /// Verifies a Merkle proof for a given leaf (simplified)
    pub async fn verify_proof(&self, _proof: Proof, _leaf: [u8; 32]) -> Result<bool> {
        Ok(true) // Simplified for now
    }
}

impl Default for L2MerkleTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
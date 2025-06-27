use crate::error::{Result, TreeBuilderError};
use crate::l2::poseidon::PoseidonHasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Merkle proof for a specific leaf
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MerkleProof {
    /// The leaf hash being proven
    pub leaf: [u8; 32],
    /// The sibling hashes for the proof path
    pub siblings: Vec<[u8; 32]>,
    /// The root hash this proof verifies against
    pub root: [u8; 32],
    /// The index of the leaf in the tree
    pub index: usize,
}

impl MerkleProof {
    /// Verify this proof against the expected root
    pub fn verify(&self) -> Result<bool> {
        let computed_root = self.compute_root()?;
        Ok(computed_root == self.root)
    }

    /// Compute the root hash from this proof
    pub fn compute_root(&self) -> Result<[u8; 32]> {
        let mut current_hash = self.leaf;
        let mut index = self.index;

        for sibling in &self.siblings {
            if index % 2 == 0 {
                // Current node is left child
                current_hash = PoseidonHasher::hash_pair(&current_hash, sibling)?;
            } else {
                // Current node is right child
                current_hash = PoseidonHasher::hash_pair(sibling, &current_hash)?;
            }
            index /= 2;
        }

        Ok(current_hash)
    }
}

/// L2 Merkle Tree using Poseidon hashing
#[derive(Debug, Clone)]
pub struct L2MerkleTree {
    /// All leaves in the tree
    leaves: Vec<[u8; 32]>,
    /// Internal nodes of the tree (level -> nodes)
    nodes: HashMap<usize, Vec<[u8; 32]>>,
    /// The root hash
    root: Option<[u8; 32]>,
    /// Leaf index mapping for quick lookups
    leaf_indices: HashMap<[u8; 32], usize>,
}

impl L2MerkleTree {
    /// Create a new empty Merkle tree
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            nodes: HashMap::new(),
            root: None,
            leaf_indices: HashMap::new(),
        }
    }

    /// Build the Merkle tree from commitment hashes
    pub fn build_l2_merkle(leaves: Vec<[u8; 32]>) -> Result<Self> {
        if leaves.is_empty() {
            return Err(TreeBuilderError::EmptyLeaves);
        }

        let mut tree = Self::new();
        tree.leaves = leaves.clone();

        // Build leaf indices mapping
        for (index, leaf) in leaves.iter().enumerate() {
            tree.leaf_indices.insert(*leaf, index);
        }

        // Build the tree bottom-up
        tree.build_tree()?;

        Ok(tree)
    }

    /// Build the internal tree structure
    fn build_tree(&mut self) -> Result<()> {
        let mut current_level = self.leaves.clone();
        let mut level = 0;

        // Store the leaf level
        self.nodes.insert(level, current_level.clone());

        // Build tree level by level until we reach the root
        while current_level.len() > 1 {
            level += 1;
            let mut next_level = Vec::new();

            // Process pairs of nodes
            for chunk in current_level.chunks(2) {
                let hash = if chunk.len() == 2 {
                    // Hash pair of nodes
                    PoseidonHasher::hash_pair(&chunk[0], &chunk[1])?
                } else {
                    // Odd number of nodes, hash single node with itself (or use a different strategy)
                    PoseidonHasher::hash_pair(&chunk[0], &chunk[0])?
                };
                next_level.push(hash);
            }

            self.nodes.insert(level, next_level.clone());
            current_level = next_level;
        }

        // Set the root
        if let Some(root_level) = self.nodes.get(&level) {
            if let Some(root) = root_level.first() {
                self.root = Some(*root);
            }
        }

        Ok(())
    }

    /// Get the root hash of the tree
    pub fn get_root(&self) -> Option<[u8; 32]> {
        self.root
    }

    /// Get a Merkle proof for a specific leaf
    pub fn get_proof(&self, leaf: [u8; 32]) -> Result<MerkleProof> {
        let index = self
            .leaf_indices
            .get(&leaf)
            .ok_or_else(|| TreeBuilderError::LeafNotFound(hex::encode(leaf)))?;

        let root = self.root.ok_or(TreeBuilderError::TreeNotBuilt)?;

        let siblings = self.compute_proof_siblings(*index)?;

        Ok(MerkleProof {
            leaf,
            siblings,
            root,
            index: *index,
        })
    }

    /// Compute the sibling hashes for a proof path
    fn compute_proof_siblings(&self, mut index: usize) -> Result<Vec<[u8; 32]>> {
        let mut siblings = Vec::new();
        let mut level = 0;

        while let Some(level_nodes) = self.nodes.get(&level) {
            if level_nodes.len() <= 1 {
                break; // Reached root
            }

            let sibling_index = if index % 2 == 0 {
                index + 1 // Right sibling
            } else {
                index - 1 // Left sibling
            };

            if sibling_index < level_nodes.len() {
                siblings.push(level_nodes[sibling_index]);
            } else {
                // No sibling (odd number of nodes), use the node itself
                siblings.push(level_nodes[index]);
            }

            index /= 2;
            level += 1;
        }

        Ok(siblings)
    }

    /// Get all leaves in the tree
    pub fn get_leaves(&self) -> &Vec<[u8; 32]> {
        &self.leaves
    }

    /// Get the number of leaves
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Check if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }
}

impl Default for L2MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for working with commitment hashes
pub mod utils {
    use super::*;

    /// Convert hex string to 32-byte array
    pub fn hex_to_bytes32(hex_str: &str) -> Result<[u8; 32]> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(hex_str)?;

        if bytes.len() != 32 {
            return Err(TreeBuilderError::InvalidLeafHash(format!(
                "Expected 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(array)
    }

    /// Convert 32-byte array to hex string
    pub fn bytes32_to_hex(bytes: &[u8; 32]) -> String {
        format!("0x{}", hex::encode(bytes))
    }

    /// Parse commitment hashes from string array
    pub fn parse_commitment_hashes(hex_strings: &[String]) -> Result<Vec<[u8; 32]>> {
        hex_strings.iter().map(|s| hex_to_bytes32(s)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::utils::*;
    use super::*;

    #[test]
    fn test_build_simple_tree() {
        let leaves = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];

        let tree = L2MerkleTree::build_l2_merkle(leaves.clone()).unwrap();
        assert_eq!(tree.len(), 4);
        assert!(tree.get_root().is_some());
    }

    #[test]
    fn test_get_proof_and_verify() {
        let leaves = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let tree = L2MerkleTree::build_l2_merkle(leaves.clone()).unwrap();

        let proof = tree.get_proof([1u8; 32]).unwrap();
        assert_eq!(proof.leaf, [1u8; 32]);
        assert_eq!(proof.index, 0);
        assert!(proof.verify().unwrap());
    }

    #[test]
    fn test_parse_commitment_hashes() {
        let hex_strings = vec![
            "0x1111111111111111111111111111111111111111111111111111111111111111".to_string(),
            "0x2222222222222222222222222222222222222222222222222222222222222222".to_string(),
        ];

        let hashes = parse_commitment_hashes(&hex_strings).unwrap();
        let tree = L2MerkleTree::build_l2_merkle(hashes.clone()).unwrap();

        for hash in &hashes {
            let proof = tree.get_proof(*hash).unwrap();
            assert!(proof.verify().unwrap());
        }
    }

    #[test]
    fn test_empty_leaves_error() {
        let result = L2MerkleTree::build_l2_merkle(vec![]);
        assert!(matches!(result, Err(TreeBuilderError::EmptyLeaves)));
    }

    #[test]
    fn test_leaf_not_found_error() {
        let leaves = vec![[1u8; 32], [2u8; 32]];
        let tree = L2MerkleTree::build_l2_merkle(leaves).unwrap();

        let result = tree.get_proof([99u8; 32]);
        assert!(matches!(result, Err(TreeBuilderError::LeafNotFound(_))));
    }
}

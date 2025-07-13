use std::{array::TryFromSliceError, sync::Arc};

use accumulators::{
    hasher::stark_poseidon,
    mmr::{MMRError, Proof, MMR},
    store::{memory::InMemoryStore, InStoreTableError, StoreError},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TreeBuilderError {
    #[error(transparent)]
    MMRError(#[from] MMRError),
    #[error(transparent)]
    StoreError(#[from] StoreError),
    #[error(transparent)]
    TableError(#[from] InStoreTableError),
    #[error("Failed to decode hex: {0}")]
    HexError(String),
    #[error("Failed to convert to array: {0}")]
    ConversionError(String),
    #[error("Invalid leaf hash: {0}")]
    InvalidLeafHash(String),
    #[error(transparent)]
    FromHexError(#[from] hex::FromHexError),
}

pub type Result<T> = std::result::Result<T, TreeBuilderError>;

/// A builder for constructing Merkle trees and generating proofs
pub struct MerkleTreeBuilder {
    mmr: MMR,
}

impl MerkleTreeBuilder {
    fn decode_hex(hex_str: &str) -> Result<[u8; 32]> {
        let hex_to_decode = if hex_str.starts_with("0x") {
            &hex_str[2..]
        } else {
            hex_str
        };
        let mut bytes = hex::decode(hex_to_decode)?;

        // Pad with zeros if needed
        while bytes.len() < 32 {
            bytes.insert(0, 0);
        }
        // Truncate if too long
        if bytes.len() > 32 {
            bytes.truncate(32);
        }

        bytes
            .as_slice()
            .try_into()
            .map_err(|e: TryFromSliceError| TreeBuilderError::ConversionError(e.to_string()))
    }

    /// Creates a new MerkleTreeBuilder instance
    pub fn new() -> Self {
        let store = InMemoryStore::default();
        let store_rc = Arc::new(store);
        let hasher = Arc::new(stark_poseidon::StarkPoseidonHasher::new(None));

        Self {
            mmr: MMR::new(store_rc, hasher, None),
        }
    }

    /// Builds a Merkle tree from a list of commitment hashes
    pub async fn build_merkle(&mut self, leaves: Vec<[u8; 32]>) -> Result<()> {
        for leaf in leaves {
            self.mmr.append(format!("0x{}", hex::encode(leaf))).await?;
        }
        Ok(())
    }

    /// Gets the current Merkle root
    pub async fn get_root(&self) -> Result<[u8; 32]> {
        let bag = self.mmr.bag_the_peaks(None).await?;
        let elements_count = self.mmr.elements_count.get().await?;
        let root = self.mmr.calculate_root_hash(&bag, elements_count)?;
        Self::decode_hex(&root)
    }

    /// Generates a Merkle proof for a given leaf
    pub async fn get_proof(&self, leaf: [u8; 32]) -> Result<Option<Vec<[u8; 32]>>> {
        let elements_count = self.mmr.elements_count.get().await?;
        let leaf_str = format!("0x{}", hex::encode(leaf));

        // Find the leaf index by scanning elements
        let mut leaf_index = None;
        for i in 1..=elements_count {
            if let Some(hash) = self
                .mmr
                .hashes
                .get(accumulators::store::SubKey::Usize(i))
                .await?
            {
                if hash == leaf_str {
                    leaf_index = Some(i);
                    break;
                }
            }
        }

        if let Some(idx) = leaf_index {
            let proof = self.mmr.get_proof(idx, None).await?;
            let mut siblings = Vec::new();
            for h in proof.siblings_hashes {
                siblings.push(Self::decode_hex(&h)?);
            }
            Ok(Some(siblings))
        } else {
            Ok(None)
        }
    }

    /// Verifies a Merkle proof for a given leaf
    pub async fn verify_proof(&self, proof: Vec<[u8; 32]>, leaf: [u8; 32]) -> Result<bool> {
        let elements_count = self.mmr.elements_count.get().await?;
        let leaf_str = format!("0x{}", hex::encode(leaf));

        // Find the leaf index
        let mut leaf_index = None;
        for i in 1..=elements_count {
            if let Some(hash) = self
                .mmr
                .hashes
                .get(accumulators::store::SubKey::Usize(i))
                .await?
            {
                if hash == leaf_str {
                    leaf_index = Some(i);
                    break;
                }
            }
        }

        if let Some(idx) = leaf_index {
            let peaks = self
                .mmr
                .get_peaks(accumulators::mmr::PeaksOptions {
                    elements_count: Some(elements_count),
                    formatting_opts: None,
                })
                .await?;

            let proof_obj = Proof {
                element_index: idx,
                element_hash: leaf_str.clone(),
                siblings_hashes: proof
                    .into_iter()
                    .map(|p| format!("0x{}", hex::encode(p)))
                    .collect(),
                peaks_hashes: peaks,
                elements_count,
            };

            Ok(self.mmr.verify_proof(proof_obj, leaf_str, None).await?)
        } else {
            Ok(false)
        }
    }
}

impl Default for MerkleTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_tree_operations() -> Result<()> {
        let mut builder = MerkleTreeBuilder::new();

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
}

use crate::error::{Result, TreeBuilderError};
use starknet_crypto::poseidon_hash_many;
use starknet_types_core::felt::Felt;

/// Poseidon hasher for L2 Merkle operations
pub struct PoseidonHasher;

impl PoseidonHasher {
    /// Hash two field elements using Poseidon
    pub fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> Result<[u8; 32]> {
        let left_felt = bytes_to_felt(left)?;
        let right_felt = bytes_to_felt(right)?;

        let result = poseidon_hash_many(&[left_felt, right_felt]);
        Ok(felt_to_bytes(&result))
    }

    /// Hash a single field element (for leaf nodes)
    pub fn hash_single(data: &[u8; 32]) -> Result<[u8; 32]> {
        let felt = bytes_to_felt(data)?;
        let result = poseidon_hash_many(&[felt]);
        Ok(felt_to_bytes(&result))
    }

    /// Hash multiple field elements
    pub fn hash_many(data: &[&[u8; 32]]) -> Result<[u8; 32]> {
        if data.is_empty() {
            return Err(TreeBuilderError::PoseidonError(
                "Cannot hash empty data".to_string(),
            ));
        }

        let felts: Result<Vec<Felt>> = data.iter().map(|bytes| bytes_to_felt(bytes)).collect();

        let felts = felts?;
        let result = poseidon_hash_many(&felts);
        Ok(felt_to_bytes(&result))
    }
}

/// Convert 32-byte array to Felt
fn bytes_to_felt(bytes: &[u8; 32]) -> Result<Felt> {
    // Create Felt from bytes - starknet-types-core 0.1 API
    Ok(Felt::from_bytes_be(bytes))
}

/// Convert Felt to 32-byte array
fn felt_to_bytes(felt: &Felt) -> [u8; 32] {
    felt.to_bytes_be()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_pair() {
        let left = [1u8; 32];
        let right = [2u8; 32];

        let result = PoseidonHasher::hash_pair(&left, &right).unwrap();
        assert_ne!(result, [0u8; 32]);

        // Test deterministic behavior
        let result2 = PoseidonHasher::hash_pair(&left, &right).unwrap();
        assert_eq!(result, result2);
    }

    #[test]
    fn test_bytes_felt_conversion() {
        let mut bytes = [0u8; 32];
        bytes[31] = 42;

        let felt = bytes_to_felt(&bytes).unwrap();
        let back_to_bytes = felt_to_bytes(&felt);
        let felt2 = bytes_to_felt(&back_to_bytes).unwrap();

        assert_eq!(felt, felt2);
    }
}

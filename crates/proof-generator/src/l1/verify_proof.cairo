use core::keccak::keccak_u256s_be_inputs;

/// A struct to handle full 256-bit hashes.
#[derive(Drop, Copy, Serde, PartialEq)]
pub struct Hash256 {
    pub high: felt252,
    pub low: felt252,
}

/// Trait for Hash256 operations.
trait Hash256Trait {
    fn from_u256(value: u256) -> Hash256;
    fn to_u256(self: Hash256) -> u256;
    fn from_felt252(value: felt252) -> Hash256;
}

impl Hash256Impl of Hash256Trait {
    /// Creates a Hash256 from a u256.
    fn from_u256(value: u256) -> Hash256 {
        let high: felt252 = value.high.into();
        let low: felt252 = value.low.into();
        Hash256 { high, low }
    }

    /// Converts a Hash256 back to a u256.
    fn to_u256(self: Hash256) -> u256 {
        u256 { high: self.high.try_into().unwrap(), low: self.low.try_into().unwrap() }
    }

    /// Creates a Hash256 from a single felt252, assuming the high part is zero.
    fn from_felt252(value: felt252) -> Hash256 {
        Hash256 { high: 0, low: value }
    }
}

/// Hashes a single felt252 value using Keccak256.
fn keccak_hash_single(value: Hash256) -> Hash256 {
    let value_u256: u256 = value.to_u256();
    let hash_u256 = keccak_u256s_be_inputs(array![value_u256].span());
    Hash256Trait::from_u256(hash_u256)
}

/// Hashes two Hash256 values together using Keccak256.
fn keccak_hash_double(left: Hash256, right: Hash256) -> Hash256 {
    let left_u256 = left.to_u256();
    let right_u256 = right.to_u256();
    let hash_u256 = keccak_u256s_be_inputs(array![left_u256, right_u256].span());
    Hash256Trait::from_u256(hash_u256)
}

/// The standard MMR proof structure using full Hash256.
#[derive(Drop, Clone, Serde)]
struct MmrProof {
    leaf_index: u32,
    leaf_value: Hash256,
    sibling_hashes: Array<Hash256>,
    peaks: Array<Hash256>,
    mmr_size: u32,
}
pub fn verify_mmr_proof(
    leaf: Hash256,
    leaf_index: u32,
    sibling_hashes: Array<Hash256>,
    peaks: Array<Hash256>,
    mmr_size: u32,
    root: Hash256,
) -> bool {
    let proof = MmrProof { leaf_index, leaf_value: leaf, sibling_hashes, peaks, mmr_size };

    verify_proof(leaf, proof, root)
}

/// Main function to verify an MMR proof against a root.
fn verify_proof(leaf: Hash256, proof: MmrProof, root: Hash256) -> bool {
    if proof.leaf_value != leaf {
        return false;
    }

    if !peaks_valid(proof.peaks.span(), proof.mmr_size, root) {
        return false;
    }

    let computed_peak = compute_peak_from_leaf(proof.leaf_index, leaf, proof.sibling_hashes.span());
    peaks_contains(proof.peaks.span(), computed_peak)
}

/// Computes the peak hash from a leaf and its sibling proof hashes.
fn compute_peak_from_leaf(leaf_index: u32, leaf_value: Hash256, proof: Span<Hash256>) -> Hash256 {
    let mut current_index = leaf_index;
    let mut current_hash = keccak_hash_single(leaf_value);
    let mut proof_index = 0;

    while proof_index < proof.len() {
        let sibling_hash = *proof.at(proof_index);

        // The order of hashing depends on whether the current node is a left or right child.
        if is_left_child(current_index) {
            current_hash = keccak_hash_double(current_hash, sibling_hash);
        } else {
            current_hash = keccak_hash_double(sibling_hash, current_hash);
        }

        current_index = get_parent_index(current_index);
        proof_index += 1;
    };

    current_hash
}

/// Hashes all MMR peaks together to produce a single value.
fn bag_peaks(peaks: Span<Hash256>) -> Hash256 {
    if peaks.len() == 0 {
        // Return a zero hash if there are no peaks.
        return Hash256 { high: 0, low: 0 };
    }

    if peaks.len() == 1 {
        return *peaks.at(0);
    }

    // Bagging is done from right to left.
    let mut result = *peaks.at(peaks.len() - 1);
    let mut i = peaks.len() - 1;

    while i > 0 {
        i -= 1;
        result = keccak_hash_double(*peaks.at(i), result);
    };

    result
}

/// Validates that the bagged peaks combined with the MMR size hash to the final root.
fn peaks_valid(peaks: Span<Hash256>, mmr_size: u32, root: Hash256) -> bool {
    let bagged_peaks = bag_peaks(peaks);
    let mmr_size_hash = Hash256Trait::from_felt252(mmr_size.into());
    let computed_root = keccak_hash_double(mmr_size_hash, bagged_peaks);
    computed_root == root
}

/// Checks if a specific peak hash exists in the list of peaks.
fn peaks_contains(peaks: Span<Hash256>, peak: Hash256) -> bool {
    let mut i = 0;
    loop {
        if i >= peaks.len() {
            break false;
        }
        if *peaks.at(i) == peak {
            break true;
        }
        i += 1;
    }
}

/// Determines if a node at a given index is a left child.
fn is_left_child(index: u32) -> bool {
    // In this MMR implementation, left children have odd indices.
    index % 2 == 1
}

/// Calculates the index of the parent node.
fn get_parent_index(index: u32) -> u32 {
    if index <= 1 {
        return 0; // Should not happen in a valid tree path.
    }
    (index + 1) / 2
}


#[cfg(test)]
mod tests {
    use super::{
        Hash256, Hash256Trait, MmrProof, keccak_hash_double, keccak_hash_single, verify_proof,
    };

    /// Helper function to create an MMR proof for testing.
    fn create_test_proof(
        leaf_value: Hash256,
        leaf_index: u32,
        sibling_hashes: Array<Hash256>,
        peaks: Array<Hash256>,
        mmr_size: u32,
    ) -> MmrProof {
        MmrProof { leaf_index, leaf_value, sibling_hashes, peaks, mmr_size }
    }

    #[test]
    #[available_gas(20000000)]
    fn test_hash256_operations() {
        // Test Hash256 creation and conversion
        let test_u256 = u256 { high: 0x123456789abcdef, low: 0xfedcba9876543210 };
        let hash = Hash256Trait::from_u256(test_u256);
        let converted_back = hash.to_u256();

        assert(converted_back.high == test_u256.high, 'High part mismatch');
        assert(converted_back.low == test_u256.low, 'Low part mismatch');
    }

    #[test]
    #[available_gas(50000000)]
    fn test_hashing() {
        let value1 = Hash256 { low: 100, high: 0 };

        // Test single hash
        let hash1 = keccak_hash_single(value1);
        let hash2 = keccak_hash_single(value1);
        assert(hash1 == hash2, 'Same input must give same hash');

        // Test double hash
        let hash3 = keccak_hash_single(Hash256 { low: 200, high: 0 });
        let combined = keccak_hash_double(hash1, hash3);
        assert(combined != hash1, 'Combined hash should differ');
    }

    #[test]
    #[available_gas(50000000)]
    fn test_large_hash_handling() {
        // Test with a hash that exceeds felt252 capacity
        let large_u256 = u256 { high: 0x123456789abcdef0123456789abcdef, low: 0x1 };

        let large_hash = Hash256Trait::from_u256(large_u256);

        // Test that we can still perform operations
        let another_hash = Hash256Trait::from_felt252(123);
        let combined = keccak_hash_double(large_hash, another_hash);

        // The result should be well-defined and not zero.
        assert(combined.high != 0 || combined.low != 0, 'Combined should not be zero');
    }


    #[test]
    #[available_gas(50000000)]
    fn test_verify_proof_success() {
        let leaf_value = Hash256 { low: 42, high: 0 };
        let leaf_hash = keccak_hash_single(leaf_value);
        let sibling_hash = keccak_hash_single(Hash256 { low: 84, high: 0 });
        let peak_hash = keccak_hash_double(leaf_hash, sibling_hash);

        let mut sibling_hashes: Array<Hash256> = ArrayTrait::new();
        sibling_hashes.append(sibling_hash);

        let mut peaks: Array<Hash256> = ArrayTrait::new();
        peaks.append(peak_hash);

        let mmr_size = 3_u32;
        let mmr_size_hash = Hash256Trait::from_felt252(mmr_size.into());
        let root = keccak_hash_double(mmr_size_hash, peak_hash);

        let proof = create_test_proof(leaf_value, 1, sibling_hashes, peaks, mmr_size);

        assert(verify_proof(leaf_value, proof, root), 'Proof should verify');
    }

    #[test]
    #[available_gas(50000000)]
    fn test_verify_proof_failures() {
        let leaf_value = Hash256 { low: 42, high: 0 };
        let leaf_hash = keccak_hash_single(leaf_value);
        let sibling_hash = keccak_hash_single(Hash256 { low: 84, high: 0 });
        let peak_hash = keccak_hash_double(leaf_hash, sibling_hash);

        let mut sibling_hashes: Array<Hash256> = ArrayTrait::new();
        sibling_hashes.append(sibling_hash);

        let mut peaks: Array<Hash256> = ArrayTrait::new();
        peaks.append(peak_hash);

        let mmr_size = 3_u32;
        let root = keccak_hash_double(Hash256Trait::from_felt252(mmr_size.into()), peak_hash);
        let wrong_root = keccak_hash_single(Hash256 { low: 999, high: 0 });

        // Test invalid leaf
        let proof_with_wrong_leaf = create_test_proof(
            Hash256 { low: 999, high: 0 }, 1, sibling_hashes.clone(), peaks.clone(), mmr_size,
        );
        assert(!verify_proof(leaf_value, proof_with_wrong_leaf, root), 'Wrong leaf should fail');

        // Test invalid root
        let valid_proof = create_test_proof(
            leaf_value, 1, sibling_hashes.clone(), peaks.clone(), mmr_size,
        );
        assert(!verify_proof(leaf_value, valid_proof, wrong_root), 'Wrong root should fail');
    }
}

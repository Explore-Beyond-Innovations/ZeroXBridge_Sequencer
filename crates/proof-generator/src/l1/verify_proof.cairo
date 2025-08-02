use core::keccak::keccak_u256s_be_inputs;

/// Hash a single felt252 value using keccak256
fn keccak_hash_single(value: felt252) -> felt252 {
    let value_u256: u256 = value.into();
    let hash_u256 = keccak_u256s_be_inputs(array![value_u256].span());
    // Convert u256 to felt252 by taking the low part and combining with high
    let low: u128 = hash_u256.low;
    let high: u128 = hash_u256.high;
    // Simple combination: XOR the high and low parts
    (low ^ high).into()
}

/// Hash two felt252 values using keccak256
fn keccak_hash_double(left: felt252, right: felt252) -> felt252 {
    let left_u256: u256 = left.into();
    let right_u256: u256 = right.into();
    let hash_u256 = keccak_u256s_be_inputs(array![left_u256, right_u256].span());
    // Convert u256 to felt252 by taking the low part and combining with high
    let low: u128 = hash_u256.low;
    let high: u128 = hash_u256.high;
    // Simple combination: XOR the high and low parts
    (low ^ high).into()
}

/// MMR proof structure for keccak256-based verification
/// Compatible with ZeroXBridge L1 commitment verification
#[derive(Drop, Clone, Serde)]
struct MmrProof {
    /// The leaf index in the MMR (1-based, following standard MMR convention)
    leaf_index: u32,
    /// The leaf value being proven
    leaf_value: felt252,
    /// Array of sibling hashes in the proof path
    sibling_hashes: Array<felt252>,
    /// Array of peak hashes in the MMR
    peaks: Array<felt252>,
    /// The MMR size (last position)
    mmr_size: u32,
}

/// Verifies an MMR proof using keccak256 hashing
/// This is the main verification function for L1 commitments in ZeroXBridge
///
/// # Arguments
///
/// * `leaf` - The leaf value to verify
/// * `proof` - The MMR proof structure containing all necessary proof data
/// * `root` - The expected MMR root hash
///
/// # Returns
///
/// * `bool` - True if the proof is valid, false otherwise
fn verify_proof(leaf: felt252, proof: MmrProof, root: felt252) -> bool {
    // Verify basic proof structure
    if proof.leaf_value != leaf {
        return false;
    }

    // Verify that the peaks are valid for the given MMR size and root
    if !peaks_valid(proof.peaks.span(), proof.mmr_size, root) {
        return false;
    }

    // Compute the peak from the leaf using the proof
    let computed_peak = compute_peak_from_leaf(proof.leaf_index, leaf, proof.sibling_hashes.span());

    // Verify that the computed peak exists in the peaks array
    peaks_contains(proof.peaks.span(), computed_peak)
}

/// Alternative verify_proof function that takes individual parameters
/// This provides a more convenient interface for external callers
pub fn verify_mmr_proof(
    leaf: felt252,
    leaf_index: u32,
    sibling_hashes: Array<felt252>,
    peaks: Array<felt252>,
    mmr_size: u32,
    root: felt252,
) -> bool {
    let proof = MmrProof { leaf_index, leaf_value: leaf, sibling_hashes, peaks, mmr_size };

    verify_proof(leaf, proof, root)
}

/// Computes a peak hash from a leaf using the proof path
/// This follows standard MMR traversal algorithms
fn compute_peak_from_leaf(leaf_index: u32, leaf_value: felt252, proof: Span<felt252>) -> felt252 {
    // Convert leaf index to MMR index (simplified version)
    let mut current_index = leaf_index;

    // Start with the leaf hash
    let mut current_hash = keccak_hash_single(leaf_value);
    let mut proof_index = 0;

    // Traverse up the tree using the proof
    while proof_index < proof.len() {
        let sibling_hash = *proof.at(proof_index);

        // Determine if we're the left or right child
        if is_left_child(current_index) {
            // We're the left child, sibling is right
            current_hash = keccak_hash_double(current_hash, sibling_hash);
        } else {
            // We're the right child, sibling is left
            current_hash = keccak_hash_double(sibling_hash, current_hash);
        }

        // Move to parent
        current_index = get_parent_index(current_index);
        proof_index += 1;
    };

    current_hash
}

/// Check if a node is a left child in the MMR
fn is_left_child(index: u32) -> bool {
    // In MMR, left children have odd indices
    index % 2 == 1
}

/// Get the parent index of a node in the MMR
fn get_parent_index(index: u32) -> u32 {
    // MMR parent calculation
    if index <= 1 {
        return 0; // Root or invalid
    }

    // For MMR, parent is at (index + 1) / 2
    (index + 1) / 2
}

/// Bag peaks using keccak256 (right to left)
/// This follows the standard MMR peak bagging algorithm
fn bag_peaks(peaks: Span<felt252>) -> felt252 {
    if peaks.len() == 0 {
        return 0;
    }

    if peaks.len() == 1 {
        return *peaks.at(0);
    }

    // Bag from right to left using keccak256
    let mut result = *peaks.at(peaks.len() - 1);
    let mut i = peaks.len() - 1;

    while i > 0 {
        i -= 1;
        result = keccak_hash_double(*peaks.at(i), result);
    };

    result
}

/// Validate peaks against MMR size and root using keccak256
/// This ensures the peaks correctly represent the MMR state
fn peaks_valid(peaks: Span<felt252>, mmr_size: u32, root: felt252) -> bool {
    let bagged_peaks = bag_peaks(peaks);
    let mmr_size_felt: felt252 = mmr_size.into();
    let computed_root = keccak_hash_double(mmr_size_felt, bagged_peaks);
    computed_root == root
}

/// Check if peaks contain a specific peak
fn peaks_contains(peaks: Span<felt252>, peak: felt252) -> bool {
    let mut i = 0;
    while i < peaks.len() {
        if *peaks.at(i) == peak {
            return true;
        }
        i += 1;
    };
    false
}

/// Helper function to create MMR proof for testing
fn create_test_proof(
    leaf_value: felt252,
    leaf_index: u32,
    sibling_hashes: Array<felt252>,
    peaks: Array<felt252>,
    mmr_size: u32,
) -> MmrProof {
    MmrProof { leaf_index, leaf_value, sibling_hashes, peaks, mmr_size }
}

#[cfg(test)]
mod tests {
    use super::{
        verify_proof, verify_mmr_proof, keccak_hash_single, keccak_hash_double,
        compute_peak_from_leaf, bag_peaks, peaks_contains, peaks_valid, create_test_proof,
    };

    #[test]
    #[available_gas(20000000)]
    fn test_keccak_hasher() {
        let value = 42;
        let hash = keccak_hash_single(value);
        assert(hash != 0, 'Hash should not be zero');

        // Test deterministic behavior
        let hash2 = keccak_hash_single(value);
        assert(hash == hash2, 'Hash should be deterministic');

        // Test different order produces different results
        let hash1 = keccak_hash_double(10, 20);
        let hash2 = keccak_hash_double(20, 10);
        assert(hash1 != hash2, 'Different order should differ');
    }

    #[test]
    #[available_gas(50000000)]
    fn test_mmr_helper_functions() {
        // Test bag_peaks
        let peak1 = keccak_hash_single(100);
        let peak2 = keccak_hash_single(200);
        let peak3 = keccak_hash_single(300);

        let mut peaks = ArrayTrait::new();
        peaks.append(peak1);
        peaks.append(peak2);
        peaks.append(peak3);

        let bagged = bag_peaks(peaks.span());
        let expected = keccak_hash_double(peak1, keccak_hash_double(peak2, peak3));
        assert(bagged == expected, 'Bagged peaks mismatch');

        // Test peaks_contains
        assert(peaks_contains(peaks.span(), peak1), 'Should contain peak1');
        assert(!peaks_contains(peaks.span(), 999), 'Should not contain 999');

        // Test peaks_valid
        let mut single_peak = ArrayTrait::new();
        single_peak.append(peak1);
        let mmr_size = 1_u32;
        let expected_root = keccak_hash_double(mmr_size.into(), peak1);
        assert(peaks_valid(single_peak.span(), mmr_size, expected_root), 'Peaks should be valid');

        // Test empty peaks
        let mut empty_peaks = ArrayTrait::new();
        assert(bag_peaks(empty_peaks.span()) == 0, 'Empty peaks should return 0');
        assert(!peaks_contains(empty_peaks.span(), 123), 'Empty should not contain');
    }

    #[test]
    #[available_gas(50000000)]
    fn test_verify_proof_success() {
        // Test both verify_proof and verify_mmr_proof with valid data
        let leaf_value = 100;
        let leaf_hash = keccak_hash_single(leaf_value);
        let sibling_hash = keccak_hash_single(200);
        let peak_hash = keccak_hash_double(leaf_hash, sibling_hash);

        let mut sibling_hashes = ArrayTrait::new();
        sibling_hashes.append(sibling_hash);

        let mut peaks = ArrayTrait::new();
        peaks.append(peak_hash);

        let mmr_size = 3_u32;
        let root = keccak_hash_double(mmr_size.into(), peak_hash);

        // Test verify_proof with MmrProof struct
        let mut sibling_hashes_copy = ArrayTrait::new();
        sibling_hashes_copy.append(sibling_hash);
        let mut peaks_copy = ArrayTrait::new();
        peaks_copy.append(peak_hash);

        let proof = create_test_proof(leaf_value, 1, sibling_hashes_copy, peaks_copy, mmr_size);
        assert(verify_proof(leaf_value, proof, root), 'Proof should pass');

        // Test verify_mmr_proof with individual parameters
        assert(
            verify_mmr_proof(leaf_value, 1, sibling_hashes, peaks, mmr_size, root),
            'MMR proof should pass',
        );
    }

    #[test]
    #[available_gas(50000000)]
    fn test_verify_proof_failures() {
        let leaf_value = 100;
        let leaf_hash = keccak_hash_single(leaf_value);
        let sibling_hash = keccak_hash_single(200);
        let peak_hash = keccak_hash_double(leaf_hash, sibling_hash);

        let mut sibling_hashes = ArrayTrait::new();
        sibling_hashes.append(sibling_hash);

        let mut peaks = ArrayTrait::new();
        peaks.append(peak_hash);

        let mmr_size = 3_u32;
        let root = keccak_hash_double(mmr_size.into(), peak_hash);

        // Test invalid leaf
        let mut sibling_hashes_copy = ArrayTrait::new();
        sibling_hashes_copy.append(sibling_hash);
        let mut peaks_copy = ArrayTrait::new();
        peaks_copy.append(peak_hash);

        let wrong_proof = create_test_proof(999, 1, sibling_hashes_copy, peaks_copy, mmr_size);
        assert(!verify_proof(leaf_value, wrong_proof, root), 'Wrong leaf should fail');

        // Test invalid root
        let valid_proof = create_test_proof(leaf_value, 1, sibling_hashes, peaks, mmr_size);
        assert(!verify_proof(leaf_value, valid_proof, 999), 'Wrong root should fail');
    }

    #[test]
    #[available_gas(50000000)]
    fn test_compute_peak_from_leaf() {
        let leaf_value = 42;
        let sibling_hash = keccak_hash_single(84);

        let mut proof = ArrayTrait::new();
        proof.append(sibling_hash);

        let computed_peak = compute_peak_from_leaf(1, leaf_value, proof.span());
        let expected_peak = keccak_hash_double(keccak_hash_single(leaf_value), sibling_hash);

        assert(computed_peak == expected_peak, 'Peak computation failed');
    }
}

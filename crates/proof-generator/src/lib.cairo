mod l2;

/// Verifies a Merkle proof for a given leaf hash against an expected root hash
///
/// # Arguments
///
/// * `commitment_hash` (felt252) - The hash of the leaf node to verify.
/// * `proof` (Array<felt252>) - An array of sibling hashes.
/// * `new_root` (felt252) - The expected Merkle root hash.
///
/// # Returns
///
/// * `felt252` - The `new_root` if the verification is successful.
///
/// # Panics
///
/// * Panics with the message 'Computed root does not match' if the verification fails.
fn verify_commitment_in_root(
    commitment_hash: felt252, proof: Array<felt252>, new_root: felt252,
) -> felt252 {
    assert(!proof.is_empty(), 'Proof must not be empty');
    let proof_span: Span<felt252> = proof.span();

    let mut computed_root: felt252 = commitment_hash;
    for proof_element in proof_span {
        computed_root =
            if Into::<felt252, u256>::into(computed_root) < (*proof_element).into() {
                core::pedersen::pedersen(computed_root, *proof_element)
            } else {
                core::pedersen::pedersen(*proof_element, computed_root)
            };
    };

    assert(computed_root == new_root, 'Computed root does not match');

    computed_root
}


/// Verifies a Merkle proof by computing the root from a leaf and its siblings.
///
/// This function takes a flattened array containing the expected Merkle root,
/// the leaf hash, and the sibling hashes that constitute the proof path.
///
/// # Arguments
///
/// * `input` (`Array<felt252>`) - An array containing the Merkle proof components,
///   structured precisely as follows:
///   - `input[0]`: expected Merkle root (`root`).
///   - `input[1]`: hash of the leaf (`leaf`).
///   - `input[2..N]`: sibling hashes (`siblings`).
///
/// # Returns
///
/// * `Array<felt252>` - An array containing a single `felt252` element representing
///   the computed Merkle root.
fn main(input: Array<felt252>) -> Array<felt252> {
    assert(input.len() >= 3, 'invalid input');
    let mut proof = ArrayTrait::new();
    for i in 2..input.len() {
        proof.append(*input.at(i));
    };
    let verified_root = verify_commitment_in_root(*input.at(1), proof, *input.at(0));

    let mut result_array = ArrayTrait::new();
    result_array.append(verified_root);
    result_array
}


#[cfg(test)]
mod tests {
    use super::verify_commitment_in_root;
    use super::l2::verify_proof::{verify_proof, verify_proof_legacy, MmrProof};

    // Helper function to build a simple 4-leaf tree for tests
    fn build_test_tree() -> (felt252, felt252, felt252, felt252, felt252, felt252, felt252) {
        let leaf_0: felt252 = 10;
        let leaf_1: felt252 = 20;
        let leaf_2: felt252 = 30;
        let leaf_3: felt252 = 40;

        let h_01 = core::pedersen::pedersen(leaf_0, leaf_1);
        let h_23 = core::pedersen::pedersen(leaf_2, leaf_3);
        let root = core::pedersen::pedersen(h_01, h_23);

        (leaf_0, leaf_1, leaf_2, leaf_3, h_01, h_23, root)
    }

    #[test]
    #[available_gas(5000000)]
    fn test_correct_proof_passes() {
        let (leaf_0, leaf_1, leaf_2, leaf_3, h_01, h_23, root) = build_test_tree();

        let mut proof_0: Array = ArrayTrait::<felt252>::new();
        proof_0.append(leaf_1); // Sibling at level 0
        proof_0.append(h_23); // Sibling at level 1
        let verified_root_0 = verify_commitment_in_root(leaf_0, proof_0, root);
        assert(verified_root_0 == root, 'Leaf 0 verification failed');

        let mut proof_3: Array = ArrayTrait::<felt252>::new();
        proof_3.append(leaf_2); // Sibling at level 0
        proof_3.append(h_01); // Sibling at level 1
        let verified_root_3 = verify_commitment_in_root(leaf_3, proof_3, root);
        assert(verified_root_3 == root, 'Leaf 3 verification failed');
    }

    #[test]
    #[available_gas(2000000)]
    #[should_panic(expected: ('Computed root does not match',))]
    fn test_wrong_leaf_fails() {
        let (_, leaf_1, _, _, _, h_23, root) = build_test_tree();
        let mut proof_0 = ArrayTrait::<felt252>::new();
        proof_0.append(leaf_1);
        proof_0.append(h_23);

        let wrong_leaf = 999;
        verify_commitment_in_root(wrong_leaf, proof_0, root);
    }

    #[test]
    #[available_gas(2000000)]
    #[should_panic(expected: ('Computed root does not match',))]
    fn test_wrong_sibling_fails() {
        let (leaf_0, _, _, _, _, h_23, root) = build_test_tree();
        let mut proof_0_wrong = ArrayTrait::<felt252>::new();
        proof_0_wrong.append(998); // Wrong sibling for leaf 0
        proof_0_wrong.append(h_23);

        verify_commitment_in_root(leaf_0, proof_0_wrong, root);
    }

    #[test]
    #[available_gas(2000000)]
    #[should_panic(expected: ('Computed root does not match',))]
    fn test_wrong_root_fails() {
        let (leaf_0, leaf_1, _, _, _, h_23, _) = build_test_tree();
        let mut proof_0 = ArrayTrait::<felt252>::new();
        proof_0.append(leaf_1);
        proof_0.append(h_23);

        let wrong_root = 777;
        verify_commitment_in_root(leaf_0, proof_0, wrong_root);
    }

    #[test]
    #[available_gas(2000000)]
    #[should_panic]
    fn test_empty_proof_fails() {
        // Only run this test if tree height > 0
        let (leaf_0, _, _, _, _, _, root) = build_test_tree();
        let mut empty_proof = ArrayTrait::<felt252>::new();

        verify_commitment_in_root(leaf_0, empty_proof, root);
    }

    #[test]
    #[available_gas(3000000)]
    fn test_l2_verify_proof_integration() {
        // Test single element MMR case
        let leaf = 42;
        let leaf_index = 0;
        let proof: MmrProof = array![].span();
        let root = leaf; // For single element MMR, root equals leaf
        let merkle_size = 1;
        let peaks = array![leaf];

        let is_valid = verify_proof(leaf, leaf_index, proof, root, merkle_size, peaks);
        assert(is_valid, 'L2 verify_proof failed');
    }


    #[test]
    #[available_gas(2000000)]
    #[should_panic]
    fn test_malformed_proof_fails() {
        let (leaf_0, leaf_1, _, _, _, _, root) = build_test_tree();
        let mut short_proof = ArrayTrait::<felt252>::new();
        short_proof.append(leaf_1); // Only one element provided

        verify_commitment_in_root(leaf_0, short_proof, root);
    }
}

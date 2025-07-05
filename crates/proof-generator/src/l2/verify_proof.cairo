use cairo_lib::hashing::poseidon::PoseidonHasher;
use cairo_lib::data_structures::mmr::proof::{Proof, ProofTrait};

// Define cairo_lib MMR type
pub type MmrProof = Proof;

/// Verifies an MMR proof using cairo_lib implementation
///
/// # Arguments
///
/// * `leaf` (felt252) - The leaf value to verify
/// * `leaf_index` (usize) - The leaf index in the MMR
/// * `proof` (MmrProof) - Proof elements from cairo_lib
/// * `root` (felt252) - Expected MMR root
///
/// # Returns
///
/// * `bool` - True if verification succeeds, false otherwise
pub fn verify_proof(leaf: felt252, leaf_index: usize, proof: MmrProof, root: felt252) -> bool {
    // Handle empty proof case (single element MMR)
    if proof.len() == 0 {
        return leaf == root;
    }

    let computed_root = proof.compute_peak(leaf_index, leaf);

    computed_root == root
}


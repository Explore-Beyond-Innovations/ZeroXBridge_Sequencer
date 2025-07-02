//! Example demonstrating L2 Merkle tree operations with Poseidon hashing
//!
//! This example shows how to:
//! 1. Build an L2 Merkle tree from withdrawal commitment hashes
//! 2. Generate Merkle proofs for specific commitments
//! 3. Verify proofs for zk-STARK proof generation

use tree_builder::{L2MerkleTree, Result};

fn main() -> Result<()> {
    println!("L2 Merkle Tree Example\n");

    // Sample commitment hashes (32 bytes each, hex encoded without 0x prefix)
    let commitment_hashes = vec![
        "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
        "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        "0987654321fedcba0987654321fedcba0987654321fedcba0987654321fedcba",
    ];

    println!(
        "Building L2 Merkle tree with {} commitment hashes:",
        commitment_hashes.len()
    );
    for (i, hash) in commitment_hashes.iter().enumerate() {
        println!("  [{}]: {}", i, hash);
    }

    // Parse hex strings to bytes
    let mut leaves = Vec::new();
    for hash_str in &commitment_hashes {
        let bytes = hex::decode(hash_str)?;
        if bytes.len() != 32 {
            return Err(tree_builder::TreeBuilderError::InvalidLeafHash(format!(
                "Expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        leaves.push(array);
    }

    // Build the Merkle tree
    let tree = L2MerkleTree::build_l2_merkle(leaves)?;
    println!("\nTree built successfully!");

    // Get the root
    let root = tree.get_root().expect("Tree should have a root");
    println!("Root: {}", hex::encode(root));

    // Generate proofs for each commitment
    println!("\nGenerating Merkle Proofs:");
    for (i, leaf) in tree.get_leaves().iter().enumerate() {
        match tree.get_proof(*leaf) {
            Ok(proof) => {
                println!("\nProof for commitment [{}]:", i);
                println!("  Hash: {}", hex::encode(leaf));
                println!("  Proof length: {}", proof.siblings.len());
                println!("  Leaf index: {}", proof.index);

                // Verify the proof
                let is_valid = proof.verify()?;
                println!(
                    "  Proof verification: {}",
                    if is_valid { "VALID" } else { "INVALID" }
                );
            }
            Err(e) => {
                println!("Failed to generate proof for commitment [{}]: {}", i, e);
            }
        }
    }

    println!("\nExample completed successfully");
    Ok(())
}

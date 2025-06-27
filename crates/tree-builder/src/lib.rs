//! # Tree Builder
//!
//! A Rust library for building L2 Merkle trees with Poseidon hashing.
//! This crate provides functionality to:
//!
//! - Build Merkle trees from L2 withdrawal commitment hashes
//! - Generate Merkle proofs for inclusion verification
//! - Use Poseidon hash function (Starknet-compatible)
//! - Support Merkle Mountain Range (MMR) operations
//!
//! ## Example
//!
//! ```rust
//! use tree_builder::{L2MerkleTree, utils::parse_commitment_hashes};
//!
//! // Parse commitment hashes from hex strings
//! let commitment_hashes = vec![
//!     "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
//!     "0xfedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321".to_string(),
//! ];
//!
//! let leaves = parse_commitment_hashes(&commitment_hashes).unwrap();
//!
//! // Build the Merkle tree
//! let tree = L2MerkleTree::build_l2_merkle(leaves.clone()).unwrap();
//!
//! // Get the root hash
//! let root = tree.get_root().unwrap();
//!
//! // Generate a proof for a specific commitment
//! let proof = tree.get_proof(leaves[0]).unwrap();
//!
//! // Verify the proof
//! assert!(proof.verify().unwrap());
//! ```

pub mod error;
pub mod l2;

pub use error::*;
pub use l2::*;

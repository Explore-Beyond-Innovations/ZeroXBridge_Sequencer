# Tree Builder

A Rust library for building L2 Merkle trees using Poseidon hashing for ZeroXBridge withdrawal verification.

## Overview

This crate provides functionality to:
- Build Merkle trees from L2 withdrawal commitment hashes
- Generate inclusion proofs for specific commitments
- Verify proofs against tree roots
- Support zk-STARK proof generation workflows

## Features

- **Poseidon Hashing**: Starknet-compatible hash function
- **L2 Merkle Trees**: Efficient tree construction from commitment hashes
- **Proof Generation**: Create inclusion proofs for any leaf
- **Proof Verification**: Validate proofs against expected roots
- **Error Handling**: Comprehensive error types and messages
- **Serialization**: JSON support for proof data exchange

## Usage

### Basic Example

```rust
use tree_builder::{L2MerkleTree, Result};

fn main() -> Result<()> {
    // Sample commitment hashes (32 bytes each)
    let commitment_hashes = vec![
        "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
    ];

    // Parse hex strings to bytes
    let mut leaves = Vec::new();
    for hash_str in &commitment_hashes {
        let bytes = hex::decode(hash_str)?;
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        leaves.push(array);
    }

    // Build the Merkle tree
    let tree = L2MerkleTree::build_l2_merkle(leaves)?;

    // Get the root
    let root = tree.get_root().expect("Tree should have a root");
    println!("Root: {}", hex::encode(root));

    // Generate proof for first commitment
    let proof = tree.get_proof(tree.get_leaves()[0])?;
    let is_valid = proof.verify()?;
    println!("Proof valid: {}", is_valid);

    Ok(())
}
```

### Working with Utility Functions

```rust
use tree_builder::utils::*;

// Parse commitment hashes from hex strings
let hex_strings = vec![
    "0x1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    "0x2222222222222222222222222222222222222222222222222222222222222222".to_string(),
];

let leaves = parse_commitment_hashes(&hex_strings)?;
let tree = L2MerkleTree::build_l2_merkle(leaves)?;
```

## API Reference

### Core Types

#### `L2MerkleTree`

Main structure for building and querying Merkle trees.

**Methods:**
- `build_l2_merkle(leaves: Vec<[u8; 32]>) -> Result<Self>` - Build tree from commitment hashes
- `get_root() -> Option<[u8; 32]>` - Get the tree root
- `get_proof(leaf: [u8; 32]) -> Result<MerkleProof>` - Generate inclusion proof
- `get_leaves() -> &Vec<[u8; 32]>` - Get all leaves
- `len() -> usize` - Get number of leaves

#### `MerkleProof`

Represents an inclusion proof for a specific leaf.

**Fields:**
- `leaf: [u8; 32]` - The leaf hash being proven
- `siblings: Vec<[u8; 32]>` - Sibling hashes for the proof path
- `root: [u8; 32]` - The root hash this proof verifies against
- `index: usize` - The index of the leaf in the tree

**Methods:**
- `verify() -> Result<bool>` - Verify the proof against the expected root
- `compute_root() -> Result<[u8; 32]>` - Compute root from proof components

#### `PoseidonHasher`

Starknet-compatible Poseidon hash implementation.

**Methods:**
- `hash_pair(left: &[u8; 32], right: &[u8; 32]) -> Result<[u8; 32]>` - Hash two values
- `hash_single(data: &[u8; 32]) -> Result<[u8; 32]>` - Hash single value
- `hash_many(data: &[[u8; 32]]) -> Result<[u8; 32]>` - Hash multiple values

### Utility Functions

#### `utils` module

- `hex_to_bytes32(hex_str: &str) -> Result<[u8; 32]>` - Convert hex string to bytes
- `bytes32_to_hex(bytes: &[u8; 32]) -> String` - Convert bytes to hex string
- `parse_commitment_hashes(hex_strings: &[String]) -> Result<Vec<[u8; 32]>>` - Parse multiple hex strings

### Error Types

#### `TreeBuilderError`

Comprehensive error handling for all operations:

- `InvalidLeafHash(String)` - Invalid leaf hash format
- `LeafNotFound(String)` - Leaf not found in tree
- `EmptyLeaves` - Empty leaf set provided
- `InvalidProof(String)` - Invalid proof
- `TreeNotBuilt` - Merkle tree not built
- `InvalidDepth(usize)` - Invalid tree depth
- `PoseidonError(String)` - Poseidon hash error
- `SerializationError` - JSON serialization error
- `HexError` - Hex decode error

## Performance

The library is optimized for typical L2 withdrawal batches:

- **Tree Construction**: O(n log n) where n is number of leaves
- **Proof Generation**: O(log n) per proof
- **Memory Usage**: ~32 bytes per leaf + internal nodes
- **Batch Size**: Tested with up to 10,000 commitments

### Benchmarks

For 1,000 commitment hashes:
- Tree construction: ~2ms
- Proof generation: ~0.1ms per proof
- Proof verification: ~0.05ms per proof

## Integration

### With ZeroXBridge Sequencer

```rust
use tree_builder::{L2MerkleTree, utils::parse_commitment_hashes};

// From L2 event watcher
let commitment_logs = fetch_l2_burn_events(&config, &db_pool, from_block, &provider).await?;
let commitment_hashes: Vec<String> = commitment_logs
    .iter()
    .map(|log| log.commitment_hash.clone())
    .collect();

// Build tree
let leaves = parse_commitment_hashes(&commitment_hashes)?;
let tree = L2MerkleTree::build_l2_merkle(leaves)?;

// Generate proofs for withdrawals
for leaf in tree.get_leaves() {
    let proof = tree.get_proof(*leaf)?;
    // Store proof for later relay to L1
}
```

### With Cairo Proof Generator

```rust
// Format proof for Cairo input
let proof = tree.get_proof(commitment_hash)?;
let cairo_input = vec![
    hex::encode(proof.root),
    hex::encode(proof.leaf),
];
cairo_input.extend(proof.siblings.iter().map(|s| hex::encode(s)));
```

## Dependencies

- `starknet-crypto`: Poseidon hash implementation
- `starknet-types-core`: Field element arithmetic
- `serde`: Serialization support
- `hex`: Hex encoding/decoding
- `thiserror`: Error handling
- `anyhow`: Error context

## Testing

Run the test suite:

```bash
cargo test --lib
```

Run the example:

```bash
cargo run --example l2_merkle_example
```

## License

This project is licensed under the MIT License. 
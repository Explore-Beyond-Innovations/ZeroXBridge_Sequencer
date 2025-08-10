use zeroxbridge_sequencer::utils::{compute_poseidon_commitment_hash, HashMethod};
use starknet::core::types::Felt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poseidon_hash_internal_computation() {
        // Test data that matches what we'd expect from a deposit request
        let recipient = Felt::from_hex("0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7").unwrap();
        let amount: u128 = 1000000;
        let nonce: u64 = 0; // First nonce should be 0
        let timestamp: u64 = 1672531200;

        // Compute hash using BatchHash method (our default)
        let hash_batch = compute_poseidon_commitment_hash(
            recipient,
            amount,
            nonce,
            timestamp,
            HashMethod::BatchHash,
        );

        // Compute hash using SequentialPairwise method 
        let hash_sequential = compute_poseidon_commitment_hash(
            recipient,
            amount,
            nonce,
            timestamp,
            HashMethod::SequentialPairwise,
        );

        // Ensure hashes are different between methods (they should be)
        assert_ne!(hash_batch, hash_sequential, "Different hash methods should produce different results");

        // Ensure hash is deterministic
        let hash_batch_2 = compute_poseidon_commitment_hash(
            recipient,
            amount,
            nonce,
            timestamp,
            HashMethod::BatchHash,
        );
        assert_eq!(hash_batch, hash_batch_2, "Hash computation should be deterministic");

        // Verify hash is valid felt
        assert_ne!(hash_batch, Felt::ZERO, "Hash should not be zero");
        
        println!("✓ Batch hash: 0x{:x}", hash_batch);
        println!("✓ Sequential hash: 0x{:x}", hash_sequential);
    }

    #[test]
    fn test_nonce_increment_behavior() {
        // Test that different nonces produce different hashes
        let recipient = Felt::from_hex("0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7").unwrap();
        let amount: u128 = 1000000;
        let timestamp: u64 = 1672531200;

        let hash_nonce_0 = compute_poseidon_commitment_hash(
            recipient,
            amount,
            0,
            timestamp,
            HashMethod::BatchHash,
        );

        let hash_nonce_1 = compute_poseidon_commitment_hash(
            recipient,
            amount,
            1,
            timestamp,
            HashMethod::BatchHash,
        );

        assert_ne!(hash_nonce_0, hash_nonce_1, "Different nonces should produce different hashes");
        
        println!("✓ Hash with nonce 0: 0x{:x}", hash_nonce_0);
        println!("✓ Hash with nonce 1: 0x{:x}", hash_nonce_1);
    }
}
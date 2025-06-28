use zeroxbridge_sequencer::utils::{
    compute_commitment_hash,
    compute_commitment_hash_from_burn_data,
    hash_to_hex_string,
    hex_string_to_hash,
    BurnData
};

#[test]
fn test_known_hash_values() {
    // Test case 1: Zero values
    let caller = [0u8; 32];
    let usd_val = 0u128;
    let nonce = 0u64;
    let timestamp = 0u64;

    let hash = compute_commitment_hash(caller, usd_val, nonce, timestamp);
    let hex_hash = hash_to_hex_string(hash);

    // This should produce a deterministic hash for all zeros
    assert_eq!(hex_hash.len(), 64); // 32 bytes = 64 hex characters
    assert!(hex_hash.chars().all(|c| c.is_ascii_hexdigit()));

    // Verify deterministic behavior
    let hash_repeat = compute_commitment_hash(caller, usd_val, nonce, timestamp);
    let hex_hash_repeat = hash_to_hex_string(hash_repeat);
    assert_eq!(hex_hash, hex_hash_repeat);

    // Test case 2: Non-zero values
    let caller = [1u8; 32]; // All bytes set to 1
    let usd_val = 1000u128;
    let nonce = 42u64;
    let timestamp = 1640995200u64; // Jan 1, 2022 00:00:00 UTC

    let hash2 = compute_commitment_hash(caller, usd_val, nonce, timestamp);
    let hex_hash2 = hash_to_hex_string(hash2);

    // Verify it's different from the zero case
    assert_ne!(hex_hash, hex_hash2);
    assert_eq!(hex_hash2.len(), 64); // 32 bytes = 64 hex characters
}

#[test]
fn test_realistic_starknet_address() {
    // Test with a realistic Starknet address (32 bytes)
    let stark_address_hex = "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7";
    let stark_bytes = hex::decode(&stark_address_hex[2..]).unwrap(); // Remove 0x prefix

    let mut caller = [0u8; 32];
    caller[..stark_bytes.len()].copy_from_slice(&stark_bytes);

    let usd_val = 50000u128; // $500.00 in cents
    let nonce = 123u64;
    let timestamp = 1672531200u64; // Jan 1, 2023 00:00:00 UTC

    let hash = compute_commitment_hash(caller, usd_val, nonce, timestamp);
    let hex_hash = hash_to_hex_string(hash);

    // Verify the hash is deterministic
    let hash2 = compute_commitment_hash(caller, usd_val, nonce, timestamp);
    assert_eq!(hash, hash2);

    // Verify hex conversion works both ways
    let parsed_hash = hex_string_to_hash(&hex_hash).unwrap();
    assert_eq!(hash, parsed_hash);
}

#[test]
fn test_edge_cases() {
    // Test with maximum values
    let caller = [0xFFu8; 32]; // All bytes set to 255
    let usd_val = u128::MAX;
    let nonce = u64::MAX;
    let timestamp = u64::MAX;

    let hash = compute_commitment_hash(caller, usd_val, nonce, timestamp);
    assert_eq!(hash.len(), 32);

    // Test with mixed values
    let mut mixed_caller = [0u8; 32];
    for i in 0..32 {
        mixed_caller[i] = (i as u8) * 8; // Create a pattern
    }

    let hash2 = compute_commitment_hash(mixed_caller, 12345u128, 999u64, 1234567890u64);
    assert_ne!(hash, hash2); // Should be different
}

#[test]
fn test_burn_data_consistency() {
    let data1 = BurnData {
        caller: [42u8; 32],
        amount: 75000u128,
        nonce: 456u64,
        time_stamp: 1672617600u64,
    };

    let data2 = BurnData {
        caller: [42u8; 32],
        amount: 75000u128,
        nonce: 456u64,
        time_stamp: 1672617600u64,
    };

    // Same data should produce same hash
    let hash1 = compute_commitment_hash_from_burn_data(&data1);
    let hash2 = compute_commitment_hash_from_burn_data(&data2);
    assert_eq!(hash1, hash2);

    // Direct function call should match struct-based call
    let hash3 = compute_commitment_hash(data1.caller, data1.amount, data1.nonce, data1.time_stamp);
    assert_eq!(hash1, hash3);
}

#[test]
fn test_byte_order_sensitivity() {
    // Test that changing byte order produces different hashes
    let caller = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
                  17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32];

    let hash1 = compute_commitment_hash(caller, 1000u128, 100u64, 200u64);

    // Reverse the caller bytes
    let mut reversed_caller = caller;
    reversed_caller.reverse();
    let hash2 = compute_commitment_hash(reversed_caller, 1000u128, 100u64, 200u64);

    assert_ne!(hash1, hash2);

    // Change just the amount
    let hash3 = compute_commitment_hash(caller, 1001u128, 100u64, 200u64);
    assert_ne!(hash1, hash3);

    // Change just the nonce
    let hash4 = compute_commitment_hash(caller, 1000u128, 101u64, 200u64);
    assert_ne!(hash1, hash4);

    // Change just the timestamp
    let hash5 = compute_commitment_hash(caller, 1000u128, 100u64, 201u64);
    assert_ne!(hash1, hash5);
}

#[test]
fn test_solidity_compatibility_hash() {
    let caller: [u8; 32] = hex::decode("049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7")
        .unwrap()
        .try_into()
        .expect("Expected 32-byte address");
    let usd_val = 1000u128;
    let nonce = 42u64;
    let timestamp = 1640995200u64;

    let hash = compute_commitment_hash(caller, usd_val, nonce, timestamp);
    let hex_hash = hash_to_hex_string(hash);

    let expected = "ca9a61c12b4e40e06f18f72b1f04cc1d8bde43c8ed9b495f8d3c499973d0179e";
    assert_eq!(hex_hash, expected);
}

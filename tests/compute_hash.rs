use zeroxbridge_sequencer::utils::BurnData;

#[test]
fn test_commitment_hash_and_hex() {
    let data = BurnData {
        caller: "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7".to_string(),
        amount: 50000u64,
        nonce: 123u64,
        time_stamp: 1672531200u64,
    };
    let hex_hash = data.hash_to_hex_string();
    assert!(hex_hash.starts_with("0x"));
    assert_eq!(hex_hash.len(), 66); // 0x + 64 hex chars
}

#[test]
fn test_commitment_hash_from_burn_data() {
    let data = BurnData {
        caller: "0x0101010101010101010101010101010101010101010101010101010101010101".to_string(),
        amount: 1000u64,
        nonce: 42u64,
        time_stamp: 1640995200u64,
    };
    let hash1 = data.compute_commitment_hash();
    let hash2 = BurnData::compute_commitment_hash(&data);
    assert_eq!(hash1, hash2);
}

#[test]
fn test_commitment_hash_solidity_compatibility() {
    // These values match the Solidity contract testKeccak()
    let data = BurnData {
        caller: "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7".to_string(),
        amount: 50000u64,
        nonce: 123u64,
        time_stamp: 1672531200u64,
    };
    let hex_hash = data.hash_to_hex_string();
    let expected = "0x2b6876060a11edcc5dde925cda8fad185f34564e35802fa40ee8ead2f9acb06f";
    assert_eq!(hex_hash, expected);
}

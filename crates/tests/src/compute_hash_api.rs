use axum::body::Bytes;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt; // for `oneshot`

use zeroxbridge_sequencer::api::routes::create_router;

use zeroxbridge_sequencer::utils::BurnData;

#[tokio::test]
async fn test_compute_hash_api_success() {
    // Example input matching the Rust hash test
    let burn_data = BurnData {
        caller: "0x0101010101010101010101010101010101010101010101010101010101010101".to_string(),
        amount: 1000u64,
        nonce: 42u64,
        time_stamp: 1640995200u64,
    };
    let hash = burn_data.hash_to_hex_string();
    assert!(hash.starts_with("0x"));
    assert_eq!(hash.len(), 66); // 0x + 64 hex chars
}

#[test]
fn test_compute_hash_api_invalid_pubkey() {
    let invalid_pubkey = "not_a_hex_pubkey";
    let result = BurnData::hex_to_bytes32(invalid_pubkey);
    assert!(result.is_err(), "Expected error for invalid hex pubkey");
}

#[test]
fn test_utils_reference_solidity_compatibility() {
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

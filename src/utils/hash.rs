use sha3::{Digest, Keccak256};

/// Data structure representing the burn data to be hashed
#[derive(Debug, Clone)]
pub struct BurnData {
    pub caller: [u8; 32],      // stark_pubkey (user's starknet address)
    pub amount: u128,          // usd_val (amount in USD being withdrawn)
    pub nonce: u64,            // tx nonce
    pub time_stamp: u64,       // block.timestamp
}

/// Compute keccak256 commitment hash from withdrawal inputs
/// This function replicates Solidity's keccak256(abi.encodePacked(...)) behavior
///
/// The Solidity equivalent is:
/// bytes32 commitmentHash = keccak256(abi.encodePacked(user, usdVal, nonce, block.timestamp));
///
/// # Arguments
/// * `caller` - 32-byte array representing the user's Starknet address
/// * `usd_val` - u128 representing the amount in USD being withdrawn
/// * `nonce` - u64 representing the transaction nonce
/// * `timestamp` - u64 representing the block timestamp
///
/// # Returns
/// * `[u8; 32]` - The 32-byte keccak256 hash
pub fn compute_commitment_hash(caller: [u8; 32], usd_val: u128, nonce: u64, timestamp: u64) -> [u8; 32] {
    let mut hasher = Keccak256::new();

    // Pack data in the same order as Solidity's abi.encodePacked
    // caller (32 bytes)
    hasher.update(&caller);

    // usd_val (16 bytes, big-endian u128)
    hasher.update(&usd_val.to_be_bytes());

    // nonce (8 bytes, big-endian u64)
    hasher.update(&nonce.to_be_bytes());

    // timestamp (8 bytes, big-endian u64)
    hasher.update(&timestamp.to_be_bytes());

    // Finalize and return the hash
    hasher.finalize().into()
}

/// Convenience function that takes a BurnData struct
pub fn compute_commitment_hash_from_burn_data(data: &BurnData) -> [u8; 32] {
    compute_commitment_hash(data.caller, data.amount, data.nonce, data.time_stamp)
}

/// Convert hash bytes to hex string for easy display/comparison
pub fn hash_to_hex_string(hash: [u8; 32]) -> String {
    hex::encode(hash)
}

/// Convert hex string to hash bytes (useful for testing)
pub fn hex_string_to_hash(hex_str: &str) -> Result<[u8; 32], hex::FromHexError> {
    let bytes = hex::decode(hex_str)?;
    if bytes.len() != 32 {
        return Err(hex::FromHexError::InvalidStringLength);
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(hash)
}


pub fn parse_stark_pubkey(hex_str: &str) -> Result<[u8; 32], String> {
    // Remove 0x prefix if present
    let clean_hex = if hex_str.starts_with("0x") || hex_str.starts_with("0X") {
        &hex_str[2..]
    } else {
        hex_str
    };

    // Validate hex string length and characters
    if clean_hex.len() > 64 {
        return Err("Hex string too long (max 64 characters)".to_string());
    }

    if !clean_hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid hex characters".to_string());
    }

    // Decode hex string
    let bytes = hex::decode(clean_hex)
        .map_err(|e| format!("Failed to decode hex: {}", e))?;

    // Convert to 32-byte array (pad with zeros on the left if needed)
    let mut result = [0u8; 32];
    if bytes.len() <= 32 {
        result[32 - bytes.len()..].copy_from_slice(&bytes);
    } else {
        return Err("Decoded bytes exceed 32 bytes".to_string());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_commitment_hash() {
        // Test with sample data
        let caller = [1u8; 32]; // Sample stark pubkey
        let usd_val = 1000u128;
        let nonce = 42u64;
        let timestamp = 1640995200u64; // Jan 1, 2022 00:00:00 UTC

        let hash = compute_commitment_hash(caller, usd_val, nonce, timestamp);
        let hex_hash = hash_to_hex_string(hash);

        // The hash should be deterministic
        assert_eq!(hash.len(), 32);
        assert!(!hex_hash.is_empty());

        // Test that same inputs produce same hash
        let hash2 = compute_commitment_hash(caller, usd_val, nonce, timestamp);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_burn_data_struct() {
        let data = BurnData {
            caller: [2u8; 32],
            amount: 500u128,
            nonce: 123u64,
            time_stamp: 1640995200u64,
        };

        let hash1 = compute_commitment_hash_from_burn_data(&data);
        let hash2 = compute_commitment_hash(data.caller, data.amount, data.nonce, data.time_stamp);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hex_conversion() {
        let hash = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                   0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                   0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
                   0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];

        let hex_str = hash_to_hex_string(hash);
        let parsed_hash = hex_string_to_hash(&hex_str).expect("hex_string_to_hash");

        assert_eq!(hash, parsed_hash);
    }
}

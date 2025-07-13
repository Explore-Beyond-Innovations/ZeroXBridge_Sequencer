use starknet_crypto::{poseidon_hash, Felt, PoseidonHasher};
use sha3::{Digest, Keccak256};

/// Data structure representing the burn data to be hashed
#[derive(Debug, Clone)]
pub struct BurnData {
    pub caller: String,         // stark_pubkey (user's starknet address)
    pub amount: u64,            // usd_val (amount in USD being withdrawn)
    pub nonce: u64,             // tx nonce
    pub time_stamp: u64,        // block.timestamp
}

impl BurnData {
    pub fn new(caller: String, amount: u64, nonce: u64, time_stamp: u64) -> Self {
        Self {
            caller,
            amount,
            nonce,
            time_stamp,
        }
    }

    /// Computes the Keccak256 commitment hash for burn/withdrawal data
    /// This replicates Solidity's keccak256(abi.encodePacked(...)) behavior
    ///
    /// The Solidity equivalent is:
    /// bytes32 commitmentHash = keccak256(abi.encodePacked(user, usdVal, nonce, block.timestamp));
    pub fn compute_commitment_hash(&self) -> [u8; 32] {
        let caller_hex = Self::hex_to_bytes32(&self.caller)
            .expect("Invalid hex string for caller address");
        let packed = Self::encode_packed(&caller_hex, self.amount, self.nonce, self.time_stamp);
        Self::keccak256(&packed)
    }

    /// Convert hash bytes to hex string for easy display/comparison
    pub fn hash_to_hex_string(&self) -> String {
        let hash = self.compute_commitment_hash();
        let hash_str = hex::encode(hash);
        format!("0x{}", hash_str)
    }

    /// Convert hex string to 32-byte array
    pub fn hex_to_bytes32(hex_str: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
        let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(hex_str)?;

        if bytes.len() != 32 {
            return Err(format!("Expected 32 bytes, got {}", bytes.len()).into());
        }

        Ok(bytes.try_into().unwrap())
    }

    /// Convert u64 to 32-byte array (like solidity's uint256)
    fn u64_to_u256_bytes(value: u64) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[24..32].copy_from_slice(&value.to_be_bytes());
        bytes
    }

    /// Encoding (abi.encodePacked equivalent)
    fn encode_packed(stark: &[u8; 32], usd_val: u64, nonce: u64, timestamp: u64) -> Vec<u8> {
        let mut packed_data = Vec::with_capacity(128); // 32 * 4 = 128 bytes

        // Add each component
        packed_data.extend_from_slice(stark);
        packed_data.extend_from_slice(&Self::u64_to_u256_bytes(usd_val));
        packed_data.extend_from_slice(&Self::u64_to_u256_bytes(nonce));
        packed_data.extend_from_slice(&Self::u64_to_u256_bytes(timestamp));

        packed_data
    }

    /// Compute keccak256 hash
    fn keccak256(data: &[u8]) -> [u8; 32] {
        let mut hasher = Keccak256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

#[derive(Debug, Clone)]
pub struct MintData {
    /// Starknet address of the recipient
    pub recipient: Felt,
    /// USD amount to mint
    pub amount: u128,
    /// Transaction nonce
    pub nonce: u64,
    /// Block timestamp
    pub timestamp: u64,
}

impl MintData {
    pub fn new(recipient: Felt, amount: u128, nonce: u64, timestamp: u64) -> Self {
        Self {
            recipient,
            amount,
            nonce,
            timestamp,
        }
    }

    /// Converts MintData to a vector of Felts for hashing
    pub fn to_field_elements(&self) -> Vec<Felt> {
        vec![
            self.recipient,
            Felt::from(self.amount),
            Felt::from(self.nonce),
            Felt::from(self.timestamp),
        ]
    }
}

/// Poseidon hash methods that can be used for computing commitment hashes
pub enum HashMethod {
    /// Uses the stateful hasher to hash all elements at once (recommended for efficiency)
    BatchHash,

    /// Uses sequential pairwise hashing, similar to:
    /// poseidon_hash(poseidon_hash(poseidon_hash(a, b), c), d)
    SequentialPairwise,
}

/// Computes a Poseidon hash over the given inputs to create a deposit commitment hash
/// compatible with Cairo contracts on Starknet.
///
/// This function supports both batch hashing and sequential pairwise hashing to match
/// the approach used by the corresponding Cairo contract. Sequential pairwise hashing
/// uses the pattern: poseidon_hash(poseidon_hash(poseidon_hash(a, b), c), d).
///
/// # Arguments
/// * `recipient` - Starknet address of the recipient (represented as Felt/felt252)
/// * `amount` - USD amount to mint
/// * `nonce` - Transaction nonce
/// * `timestamp` - Block timestamp
/// * `method` - The hashing method to use (batch or sequential pairwise)
///
/// # Returns
/// A `Felt` (felt252) representing the Poseidon hash of the input values
pub fn compute_poseidon_commitment_hash(
    recipient: Felt,
    amount: u128,
    nonce: u64,
    timestamp: u64,
    method: HashMethod,
) -> Felt {
    let mint_data = MintData::new(recipient, amount, nonce, timestamp);
    let field_elements = mint_data.to_field_elements();

    match method {
        HashMethod::BatchHash => {
            let mut hasher = PoseidonHasher::new();

            for element in field_elements {
                hasher.update(element);
            }

            hasher.finalize()
        }
        HashMethod::SequentialPairwise => {
            if field_elements.is_empty() {
                panic!("Cannot compute Poseidon hash on empty input");
            }

            let mut result = field_elements[0];

            for element in field_elements.iter().skip(1) {
                result = poseidon_hash(result, *element);
            }

            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starknet_crypto::poseidon_hash_many;

    #[test]
    fn test_poseidon_hash_consistency() {
        let recipient = Felt::from_dec_str("123456789012345678901234567890").unwrap();
        let amount: u128 = 1000000;
        let nonce: u64 = 42;
        let timestamp: u64 = 1650000000;

        let hash1 = compute_poseidon_commitment_hash(
            recipient,
            amount,
            nonce,
            timestamp,
            HashMethod::BatchHash,
        );
        let hash2 = compute_poseidon_commitment_hash(
            recipient,
            amount,
            nonce,
            timestamp,
            HashMethod::BatchHash,
        );

        assert_eq!(hash1, hash2, "Hash function should be deterministic");
    }

    #[test]
    fn test_poseidon_hash_direct_api() {
        let recipient = Felt::from_dec_str("123456789012345678901234567890").unwrap();
        let amount: u128 = 1000000;
        let nonce: u64 = 42;
        let timestamp: u64 = 1650000000;

        let hash1 = compute_poseidon_commitment_hash(
            recipient,
            amount,
            nonce,
            timestamp,
            HashMethod::BatchHash,
        );

        let elements = vec![
            recipient,
            Felt::from(amount),
            Felt::from(nonce),
            Felt::from(timestamp),
        ];
        let hash2 = poseidon_hash_many(&elements);

        assert_eq!(
            hash1, hash2,
            "Hash calculations should match between methods"
        );
    }

    #[test]
    fn test_poseidon_hash_direct_vs_stateful() {
        let recipient = Felt::from_dec_str("123456789012345678901234567890").unwrap();
        let amount: u128 = 1000000;
        let nonce: u64 = 42;
        let timestamp: u64 = 1650000000;

        let hash1 = compute_poseidon_commitment_hash(
            recipient,
            amount,
            nonce,
            timestamp,
            HashMethod::BatchHash,
        );

        let elements = vec![
            recipient,
            Felt::from(amount),
            Felt::from(nonce),
            Felt::from(timestamp),
        ];
        let hash2 = poseidon_hash_many(&elements);

        assert_eq!(
            hash1, hash2,
            "Stateful hash should match direct hash_many for same inputs"
        );
    }

    #[test]
    fn test_poseidon_sequential_pairwise() {
        let recipient = Felt::from_dec_str("123456789012345678901234567890").unwrap();
        let amount: u128 = 1000000;
        let nonce: u64 = 42;
        let timestamp: u64 = 1650000000;

        let hash1 = compute_poseidon_commitment_hash(
            recipient,
            amount,
            nonce,
            timestamp,
            HashMethod::SequentialPairwise,
        );

        let a = recipient;
        let b = Felt::from(amount);
        let c = Felt::from(nonce);
        let d = Felt::from(timestamp);

        let hash_ab = poseidon_hash(a, b);
        let hash_abc = poseidon_hash(hash_ab, c);
        let hash2 = poseidon_hash(hash_abc, d);

        assert_eq!(
            hash1, hash2,
            "Sequential pairwise hash should match manual calculation"
        );

        let batch_hash = compute_poseidon_commitment_hash(
            recipient,
            amount,
            nonce,
            timestamp,
            HashMethod::BatchHash,
        );

        assert_ne!(
            hash1, batch_hash,
            "Sequential pairwise hash should differ from batch hash"
        );
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
        // Matches the Solidity testKeccak() contract
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
}

use starknet_crypto::{poseidon_hash, Felt, PoseidonHasher};

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
}

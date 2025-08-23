use starknet_crypto::{poseidon_hash, poseidon_hash_many, Felt, PoseidonHasher};

#[tokio::test]
async fn main() {
    println!("Testing Poseidon hash implementation...");

    // Test values
    let recipient =
        Felt::from_hex("0x06ee7c7a561ae5c39e3a2866e8e208ed8ebe45da686e2929622102c80834b771")
            .unwrap();
    let amount = Felt::from(1000000_u128);
    let nonce = Felt::from(42_u64);
    let timestamp = Felt::from(1650000000_u64);

    // Create a new hasher - same as our implementation
    println!("Method 1: Using PoseidonHasher stateful API");
    let mut hasher = PoseidonHasher::new();
    hasher.update(recipient);
    hasher.update(amount);
    hasher.update(nonce);
    hasher.update(timestamp);
    let hash1 = hasher.finalize();
    println!("Hash1: {:?}", hash1);

    // Compare with direct call to poseidon_hash_many - should give the same result
    println!("\nMethod 2: Using poseidon_hash_many");
    let elements = vec![recipient, amount, nonce, timestamp];
    let hash2 = poseidon_hash_many(&elements);
    println!("Hash2: {:?}", hash2);

    // Compare with sequential pairwise hashing - shows compatibility with Cairo's pattern
    println!("\nMethod 3: Using sequential poseidon_hash");
    let hash_1_2 = poseidon_hash(recipient, amount);
    let hash_1_2_3 = poseidon_hash(hash_1_2, nonce);
    let hash3 = poseidon_hash(hash_1_2_3, timestamp);
    println!("Hash3: {:?}", hash3);

    // Verify all methods produce the same hash
    if hash1 == hash2 && hash2 == hash3 {
        println!("\nSUCCESS: All hash methods produce identical results!");
    } else {
        println!("\nFAILURE: Hash methods produce different results!");
        if hash1 != hash2 {
            println!("  Hash1 != Hash2");
        }
        if hash2 != hash3 {
            println!("  Hash2 != Hash3");
        }
    }
}

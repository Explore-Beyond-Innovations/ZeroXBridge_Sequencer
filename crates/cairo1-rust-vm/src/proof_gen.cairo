use poseidon::poseidon_hash_span;

// Function that generates proof data for the sequencer
fn generate_deposit_proof(
    user_address: felt252,
    amount: u256,
    merkle_path: Span<felt252>
) -> felt252 {
    let mut hash_data = array![user_address, amount.low.into(), amount.high.into()];
    
    let mut current_hash = poseidon_hash_span(hash_data.span());
    
    let mut i = 0;
    loop {
        if i >= merkle_path.len() {
            break;
        }
        
        let sibling = *merkle_path.at(i);
        hash_data = array![current_hash, sibling];
        current_hash = poseidon_hash_span(hash_data.span());
        
        i += 1;
    };
    
    current_hash
}

// Function for withdrawal proof verification
fn verify_withdrawal_proof(
    claimed_root: felt252,
    user_address: felt252,
    amount: u256,
    merkle_path: Span<felt252>
) -> bool {
    let computed_root = generate_deposit_proof(user_address, amount, merkle_path);
    computed_root == claimed_root
}

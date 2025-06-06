#!/bin/bash

# setup_cairo_project.sh - Script to set up a sample Cairo project

set -euo pipefail
IFS=$'\n\t'

echo "🏗️  Setting up Cairo project for ZeroXBridge"
echo "============================================"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Create the Cairo project directory
CAIRO_DIR="crates/cairo1-rust-vm"
print_status "Creating Cairo project directory: $CAIRO_DIR"
mkdir -p "$CAIRO_DIR/src"

# Create Scarb.toml
print_status "Creating Scarb.toml..."
cat > "$CAIRO_DIR/Scarb.toml" << 'EOF'
[package]
name = "cairo1"
version = "0.1.0"
edition = "2023_11"

[dependencies]
starknet = ">=2.3.0"

[[target.starknet-contract]]
EOF

# Create a sample Cairo contract and module declarations in one block
print_status "Creating sample Cairo contract and module declarations..."
cat > "$CAIRO_DIR/src/lib.cairo" << 'EOF'
mod proof_gen;

use starknet::ContractAddress;

#[starknet::interface]
trait IZeroXBridge<TContractState> {
    fn get_balance(self: @TContractState, user: ContractAddress) -> u256;
    fn deposit(ref self: TContractState, amount: u256);
    fn withdraw(ref self: TContractState, amount: u256);
    fn get_merkle_root(self: @TContractState) -> felt252;
    fn update_merkle_root(ref self: TContractState, new_root: felt252);
}

#[starknet::contract]
mod ZeroXBridge {
    use super::IZeroXBridge;
    use starknet::{ContractAddress, get_caller_address};
    use starknet::storage::{Map, StorageMapReadAccess, StorageMapWriteAccess};

    #[storage]
    struct Storage {
        balances: Map<ContractAddress, u256>,
        merkle_root: felt252,
        total_deposits: u256,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        Deposit: Deposit,
        Withdrawal: Withdrawal,
        MerkleRootUpdated: MerkleRootUpdated,
    }

    #[derive(Drop, starknet::Event)]
    struct Deposit {
        user: ContractAddress,
        amount: u256,
    }

    #[derive(Drop, starknet::Event)]
    struct Withdrawal {
        user: ContractAddress,
        amount: u256,
    }

    #[derive(Drop, starknet::Event)]
    struct MerkleRootUpdated {
        old_root: felt252,
        new_root: felt252,
    }

    #[abi(embed_v0)]
    impl ZeroXBridgeImpl of IZeroXBridge<ContractState> {
        fn get_balance(self: @ContractState, user: ContractAddress) -> u256 {
            self.balances.read(user)
        }

        fn deposit(ref self: ContractState, amount: u256) {
            let caller = get_caller_address();
            let current_balance = self.balances.read(caller);
            self.balances.write(caller, current_balance + amount);
            
            let total = self.total_deposits.read();
            self.total_deposits.write(total + amount);

            self.emit(Deposit { user: caller, amount });
        }

        fn withdraw(ref self: ContractState, amount: u256) {
            let caller = get_caller_address();
            let current_balance = self.balances.read(caller);
            assert(current_balance >= amount, 'Insufficient balance');
            
            self.balances.write(caller, current_balance - amount);
            
            let total = self.total_deposits.read();
            self.total_deposits.write(total - amount);

            self.emit(Withdrawal { user: caller, amount });
        }

        fn get_merkle_root(self: @ContractState) -> felt252 {
            self.merkle_root.read()
        }

        fn update_merkle_root(ref self: ContractState, new_root: felt252) {
            let old_root = self.merkle_root.read();
            self.merkle_root.write(new_root);
            self.emit(MerkleRootUpdated { old_root, new_root });
        }
    }
}
EOF

print_success "Cairo contract created successfully!"

# Create a simple Cairo function for proof generation
print_status "Creating proof generation module..."
cat > "$CAIRO_DIR/src/proof_gen.cairo" << 'EOF'
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
EOF

print_success "Proof generation module created successfully!"

echo ""
echo "📁 Project structure created:"
echo "$CAIRO_DIR/"
echo "├── Scarb.toml"
echo "└── src/"
echo "    ├── lib.cairo"
echo "    └── proof_gen.cairo"

echo ""
print_status "To test the Cairo project:"
echo "1. cd $CAIRO_DIR"
echo "2. scarb build"
echo "3. Check target/dev/ for generated .sierra.json files"

echo ""
print_status "To test with the Rust integration:"
echo "1. cargo build"
echo "2. cargo test"
echo "3. cargo run --example cairo_build_demo"

print_success "Setup complete! You can now test the Cairo build integration."
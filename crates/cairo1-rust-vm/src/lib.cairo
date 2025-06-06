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

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
        owner: ContractAddress, // Add this line
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
            let new_balance = current_balance.checked_add(amount)
                .expect('Balance overflow on deposit');
            self.balances.write(caller, new_balance);

            let total = self.total_deposits.read();
            let new_total = total.checked_add(amount)
                .expect('Total deposits overflow on deposit');
            self.total_deposits.write(new_total);

            self.emit(Deposit { user: caller, amount });
        }

        fn withdraw(ref self: ContractState, amount: u256) {
            let caller = get_caller_address();
            let current_balance = self.balances.read(caller);
            let new_balance = current_balance.checked_sub(amount)
                .expect('Insufficient balance');
            self.balances.write(caller, new_balance);

            let total = self.total_deposits.read();
            let new_total = total.checked_sub(amount)
                .expect('Total deposits underflow on withdraw');
            self.total_deposits.write(new_total);

            self.emit(Withdrawal { user: caller, amount });
        }

        fn get_merkle_root(self: @ContractState) -> felt252 {
            self.merkle_root.read()
        }

        fn update_merkle_root(ref self: ContractState, new_root: felt252) {
            self.assert_only_owner(); // Add this line for access control
            let old_root = self.merkle_root.read();
            self.merkle_root.write(new_root);
            self.emit(MerkleRootUpdated { old_root, new_root });
        }

        fn assert_only_owner(self: @ContractState) {
            let caller = get_caller_address();
            let owner = self.owner.read();
            assert(caller == owner, 'Only owner can call this function');
        }
    }
}

#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub use self::token_lock::{TokenLockRef, TokenLock};

#[ink::contract]
mod token_lock {
    use token_contract::TokenContractRef;
    use token_contract::PSP22;
    use ink::storage::Mapping as StorageHashMap;
    use ink::prelude::{
        vec::Vec,
    };


    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(
        feature = "std",
        derive(ink::storage::traits::StorageLayout)
    )]
    pub struct TimeLockDetails {
        token_address: AccountId,
        token_owner: AccountId,
        locked_amount: Balance,
        start_time: Timestamp,
        duration_time: Timestamp,
    }
    
    #[ink(storage)]
    pub struct TokenLock {
        tokens: StorageHashMap<AccountId, TokenContractRef>,
        token_lock_details: StorageHashMap<AccountId, TimeLockDetails>
    }

    impl TokenLock {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                tokens: StorageHashMap::default(),
                token_lock_details: StorageHashMap::default(),
            }
        }

        #[ink(message)]
        pub fn create_lock(
            &mut self,
            token_address: AccountId,
            token_owner: AccountId,
            lock_amount: Balance,
            duration_time: Timestamp,
        ) -> AccountId {
            let token_details = TimeLockDetails {
                token_address,
                token_owner,
                locked_amount: lock_amount,
                start_time: self.time_now(),
                duration_time
            };
            self.token_lock_details.insert(token_address, &token_details);
            
            let _data = Vec::new();
            let mut token_contract: TokenContractRef = ink::env::call::FromAccountId::from_account_id(token_address);
            let _= token_contract.transfer_from(self.env().caller(), self.env().account_id(), lock_amount, _data);

            token_address
        }

        #[ink(message)]
        pub fn release_lock(
            &mut self,
            token_address: AccountId,
        ) -> AccountId {
            let token_details = self.token_lock_details.get(token_address).unwrap();
            assert!(token_details.start_time.checked_add(token_details.duration_time) > Some(self.time_now()), "Lock duration isn't expired");

            let _data = Vec::new();
            let mut token_contract: TokenContractRef = ink::env::call::FromAccountId::from_account_id(token_address);
            let _ = token_contract.transfer_from(self.env().account_id(), token_details.token_owner, token_details.locked_amount, _data);
            token_address
        }

        /// Check contract balance
        #[ink(message)]
        pub fn contract_balance(&self) -> u128 {
            self.env().balance()
        }

        pub fn time_now(&self) -> Timestamp {
            self.env().block_timestamp()
        }
    }
}
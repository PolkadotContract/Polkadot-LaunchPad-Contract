#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod token_factory {
    use ink::prelude::string::String;
    use ink::storage::Mapping;
    use erc20::Erc20Ref;

    #[ink(storage)]
    pub struct TokenFactory {
        tokens: Mapping<AccountId, Erc20Ref>,
        owner: AccountId,
        fee: Balance, // Fee for creating a token
    }

    impl TokenFactory {
        #[ink(constructor)]
        pub fn new(fee: Balance) -> Self {
            let caller = Self::env().caller();
            Self {
                tokens: Mapping::new(),
                owner: caller,
                fee,
            }
        }

        #[ink(message)]
        pub fn create_token(
            &mut self,
            name: String,
            symbol: String,
            initial_supply: Balance,
            logo_uri: String, // New parameter for logo URI
        ) -> AccountId {
            let caller = self.env().caller();
            if caller != self.owner {
                panic!("Only the owner can create tokens");
            }

            let transferred_fee = self.env().transferred_value();
            if transferred_fee < self.fee {
                panic!("Insufficient fee");
            }

            let new_token = Erc20Ref::new(initial_supply);
            // Get contract address.
            let callee = ink::env::account_id::<ink::env::DefaultEnvironment>();
            // let token_address = new_token.env().account_id();
            let finalized_token = new_token.build(); 
            self.tokens.insert(callee, finalized_token);

            callee
        }

        #[ink(message)]
        pub fn get_token_info(&self, token_address: AccountId) -> (Balance) {
            let token = self.tokens.get(&token_address).expect("Token not found");
            (token.total_supply())
        }
    }
}

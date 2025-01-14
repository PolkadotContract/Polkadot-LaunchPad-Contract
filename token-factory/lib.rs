#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub use self::token_factory::{TokenFactoryRef, TokenFactory};

#[ink::contract]
mod token_factory {
    use token_contract::TokenContractRef;
    use token_contract::PSP22Metadata;
    use token_contract::PSP22;
    use ink::storage::Mapping as StorageHashMap;
    use ink::prelude::string::String;
    use ink::ToAccountId;
    use ink::env::hash::Blake2x256;

    #[ink(storage)]
    pub struct TokenFactory {
        tokens: StorageHashMap<AccountId, TokenContractRef>,
        owner: AccountId,
        fee: Balance,
        other_contract_code_hash: Hash,
    }

    impl TokenFactory {
        #[ink(constructor)]
        pub fn new(other_contract_code_hash: Hash, fee: Balance) -> Self {
            let caller = Self::env().caller();

            Self {
                tokens: StorageHashMap::default(),
                owner: caller,
                fee,
                other_contract_code_hash,
            }
        }

        #[ink(message)]
        pub fn get_fee(&self) -> Balance {
            self.fee
        }

        #[ink(message, payable)]
        pub fn create_token(
            &mut self,
            initial_supply: Balance,
            name: String,
            symbol: String,
            decimals: u8,
            logo_uri: String, // New parameter for logo URI
        ) -> AccountId {
            let transferred_fee = self.env().transferred_value();
            if transferred_fee < self.fee {
                panic!("Insufficient fee");
            }

            let timestamp = Self::env().block_timestamp();
            let input = timestamp.to_le_bytes();
            

            let salt = Self::env().hash_encoded::<Blake2x256, _>(&input);

            let token_contract = TokenContractRef::new(initial_supply, Some(name), Some(symbol), decimals, Some(logo_uri), self.env().caller())
                .code_hash(self.other_contract_code_hash)
                .endowment(0)
                .salt_bytes(&salt.as_ref()[..4])
                .instantiate();


            let token_address = token_contract.to_account_id();

            self.tokens.insert(token_address, &token_contract);

            token_address
        }

        #[ink(message)]
        pub fn get_token_balance(&self, token_address: AccountId, account: AccountId) -> Balance {
            let token = self.tokens.get(token_address).expect("Token not found");

            // Call the `balance_of` method of the token contract
            token.balance_of(account)
        }

        #[ink(message)]
        pub fn withdraw(&mut self, amount: u128) -> Result<(), String> {
            // Check if the caller is the owner (optional: remove or modify)
            let caller = self.env().caller();
            if caller != self.owner {
                return Err("Only the owner can withdraw".into());
            }

            // Ensure the contract has enough balance
            let contract_balance = self.env().balance();
            if contract_balance < amount {
                return Err("Insufficient balance".into());
            }

            // Perform the transfer of native tokens from the contract to the caller
            let destination = caller;
            let _ =  Self::env().transfer(destination, amount);

            Ok(())
        }

        /// Check contract balance
        #[ink(message)]
        pub fn contract_balance(&self) -> u128 {
            self.env().balance()
        }

        #[ink(message)]
        pub fn get_caller(&self) -> AccountId {
            self.env().caller() // Get the caller's address
        }

        #[ink(message)]
        pub fn get_token_info(&self, token_address: AccountId) -> (String, String, u8, String) {
            let token = self.tokens.get(token_address).expect("Token not found");
            (token.token_name().expect("undefined"), token.token_symbol().expect("undefined"), token.token_decimals(), token.token_logo_uri().expect("undefined"))
        }
    }

    // #[cfg(test)]
    // mod tests {
    //     use super::*;
    //     #[ink::test]
    //     fn new_works() {
    //     }
    
    // }

    

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use ink_e2e::ContractsBackend;
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// The default constructor does its job.
        #[ink_e2e::test]
        async fn init<Client: E2EBackend>(
            mut client: Client,
        ) -> E2EResult<()> {
            println!("TEST OF TOKEN-FACTORY CONTRACT");
            let accounts =
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let token_contract_code = client
                .upload("token-contract", &ink_e2e::alice())
                .submit()
                .await
                .expect("other_contract upload failed");

            const FEE_LIMIT: u128 = 500_000_000;

            let mut constructor = TokenFactoryRef::new(
                token_contract_code.code_hash,
                FEE_LIMIT
            );

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(10_000_000_000);
            println!("Creating token factory contract...");
            let contract = client
                .instantiate("token-factory", &ink_e2e::alice(), &mut constructor)
                .submit()
                .await
                .expect("token-factory instantiate failed");
            println!("Token factory contract created.");
            let mut call_builder = contract.call_builder::<TokenFactory>();
            println!("Calling create_token...");
            let fee_call = call_builder.get_fee();

            println!("Creating token factory contract...");
            let result = client
                .call(&ink_e2e::alice(), &fee_call)
                .submit()
                .await
                .expect("Calling `create_token` failed")
                .return_value();
            assert_eq!(result, FEE_LIMIT);
            
            let value_to_send = 500_000_000;

            let create_call = call_builder.create_token(
                100_000_000,
                "my_token".to_string(),
                "MT".to_string(),
                4,
                "Logo".to_string()
            );
            
            let token_address = client
            .call(&ink_e2e::alice(), &create_call)
            .value(value_to_send)
            .submit()
            .await
            .expect("Calling `create_token` failed")
            .return_value();
            
            println!("Token {:?} created: supply {:?}", "my_token", 100_000_000);
            let get_caller_call = call_builder.get_caller();
            let caller_address = client
                .call(&ink_e2e::alice(), &get_caller_call)
                .submit()
                .await
                .expect("Calling `get_caller` failed")
                .return_value();

            // let get_balance_call = call_builder.get_token_info(token_address);
            // let balance = client
            //     .call(&ink_e2e::alice(), &get_balance_call)
            //     .submit()
            //     .await
            //     .expect("Calling `get_balance` failed")
            //     .return_value();

            println!("FINISEHD TEST OF TOKEN-FACTORY CONTRACT");
                
            Ok(())
        }
    }
}
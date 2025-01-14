#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub use self::bonding_curve_presale::BondingCurvePresale;

#[ink::contract]
mod bonding_curve_presale {
    use token_factory::TokenFactoryRef;
    use token_lock::TokenLockRef;
    use ink::storage::{
        Mapping as StorageHashMap
    };
    use ink::prelude::string::String;
    use ink::prelude::{
        vec::Vec,
    };

    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(
        feature = "std",
        derive(ink::storage::traits::StorageLayout)
    )]
    pub struct Project {
        token: AccountId,
        total_presale_token_amount: Balance,
        presaled_amount: Balance,
        intended_raise_amount: Balance,
        raised_amount: Balance,
        start_time: Timestamp,
        end_time: Timestamp,
        creator: AccountId,
        contributors: Vec<AccountId>,
        is_finished: bool,
        is_successful: bool,
    }
    #[ink(storage)]
    pub struct BondingCurvePresale {
        projects: StorageHashMap<u32, Project>,
        last_project_id: u32,
        token_factory: TokenFactoryRef,
        token_lock: TokenLockRef,
    }

    impl BondingCurvePresale {
        #[ink(constructor)]
        pub fn new(token_factory_address: AccountId, token_lock_address: AccountId) -> Self {
            let token_factory: TokenFactoryRef = ink::env::call::FromAccountId::from_account_id(token_factory_address);
            let token_lock: TokenLockRef = ink::env::call::FromAccountId::from_account_id(token_lock_address);
            
            Self { token_factory, token_lock, last_project_id: 0, projects: StorageHashMap::new()}
        }

        #[ink(message, payable)]
        #[allow(clippy::too_many_arguments)]
        pub fn create_presale(
            &mut self,
            max_supply: Balance,
            name: String,
            symbol: String,
            decimals: u8,
            logo_uri: String,
            lock_amount: Balance,
            lock_duartion: Timestamp,
            intended_raise_amount: Balance,
            start_time: Timestamp,
            end_time: Timestamp,
        ) -> Project {
            let project_id = self.last_project_id.checked_add(1).expect("Overflow detected in project_id calculation");
            self.last_project_id = project_id;

            let token_address = self.token_factory
            // .call()
            // .transferred_value(self.env().transferred_value()) // Send the value
            .create_token(max_supply, name, symbol, decimals, logo_uri);
            
            let _ = self.token_lock.create_lock(token_address, self.env().caller(), lock_amount, lock_duartion);

            let project = Project {
                token: token_address,
                total_presale_token_amount: max_supply.checked_sub(lock_amount).expect("error"),
                presaled_amount: 0,
                intended_raise_amount,
                raised_amount: 0,
                start_time,
                end_time,
                creator: self.env().caller(),
                contributors: Vec::new(),
                is_finished: false,
                is_successful: false,
            };

            self.projects.insert(project_id, &project);

            project
        }


        #[ink(message, payable)]
        pub fn join_project_presale(
            &mut self,
            project_id: u32,
            buy_token_amount: Balance,
        ) -> Project {
            let caller = self.env().caller();
            let mut project = self.projects.get(project_id).expect("Project not found");

            assert!(project.start_time <= self.time_now(), "Presale not started");
            assert!(project.end_time > self.time_now(), "Presale ended");

            let cost = self.calculate_price(project.presaled_amount, buy_token_amount);
            assert!(cost < self.env().transferred_value(), "Insufficient payment");
            assert!(project.presaled_amount.checked_add(buy_token_amount) < Some(project.total_presale_token_amount), "Insufficient amount");

            project.presaled_amount = project.presaled_amount.checked_add(buy_token_amount).expect("Invalid Operation");
            project.raised_amount = project.raised_amount.checked_add(cost).expect("Invalid Operation");
            project.contributors.push(caller);

            project
        }

        #[ink(message)]
        pub fn finish_presale(
            &mut self,
            project_id: u32,
        ) -> Project {
            let mut project = self.projects.get(project_id).expect("Project not found");

            assert!(project.end_time > self.time_now(), "Presale not finished");
            project.is_finished = true;
            project.is_successful = match (
                project.intended_raise_amount.checked_mul(3),
                project.raised_amount.checked_mul(10),
            ) {
                (Some(intended), Some(raised)) => intended <= raised,
                _ => {
                    // Handle overflow explicitly, for example:
                    false
                },
            };

            project
            // Add more logic here
        }

        #[ink(message)]
        pub fn calculate_price(
            &self,
            presaled_amount: Balance,
            buy_token_amount: Balance,
        ) -> Balance {
            let k: Balance = 1;
            let c: Balance = 0;

            let current_price = k.checked_mul(presaled_amount).unwrap_or(0).checked_add(c).unwrap_or(0);
            let next_price = k
                .checked_mul(presaled_amount.checked_add(buy_token_amount).unwrap_or(0))
                .unwrap_or(0)
                .checked_add(c)
                .unwrap_or(0);

            current_price
                .checked_add(next_price)
                .unwrap_or(0)
                .checked_mul(buy_token_amount)
                .unwrap_or(0)
                .checked_div(2)
                .unwrap_or(0)
        }

        #[ink(message)]
        pub fn time_now(&self) -> Timestamp {
            self.env().block_timestamp()
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod tests {
        use super::*;
        use ink_e2e::ContractsBackend;
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test]
        async fn create_presale_works<Client: E2EBackend>(mut client: Client,) -> E2EResult<()> {
            println!("TEST OF CREATING PRESALE");
            let accounts =
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.alice);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000_000);
            println!("Creating token-factory contract");
            let token_contract_code = client
                .upload("token-contract", &ink_e2e::alice())
                .submit()
                .await
                .expect("other_contract upload failed");

            const FEE_LIMIT: u128 = 0;

            let mut token_factory_constructor = TokenFactoryRef::new(token_contract_code.code_hash, FEE_LIMIT);
            let token_factory = client
                .instantiate("token-factory", &ink_e2e::alice(), &mut token_factory_constructor)
                .submit()
                .await
                .expect("token-factory instantiate failed");
            println!("Created token-factory contract");
            let token_factory_address = token_factory.account_id;
            println!("Creating token-lock contract");
            let mut token_lock_constructor = TokenLockRef::new();
            let token_lock = client
                .instantiate("token-lock", &ink_e2e::alice(), &mut token_lock_constructor)
                .submit()
                .await
                .expect("token-lock instantiate failed");

            let token_lock_address = token_lock.account_id;

            let mut constructor = BondingCurvePresaleRef::new(token_factory_address, token_lock_address);
            let contract = client
                .instantiate("bonding-curve-presale", &ink_e2e::alice(), &mut constructor)
                .submit()
                .await
                .expect("bonding-curve-presale instantiate failed");
            println!("Created token-lock contract");
            let mut call_builder = contract.call_builder::<BondingCurvePresale>();
            println!("Creating presale...");
            let time_now = call_builder.time_now();
            let result = client
                .call(&ink_e2e::alice(), &time_now)
                .submit()
                .await
                .expect("Calling `time_now` failed")
                .return_value();

            let start_time = result;
            let end_time = result + 100000000;
            let create_presale = call_builder.create_presale(
                1_000_000,
                String::from("TestToken"),
                String::from("TTK"),
                18,
                String::from("https://logo.uri"),
                100_000,
                3600,
                900_000,
                start_time,
                end_time,
            );

            let result = client
                .call(&ink_e2e::alice(), &create_presale)
                .value(FEE_LIMIT)
                .submit()
                .await
                .expect("Calling `create_presale` failed")
                .return_value();
            println!("Created presale");

            assert_eq!(result.total_presale_token_amount, 900_000);
            assert_eq!(result.intended_raise_amount, 900_000);
            println!("FINISEHD TEST OF CREATING PRESALE");
            Ok(())
        }

        #[ink_e2e::test]
        async fn join_project_presale_works<Client: E2EBackend>(mut client: Client,) -> E2EResult<()> {
            println!("TEST OF JOINNING PRESALE");
            let accounts =
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            println!("Creating token-factory contract");
            let token_contract_code = client
                .upload("token-contract", &ink_e2e::alice())
                .submit()
                .await
                .expect("other_contract upload failed");

            const FEE_LIMIT: u128 = 0;

            let mut token_factory_constructor = TokenFactoryRef::new(token_contract_code.code_hash, FEE_LIMIT);
            let token_factory = client
                .instantiate("token-factory", &ink_e2e::alice(), &mut token_factory_constructor)
                .submit()
                .await
                .expect("token-factory instantiate failed");
            println!("Created token-factory contract");
            let token_factory_address = token_factory.account_id;
            println!("Creating token-lock contract");
            let mut token_lock_constructor = TokenLockRef::new();
            let token_lock = client
                .instantiate("token-lock", &ink_e2e::alice(), &mut token_lock_constructor)
                .submit()
                .await
                .expect("token-lock instantiate failed");

            let token_lock_address = token_lock.account_id;
            
            let mut constructor = BondingCurvePresaleRef::new(token_factory_address, token_lock_address);
            let contract = client
                .instantiate("bonding-curve-presale", &ink_e2e::alice(), &mut constructor)
                .submit()
                .await
                .expect("bonding-curve-presale instantiate failed");
            println!("Created token-lock contract");
            let mut call_builder = contract.call_builder::<BondingCurvePresale>();
            println!("Creating presale...");
            let time_now = call_builder.time_now();
            let result = client
                .call(&ink_e2e::alice(), &time_now)
                .submit()
                .await
                .expect("Calling `time_now` failed")
                .return_value();

            let start_time = result;
            let end_time = result + 100000000;

            let create_presale = call_builder.create_presale(
                1_000_000,
                String::from("TestToken"),
                String::from("TTK"),
                18,
                String::from("https://logo.uri"),
                100_000,
                3600,
                900_000,
                start_time,
                end_time,
            );

            let result = client
                .call(&ink_e2e::alice(), &create_presale)
                .value(FEE_LIMIT)
                .submit()
                .await
                .expect("Calling `create_presale` failed")
                .return_value();
            println!("Created presale");
            assert_eq!(result.total_presale_token_amount, 900_000);
            assert_eq!(result.intended_raise_amount, 900_000);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(10_000_000_000);
            println!("Joining to presale...");
            let calculate_price = call_builder.calculate_price(result.presaled_amount, 100);
            client
                .call(&ink_e2e::charlie(), &calculate_price)
                .submit()
                .await
                .expect("Calling `calculate_price` failed")
                .return_value();

            let join_presale = call_builder.join_project_presale(1, 100);

            let result = client
                .call(&ink_e2e::charlie(), &join_presale)
                .value(100000)
                .submit()
                .await
                .expect("Calling `join_presale` failed")
                .return_value();
            println!("Joinned to presale bought {:?} tokens", 100);
            // let project = contract.projects.get(1).expect("Project should exist");
            assert_eq!(result.presaled_amount, 100);
            assert_eq!(result.contributors.len(), 1);
            // assert_eq!(result.contributors[0], accounts.charlie);
            println!("FINISHED TEST OF JOINING PRESALE");
            Ok(())
        }

        #[ink_e2e::test]
        async fn finish_presale_works<Client: E2EBackend>(mut client: Client,) -> E2EResult<()> {
            println!("TEST OF FINISHING PRESALE");
            let accounts =
                ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            println!("Creating token-factory contract");
            let token_contract_code = client
            .upload("token-contract", &ink_e2e::alice())
            .submit()
            .await
            .expect("other_contract upload failed");

            const FEE_LIMIT: u128 = 0;

            let mut token_factory_constructor = TokenFactoryRef::new(token_contract_code.code_hash, FEE_LIMIT);
            let token_factory = client
                .instantiate("token-factory", &ink_e2e::alice(), &mut token_factory_constructor)
                .submit()
                .await
                .expect("token-factory instantiate failed");
            println!("Created token-factory contract");
            let token_factory_address = token_factory.account_id;
            println!("Creating token-lock contract");   
            let mut token_lock_constructor = TokenLockRef::new();
            let token_lock = client
                .instantiate("token-lock", &ink_e2e::alice(), &mut token_lock_constructor)
                .submit()
                .await
                .expect("token-lock instantiate failed");

            let token_lock_address = token_lock.account_id;

            let mut constructor = BondingCurvePresaleRef::new(token_factory_address, token_lock_address);
            let contract = client
                .instantiate("bonding-curve-presale", &ink_e2e::alice(), &mut constructor)
                .submit()
                .await
                .expect("bonding-curve-presale instantiate failed");
            println!("Created token-lock contract");
            let mut call_builder = contract.call_builder::<BondingCurvePresale>();
            println!("Creating presale...");
            let time_now = call_builder.time_now();
            let result = client
                .call(&ink_e2e::alice(), &time_now)
                .submit()
                .await
                .expect("Calling `time_now` failed")
                .return_value();

            let start_time = result;
            let end_time = result + 100000000;

            let create_presale = call_builder.create_presale(
                1_000_000,
                String::from("TestToken"),
                String::from("TTK"),
                18,
                String::from("https://logo.uri"),
                100_000,
                3600,
                900_000,
                start_time,
                end_time,
            );

            let result = client
                .call(&ink_e2e::alice(), &create_presale)
                .value(FEE_LIMIT)
                .submit()
                .await
                .expect("Calling `create_presale` failed")
                .return_value();
            println!("Created presale");
            assert_eq!(result.total_presale_token_amount, 900_000);
            assert_eq!(result.intended_raise_amount, 900_000);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            ink::env::test::set_value_transferred::<ink::env::DefaultEnvironment>(1_000_000);
            println!("Joining to presale...");
            let calculate_price = call_builder.calculate_price(result.presaled_amount, 100);
            client
                .call(&ink_e2e::alice(), &calculate_price)
                .submit()
                .await
                .expect("Calling `calculate_price` failed")
                .return_value();

            let join_presale = call_builder.join_project_presale(1, 100);

            let result = client
                .call(&ink_e2e::charlie(), &join_presale)
                .value(100000)
                .submit()
                .await
                .expect("Calling `join_presale` failed")
                .return_value();

            // let project = contract.projects.get(1).expect("Project should exist");
            assert_eq!(result.presaled_amount, 100);
            assert_eq!(result.contributors.len(), 1);
            // assert_eq!(result.contributors[0], accounts.charlie);
            println!("Joinned to presale bought {:?} tokens", 100);
            let finish_presale = call_builder.finish_presale(1);
            println!("Finishing presale");
            let result = client
                .call(&ink_e2e::alice(), &finish_presale)
                .submit()
                .await
                .expect("Calling `finish_presale` failed")
                .return_value();
            println!("Finished presale");
            // let project = contract.projects.get(1).expect("Project should exist");
            assert!(result.is_finished);
            assert!(!result.is_successful);
            println!("FINISHED TEST OF FINISHING PRESALE");
            Ok(())
        }
    }

}

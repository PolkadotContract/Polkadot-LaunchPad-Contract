#![cfg_attr(not(feature = "std"), no_std)]

use ink::prelude::vec::Vec;
use ink::storage::Mapping;

#[ink::contract]
mod bonding_curve_presale {
    use super::*;
    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.

    const DECIMALS: u128 = 1e18 as u128;
    const PRICE_CHANGE_SLOPE: u128 = 0.01e18 as u128;
    const BASE_PRICE: u128 = 0.01e18 as u128;
    const LOCK_PERIOD: u128 = 6 * 30 * 24 * 60 * 60; // 6 months in seconds
    const LOCK_PERCENTAGE: u128 = 10e16 as u128; // 10%

    #[ink(storage)]
    pub struct BondingCurvePresale {
        /// Stores a single `bool` value on the storage.
        projects: Mapping<u32, Project>,
        last_project_id: u32,
        tokens_owed_to_contributor: Mapping<(u128, AccountId), u128>,
        fee_collector: AccountId,
        successful_end_fee: u128,
    }

    #[derive(Clone)]
    #[cfg_attr(
        feature = "std",
        derive(Debug, PartialEq, Eq, ink::storage::traits::StorageLayout)
    )]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum ProjectStatus {
        Pending,
        Success,
        Failed,
    }

    #[derive(Clone)]
    #[cfg_attr(
        feature = "std",
        derive(Debug, PartialEq, Eq, ink::storage::traits::StorageLayout)
    )]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub struct Project {
        token: AccountId,
        initial_token_amount: Balance,
        raised: Balance,
        start_time: u64,
        end_time: u64,
        creator: AccountId,
        contributors: Vec<AccountId>,
        status: ProjectStatus,
        price_after_failure: u64,
        creator_claimed_locked_tokens: bool,
    }


    impl BondingCurvePresale {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(fee_collector: AccountId, successful_end_fee: u128) -> Self {
            Self {
                projects: Mapping::new(),
                last_project_id: 0,
                tokens_owed_to_contributor: Mapping::new(),
                fee_collector,
                successful_end_fee,
            }
        }

        #[ink(message)]
        pub fn create_presale(
            &mut self,
            token: AccountId,
            initial_token_amount: u128,
            start_time: u64,
            end_time: u64,
        ) {
            assert!(start_time > Self::env().block_timestamp(), "Start time must be in the future.");
            assert!(end_time > start_time, "End time must be after start time.");
            assert!(initial_token_amount % 2 == 0, "Initial token amount must be even.");

            self.last_project_id += 1;
            // create new token here

            ////////////////////////
            let project = Project {
                token,
                initial_token_amount,
                raised: 0,
                start_time,
                end_time,
                creator: Self::env().caller(),
                contributors: Vec::new(),
                status: ProjectStatus::Pending,
                price_after_failure: 0,
                creator_claimed_locked_tokens: false,
            };

            self.projects.insert(self.last_project_id, &project);
        }

        #[ink(message)]
        pub fn join_project_presale(
            &mut self,
            project_id: u32,
            expected_token_amount: Balance,
        ) {
            let caller = self.env().caller();
            let mut project = self.projects.get(&project_id).expect("Project not found");
            
            // Check various conditions
            assert!(project.status == ProjectStatus::Pending, "Presale already ended");
            assert!(project.start_time <= self.env().block_timestamp(), "Presale not started");
            assert!(project.end_time > self.env().block_timestamp(), "Presale ended");

            // Calculate the token amount and update the project
            let mut token_amount = self.calculate_buy_amount(project.initial_token_amount, project.raised);
            
            // Ensure user is not contributing more than allowed
            let max_tokens = project.initial_token_amount / 2;
            let remaining_tokens = max_tokens - project.raised;
            if token_amount > remaining_tokens {
                token_amount = remaining_tokens;
            }
            assert!(token_amount >= expected_token_amount, "Lack of token");

            // Update the project and contributor's tokens
            project.raised += token_amount;
            if !project.contributors.contains(&caller) {
                project.contributors.push(caller);
            }
            // here update contributors tokens

            //////////////////////////////////
            // Emit event
            self.env().emit_event(UserJoinedProject {
                project_id,
                contributor: caller,
                token_amount,
            });
        }

        #[ink(message)]
        pub fn leave_ongoing_project_presale(
            &mut self,
            id: u32,
            expected_eth_amount: Balance,
        ) {
            // Ensure project ID is valid
            let mut project = self.projects.get(&id).expect("Project not found");
            
            // Ensure project status is Pending
            assert!(project.status == ProjectStatus::Pending, "Presale already ended");
            
            // Ensure the project has started
            assert!(project.start_time <= self.env().block_timestamp(), "Presale not started");
            
            // Ensure the project has not ended
            assert!(project.end_time > self.env().block_timestamp(), "Presale ended");

            
            // Get the token amount owed to the caller
            // let caller = self.env().caller();
            // let token_amount = *self.tokens_owed_to_contributor.get(&(id, caller)).unwrap_or(&0);
            
            // // Check if the token amount is greater than zero
            // assert!(token_amount > 0, "No tokens owed to the caller");
            
            // // Check if the user has enough token balance
            // let user_balance = self.token_balances.get(&(id, caller)).unwrap_or(&0);
            // assert!(*user_balance > token_amount, "Insufficient token balance");
            
            // // Calculate the ETH amount the user should receive
            // let old_supply = project.initial_token_amount - self.token_supply.get(&id).unwrap_or(&0);
            // let eth_amount = self.calculate_sell_amount(token_amount, *old_supply);
            
            // // Ensure the ETH amount is not less than the expected amount
            // if eth_amount < expected_eth_amount {
            //     return Err("ETH amount is less than expected".into());
            // }
            
            // // Update the project's raised ETH amount
            // self.projects.get_mut(&id).unwrap().raised -= eth_amount;
            
            // // Reset the tokens owed to the contributor
            // self.tokens_owed_to_contributor.insert((id, caller), 0);
            
            // // Transfer the tokens back to the contract
            // self.token_balances.insert((id, caller), user_balance - token_amount);
            // let contract_balance = self.token_balances.get(&(id, self.env().account_id())).unwrap_or(&0);
            // self.token_balances.insert((id, self.env().account_id()), contract_balance + token_amount);
            
            // // Send ETH to the user
            // self.env()
            //     .transfer(caller, eth_amount)
            //     .map_err(|_| "Failed to send ETH to the user")?;
            
            // Emit an event
            // self.env().emit_event(UserLeftPendingProject {
            //     id,
            //     contributor: caller,
            //     token_amount,
            //     eth_amount,
            // });
            
        }

        #[ink(message)]
        pub fn end_presale(&mut self, id: u32) {
            let project = self.projects.get(&id).expect("Project does not exist.");
            assert!(
                project.end_time < Self::env().block_timestamp(),
                "Project has not ended yet."
            );

            let is_successful = project.raised >= self.get_soft_cap(id);
            let mut updated_project = project.clone();
            updated_project.status = if is_successful {
                ProjectStatus::Success
            } else {
                ProjectStatus::Failed
            };

            if is_successful {
                // Logic for successful presale
            } else {
                // Logic for failed presale
            }

            self.projects.insert(&id, &updated_project);
        }

        #[ink(message)]
        pub fn calculate_buy_amount(
            &self,
            total_supply: Balance,
            total_raised: Balance,
        ) -> Balance {
            // Implement bonding curve logic here, similar to Solidity's
            total_raised * 10u128.pow(18) / total_supply
        }

        fn get_soft_cap(&self, id: u32) -> u128 {
            let project = self.projects.get(&id).expect("Project does not exist.");
            project.initial_token_amount * 3 / 10 //30%
        }
    }

    #[ink(event)]
    pub struct UserJoinedProject {
        #[ink(topic)]
        project_id: u32,
        #[ink(topic)]
        contributor: AccountId,
        token_amount: Balance,
    }

    #[ink(event)]
    pub struct UserLeftPendingProject {
        #[ink(topic)]
        project_id: u32,
        #[ink(topic)]
        contributor: AccountId,
        token_amount: Balance,
        eth_amount: Balance
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

       
    }
}

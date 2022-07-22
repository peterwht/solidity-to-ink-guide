//! # Vesting Wallet
//! 
//! Based on https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/finance/VestingWallet.sol
//! 
//! ## Overview
//! This contract handles the vesting of the local chain currency for a given beneficiary. 
//! The vesting period can be customized, but is currently set to a linear schedule.
//! The schedule is based on a start timestamp, and a duration (in seconds). 

#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod vesting_wallet {

    use ink_storage::{
        traits::SpreadAllocate,
    };

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct VestingWallet {
        released: Balance,
        beneficiary: AccountId,
        start: Timestamp,
        duration: u64,
    }

    /// event for when a new payee is added
    #[ink(event)]
    pub struct TokensReleased{
        #[ink(topic)]
        amount: Balance,
    }

    impl VestingWallet {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(beneficiary: AccountId, start: Timestamp, duration_seconds: u64) -> Self {
            Self {
                released: 0,
                beneficiary: beneficiary,
                start: start,
                duration: duration_seconds,
            }
        }

        #[ink(message)]
        pub fn beneficiary(&self) -> AccountId {
            self.beneficiary
        }

        #[ink(message)]
        pub fn start(&self) -> Timestamp {
            self.start
        }

        #[ink(message)]
        pub fn duration(&self) -> u64 {
            self.start
        }

        #[ink(message)]
        pub fn released(&self) -> Balance {
            self.released
        }

        #[ink(message)]
        pub fn release(&mut self) {
            let releasable = self.vested_amount(self.env().block_timestamp()) - self.released;
            self.released += releasable;

            self.env().emit_event(TokensReleased {
                amount: releasable,
            });

            // transfer the payment into the payee's account
            if self.env().transfer(self.beneficiary, releasable).is_err() {
                panic!("requested transfer failed")
            }
        }

        #[ink(message)]
        pub fn vested_amount(&self, timestamp: Timestamp) -> Balance {
            self.vesting_schedule(self.env().balance() + self.released, timestamp)
        }

        fn vesting_schedule(&self, total_allocation: Balance, timestamp: Timestamp) -> Balance {
            if timestamp < self.start {
                return 0
            }else if timestamp > self.start + self.duration {
                return total_allocation;
            }else{
                return (total_allocation * (timestamp - self.start) as u128) / self.duration as u128;   
            }
        }
        

    }

    #[cfg(test)]
    mod tests {
    }
}

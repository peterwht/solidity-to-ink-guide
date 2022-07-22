//! # Payment Splitter
//! 
//! Based on https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/finance/PaymentSplitter.sol
//! 
//! ## Overview
//! This contract allows payments (based on the local chain currency) to be split among a group of accounts.
//! 
//! Every ink! contract has an associated address (an `AccountID`). This address can be paid like any 
//! normal address. Payments sent to this contract address will be held by the contract (not automatically distributed).
//! Shareholders are able to request that their portion of the funds are `released` (tranferred) from the contract
//! into their own account. 
//! 
//! Contract instantiation requires a list of `payees` and a list of `shares`. `payees` and `shares` need to have the same length
//! (`payee[0]` will be assigned `shares[0]`). The amount of shares each payee has determines what portion of the payments
//! belongs to the payee. Adding payees is only possible during contract instantiation.
//! 
//! ## Notes
//! This contract is missing some of the functionality present in OpenZeppelin's version.
//! In addition to the normal payments, OpenZeppelin's contract implements an ERC20 interface.
//! This interface allows payees to also receive ERC20 tokens (based on the ERC20 contract's address). 

#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod payment_splitter {

    use ink_storage::{
        traits::SpreadAllocate,
        Mapping,
    };

    use ink_prelude::vec::Vec;

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct PaymentSplitter {
        /// The total amount of shares distributed to payees
        total_shares: Balance,
        /// The total amount of payments released from the contract
        total_released: Balance,
        /// Map each payee's AccountId to the amount of shares they hold
        shares: Mapping<AccountId, Balance>,
        /// Map the amount each payee has already received from the contract
        released: Mapping<AccountId, Balance>,
        /// a list of payees
        payees: Vec<AccountId>,
    }

    /// event for when a new payee is added
    #[ink(event)]
    pub struct PayeeAdded {
        #[ink(topic)]
        account: Option<AccountId>,
        #[ink(topic)]
        shares: Balance,
    }

    /// event for when a payment is released to a share holder
    #[ink(event)]
    pub struct PaymentReleased {
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        amount: Balance,
    }

    impl PaymentSplitter {
        /// The only constructor of the contract
        /// 
        /// A list of shareholders (`payees`) and a corresponding list of `shares` that each
        /// payee has needs to be supplied. `payee[0]` will have `shares[0]` amount of shares.
        /// `payees.len() == shares.len()`
        #[ink(constructor)]
        pub fn new(payees: Vec<AccountId>, shares: Vec<Balance>) -> Self {
            ink_lang::utils::initialize_contract(|contract| {
                Self::new_init(contract, &payees, &shares);
            })
        }

        fn new_init(&mut self, payees: &Vec<AccountId>, shares: &Vec<Balance>) {
            assert!(payees.len() == shares.len(), "PaymentSplitter: payees and shares length mismatch");
            assert!(payees.len() > 0,  "PaymentSplitter: no payees");

            // save each payee into the contract
            for i in 0..payees.len() {
                self.add_payee(&payees[i], shares[i]);
            }
        }

        /// Getter for the total shares held by payees
        #[ink(message)]
        pub fn total_shares(&self) -> Balance {
            self.total_shares
        }   
        /// Getter for the total amount of gas held by payees
        #[ink(message)]
        pub fn total_released(&self) -> Balance {
            self.total_released
        }   
    
        /// Getter for the amount of shares a specific account holds
        #[ink(message)]
        pub fn shares(&self, account: AccountId) -> Balance {
            self.shares.get(account).unwrap_or_default()
        }

        /// Getter for the amount of payments released to a specific account
        #[ink(message)]
        pub fn released(&self, account: AccountId) -> Balance {
            self.released.get(account).unwrap_or_default()
        }

        /// Get the payee at the inputted index
        #[ink(message)]
        pub fn payee(&self, index: u128) -> AccountId {
            self.payees[index]
        }

        /// "releases" (transfers) the amount owed to the inputted account
        #[ink(message)]
        pub fn release(&mut self, account: AccountId) {
            assert!(self.shares.get(account).unwrap_or_default() > 0, "PaymentSplitter: account has no shares");

            // get the current balance of the contract, and the historical amount released.
            let total_received = self.env().balance() + self.total_released();
            // get the amount owed to the account
            let payment = self.pending_payment(&account, total_received, self.released(account));

            assert!(payment != 0, "PaymentSplitter: account is not due payment");

            // get the amount the account has already received
            let account_released = self.released.get(account).unwrap_or_default();
            
            // update the amount released into the account and the total released for the contract
            self.released.insert(&account, &(account_released + payment));
            self.total_released += payment;

            // transfer the payment into the payee's account
            if self.env().transfer(account, payment).is_err() {
                panic!("requested transfer failed")
            }

            self.env().emit_event(PaymentReleased {
                to: Some(account),
                amount: payment,
            });
        }

        /// Calculates the amount owed the payee based the historical balances and released amounts
        fn pending_payment(&self, account: &AccountId, total_received: Balance, already_released: Balance) -> Balance {
            (total_received * self.shares.get(account).unwrap_or_default()) / self.total_shares - already_released
        }


        /// Adds a new payee to the contract with the supplied shares. A private function
        /// that is only called on contract instantiation. 
        fn add_payee(&mut self, account: &AccountId, shares: Balance) {
            assert!(shares > 0, "PaymentSplitter: shares are 0");
            assert!(self.shares.get(account).unwrap_or_default() == 0, "PaymentSplitter: account already has shares");
    
            //add the payee, save their share amounts, and update the total shares 
            self.payees.push(*account);
            self.shares.insert(&account, &shares);
            self.total_shares += shares;

            self.env().emit_event(PayeeAdded {
                account: Some(*account),
                shares: shares,
            });
        }
    }

    #[cfg(test)]
    mod tests {
    }
}

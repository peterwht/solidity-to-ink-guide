//! https://github.com/blockchainsllc/DAO/blob/develop/DAO.sol
//! An example contract showing how to port a Solidity
//! contract to an ink! contract.
//!
//! * This is just an example and not safe for production use *

#![cfg_attr(not(feature = "std"), no_std)]
// #![feature(min_specialization)]

use ink_lang as ink;

pub use self::dao::{
    Dao,
    Proposal,
    WEEK,
};
#[ink::contract]
mod dao {
    use ink_storage::{
        traits::{
            PackedLayout,
            SpreadAllocate,
            SpreadLayout,
        },
        Mapping,
    };

    use ink_prelude::vec::Vec;
    use ink_primitives::Key;
    use ink_prelude::collections::BTreeMap;

    use ink_env::{hash::{Keccak256, HashOutput}};
    use ink_env::call::{
        build_call,
        Call,
        ExecutionInput,
        Selector, 
    };
    use scale::Output;

    use erc20::Erc20Ref;

    pub const SECOND: u64 = 1;
    pub const MINUTE: u64 = 60 * SECOND;
    pub const HOUR: u64 = 60 * MINUTE;
    pub const DAY: u64 = 24 * HOUR;
    pub const WEEK: u64 = 7 * DAY;

    // The minimum debate period that a generic proposal can have
    const MIN_PROPOSAL_DEBATE_PERIOD: u64 = 2 * WEEK;
    // The minimum debate period that a split proposal can have
    const QUORUM_HALVING_PERIOD: u64 = 25 * WEEK;
    // Period after which a proposal is closed
    // (used in the case `executeProposal` fails because it throws)
    const EXECUTE_PROPOSAL_PERIOD: u64 = 10 * DAY;
    // Time for vote freeze. A proposal needs to have majority support before votingDeadline - preSupportTime
    const PRE_SUPPORT_TIME: u64 = 2 * DAY;
    // Denotes the maximum proposal deposit that can be given. It is given as
    // a fraction of total Ether spent plus balance of the DAO
    const MAX_DEPOSIT_DIVISOR: u128 = 100;


    /// A wrapper that allows us to encode a blob of bytes.
    ///
    /// We use this to pass the set of untyped (bytes) parameters to the `CallBuilder`.
    struct CallInput<'a>(&'a [u8]);

    impl<'a> scale::Encode for CallInput<'a> {
        fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
            dest.write(self.0);
        }
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Dao {
        // Proposals to spend the DAO's ether
        proposals: Vec<Proposal>,
        // The quorum needed for each proposal is partially calculated by
        // totalSupply / minQuorumDivisor
        min_quorum_divisor: u128, // u256;
        // The unix time of the last time quorum was reached on a proposal
        last_time_min_quorum_met: u64, // u256;

        // Address of the curator
        curator: AccountId,
        
        // The whitelist: List of addresses the DAO is allowed to send ether to;
        allowed_recipients: Mapping<AccountId, bool>,

        // Map of addresses blocked during a vote (not allowed to transfer DAO
        // tokens). The address points to the proposal ID.
        blocked: Mapping<AccountId, u64>, // u256>,

        // Map of addresses and proposal voted on by this address
        voting_register: Mapping<AccountId, Vec<u64>>, // u256>>,

        // The minimum deposit (in wei) required to submit any proposal that is not
        // requesting a new Curator (no deposit is required for splits)
        proposal_deposit: u128, // u256;

        // the accumulated sum of all current proposal deposits
        sum_of_proposal_deposits: u128, // u256;

        //Voting power is represented by amount of Erc20 tokens
        token: Erc20Ref,
    }

    // A proposal with `newCurator == false` represents a transaction
    // to be issued by this DAO
    // A proposal with `newCurator == true` represents a DAO split
    #[derive(
        Debug,
        scale::Encode,
        scale::Decode,
        SpreadLayout,
        PackedLayout,
        SpreadAllocate,
        Default,
        Clone,
    )]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
    )]
    pub struct Proposal {
        // The address where the `amount` will go to if the proposal is accepted
        recipient: AccountId,
        // The amount to transfer to `recipient` if the proposal is accepted.
        amount: Balance,
        // A plain text description of the proposal
        description: Vec<u8>,
        // A unix timestamp, denoting the end of the voting period
        voting_deadline: Timestamp,
        // True if the proposal's votes have yet to be counted, otherwise False
        open: bool,
        // True if quorum has been reached, the votes have been counted, and
        // the majority said yes
        proposal_passed: bool,
        // A hash to check validity of a proposal
        proposal_hash: Hash,
        // Deposit in wei the creator added when submitting their proposal. It
        // is taken from the msg.value of a newProposal call.
        proposal_deposit: Balance,
        // True if this proposal is to assign a new Curator
        new_curator: bool,
        // true if more tokens are in favour of the proposal than opposed to it at
        // least `preSupportTime` before the voting deadline
        pre_support: bool,
        // Number of Tokens in favor of the proposal
        yea: u128, // u256
        // Number of Tokens opposed to the proposal
        nay: u128,// u256
        // Simple mapping to check if a shareholder has voted for it
        voted_yes: BTreeMap<AccountId, bool>,
        // Simple mapping to check if a shareholder has voted against it
        voted_no: BTreeMap<AccountId, bool>,
        // Address of the shareholder who created the proposal
        creator: AccountId,
    }

    impl ink_storage::traits::PackedAllocate for Proposal {
        fn allocate_packed(&mut self, at: &Key){
            // PackedAllocate::allocate_packed(&mut *self, at);
        }
    }

    #[ink(event)]
    pub struct ProposalAdded {
        #[ink(topic)]
        proposal_id: u64,
        recipient: AccountId,
        amount: Balance, 
        description: Vec<u8>
    }

    #[ink(event)]
    pub struct Voted {
        #[ink(topic)]
        proposal_id: u64,
        position: bool,
        #[ink(topic)]
        voter: AccountId,
    }

    #[ink(event)]
    pub struct ProposalTallied {
        #[ink(topic)]
        proposal_id: u64,
        result: bool,
        quorum: u128,
    }

    #[ink(event)]
    pub struct AllowedRecipientChanged {
        #[ink(topic)]
        recipient: AccountId,
        allowed: bool,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        ProposalExecutionFailed,
        ProposalCreationFailed,
        OutsideDeadline,
        TransactionFailed,
        CallerIsCurator,
        UnableToHalveQuorum,
        UnableToChangeDeposit,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    impl Dao {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        #[ink(constructor)]
        pub fn new(curator: AccountId, proposal_deposit: Balance, token_contract_id: AccountId) -> Self {
            ink_lang::utils::initialize_contract(|contract| {
                Self::new_init(contract, curator, proposal_deposit, token_contract_id)
            })
        }

        fn new_init(&mut self, curator: AccountId, proposal_deposit: Balance, token_contract_id: AccountId) {

            self.token = ink_env::call::FromAccountId::from_account_id(token_contract_id);

            self.curator = curator;
            self.proposal_deposit = proposal_deposit;
            self.last_time_min_quorum_met = self.env().block_timestamp();
            self.min_quorum_divisor = 7; // sets the minimal quorum to 14.3%

            //index 0 is used for null-entries (get_or_modify_blocked)
            self.proposals.push(Proposal::default());

            self.allowed_recipients.insert(&self.env().account_id(), &true);
            self.allowed_recipients.insert(&self.curator, &true);
        }

        //NOTE: This returns a u64 (instead of the uint256 in Solidity).
        //u64 is more than large enough to represent the proposals that could likely exist.
        #[ink(message, payable)]
        pub fn new_proposal(&mut self, recipient: AccountId, amount: Balance, description: Vec<u8>, transaction_data: Vec<u8>, debating_period: u64) -> Result<u64> {
            let caller = self.env().caller();
            self.ensure_tokenholder(&caller);

            let deposit = self.env().transferred_value();

            if !self.allowed_recipients.get(recipient).unwrap_or(false)
                || debating_period < MIN_PROPOSAL_DEBATE_PERIOD 
                || debating_period > 8 * WEEK 
                || deposit < self.proposal_deposit 
                || caller == self.env().account_id() {
                    return Err(Error::ProposalCreationFailed)
            }

            // to prevent curator from halving quorum before first proposal
            if self.proposals.len() == 1 { // initial length is 1 (see constructor)
                self.last_time_min_quorum_met = self.env().block_timestamp();
            }

            let proposal_id: u64 = self.proposals.len() as u64;

            // let encodable = (recipient, amount, transaction_data); // Implements `scale::Encode`
            // let mut output = <Keccak256 as HashOutput>::Type::default(); // 256-bit buffer
            // ink_env::hash_encoded::<Keccak256, _>(&encodable, &mut output);
            let proposal_hash = hash_proposal(&recipient, &amount, &transaction_data);

            let p: Proposal = Proposal{
                recipient: recipient,
                amount: amount,
                description: description.clone(),
                voting_deadline: self.env().block_timestamp() + debating_period,
                open: true,
                proposal_passed: false,
                proposal_hash: proposal_hash,
                proposal_deposit: deposit,
                new_curator: false,
                pre_support: false,
                yea: 0,
                nay: 0,
                voted_yes: BTreeMap::new(),
                voted_no: BTreeMap::new(),
                creator: caller,
            };

            self.sum_of_proposal_deposits += deposit;
            
            self.proposals.push(p);

            //NOTE: because cross-contract calls are being used, emitting events does not work
            // self.env().emit_event(ProposalAdded {
            //     proposal_id,
            //     recipient,
            //     amount,
            //     description
            // });

            Ok(proposal_id)
        }

        //NOTE: not all Solidity bool returns should be a Result<()>. 
        //Ensure that Result is only used for Solidity functions returning a boolean as a 
        //success or no success
        #[ink(message)]
        pub fn check_proposal_code(&mut self, proposal_id: u64, recipient: AccountId, amount: u128, transaction_data: Vec<u8>) -> bool {
            let p = &self.proposals[proposal_id as usize];
            let encodable = (recipient, amount, transaction_data); // Implements `scale::Encode`
            let mut output = <Keccak256 as HashOutput>::Type::default(); // 256-bit buffer
            ink_env::hash_encoded::<Keccak256, _>(&encodable, &mut output);
            return p.proposal_hash == Hash::from(output);
        }



        #[ink(message)]
        pub fn vote(&mut self, proposal_id: u64, supports_proposal: bool) {
            let caller = self.env().caller();

            self.un_vote(proposal_id);

            let caller_balance = self.get_token_balance(&caller);

            let mut p = &mut self.proposals[proposal_id as usize];

            if supports_proposal {
                p.yea += caller_balance;
                p.voted_yes.insert(caller, true);
            }else {
                p.nay += caller_balance;
                p.voted_no.insert(caller, true);
            }


            let blocked_proposal = self.blocked.get(caller).unwrap_or(0);
            if  blocked_proposal == 0 {
                self.blocked.insert(caller, &proposal_id);
            }else if p.voting_deadline > self.proposals[blocked_proposal as usize].voting_deadline {
                self.blocked.insert(caller, &proposal_id);
            }

            let voted_proposals = &mut self.voting_register.get(caller).unwrap_or(Vec::new());
            voted_proposals.push(proposal_id);
            self.voting_register.insert(caller, voted_proposals);
            
            // self.env().emit_event(Voted {
            //     proposal_id,
            //     position: supports_proposal,
            //     voter: caller,
            // });
        }

        #[ink(message)]
        pub fn un_vote(&mut self, proposal_id: u64) -> Result<()>{
            let caller = self.env().caller();
            let now = self.env().block_timestamp();

            let caller_balance = self.get_token_balance(&caller);            

            let mut p = &mut self.proposals[proposal_id as usize];

            if now >= p.voting_deadline {
                //NOTE: this is more specific than the .sol version.
                //The .sol version uses `throw`
                return Err(Error::OutsideDeadline);
            }

            if *p.voted_yes.get(&caller).unwrap_or(&false) {
                p.yea -= caller_balance;
                p.voted_yes.insert(caller, false);
            }
            
            if *p.voted_no.get(&caller).unwrap_or(&false) {
                p.nay -= caller_balance;
                p.voted_no.insert(caller, false);
            }
            Ok(())
        }

        #[ink(message)]
        pub fn un_vote_all(&mut self) {
            let caller = self.env().caller();
            let now = self.env().block_timestamp();
            let voting_register = &mut self.voting_register.get(caller).unwrap_or(Vec::new());

            // DANGEROUS loop with dynamic length - needs improvement.
            for i in 0..(voting_register.len()){
                let prop_id = voting_register[i];
                let p = &self.proposals[prop_id as usize];
                if now < p.voting_deadline {
                    self.un_vote(prop_id).expect("unable to unvote");
                }
                
            }

            //clear the voting register, and update the mapping entry
            voting_register.clear();
            self.voting_register.insert(caller, voting_register);
            self.blocked.insert(caller, &0);
        }

        fn verify_pre_support(&mut self, proposal_id: u64) {
            let now = self.env().block_timestamp();
            let mut p = &mut self.proposals[proposal_id as usize];
            
            if now < p.voting_deadline - PRE_SUPPORT_TIME {
                p.pre_support = true;
            }else{
                p.pre_support = false;
            }
        }

        #[ink(message)]
        //TODO: turn function_selector back to [u8; 4] -- edited because UI does not work with it
        pub fn execute_proposal(&mut self, proposal_id: u64, function_selector: Vec<u8>, transaction_data: Vec<u8>, gas_limit: u64) -> Result<()>{
            let now = self.env().block_timestamp();

            let p = &self.proposals[proposal_id as usize];

            if p.open && now > p.voting_deadline + EXECUTE_PROPOSAL_PERIOD {
                self.close_proposal(proposal_id);
                return Ok(())
            }

            let encodable = (p.recipient, p.amount, transaction_data.clone()); // Implements `scale::Encode`
            let mut output = <Keccak256 as HashOutput>::Type::default(); // 256-bit buffer
            ink_env::hash_encoded::<Keccak256, _>(&encodable, &mut output);

            if now < p.voting_deadline
                || !p.open
                || p.proposal_passed
                || p.proposal_hash != Hash::from(output) {
                    return Err(Error::ProposalExecutionFailed)
                }

            if !self.allowed_recipients.get(p.recipient).unwrap_or(false) {
                // transfer the payment into the payee's account
                if self.env().transfer(p.creator, p.proposal_deposit).is_err() {
                    panic!("unable to return deposit")
                }

                self.close_proposal(proposal_id);

                return Ok(());
            }

            let mut proposal_check = true;

            if p.amount > self.actual_balance() || p.pre_support == false{
                proposal_check = false;
            }

            let quorum = p.yea;
            if transaction_data.len() >= 4 && transaction_data[0] == 0x68
                && transaction_data[1] == 0x37 && transaction_data[2] == 0xff
                && transaction_data[3] == 0x1e
                && quorum < self.min_quorum(self.actual_balance()) {
                    proposal_check = false
            }

            if quorum >= self.min_quorum(p.amount){
                if self.env().transfer(p.creator, p.proposal_deposit).is_err() {
                    panic!("unable to return deposit")
                }

                self.last_time_min_quorum_met = now;

                if quorum > self.token.total_supply() / 7{
                    self.min_quorum_divisor = 7;
                }
            }

            if quorum >= self.min_quorum(p.amount) && p.yea > p.nay && proposal_check {
                // we are setting this here before the CALL() value transfer to
                // assure that in the case of a malicious recipient contract trying
                // to call executeProposal() recursively money can't be transferred
                // multiple times out of the DAO
                {
                    let p_mut = &mut self.proposals[proposal_id as usize];
                    p_mut.proposal_passed = true;
                }

                //TODO: remove this once the UI is fixed
                let mut tmp_selector: [u8; 4] = [0;4];
                tmp_selector[0] = function_selector[0];
                tmp_selector[1] = function_selector[1];
                tmp_selector[2] = function_selector[2];
                tmp_selector[3] = function_selector[3];

                // this call is as generic as any transaction. It sends all gas and
                // can do everything a transaction can do. It can be used to reenter
                // the DAO. The `p.proposalPassed` variable prevents the call from 
                // reaching this line again
                let res = self.invoke_transaction(proposal_id, &tmp_selector, &transaction_data, &gas_limit);
                if res.is_err(){
                    return res;
                }
            }

            self.close_proposal(proposal_id);

            // self.env().emit_event(ProposalTallied {
            //     proposal_id,
            //     result: true,
            //     quorum,
            // });

            Ok(())
        }

        fn close_proposal(&mut self, proposal_id: u64) {
            let p = &mut self.proposals[proposal_id as usize];

            if p.open {
                self.sum_of_proposal_deposits -= p.proposal_deposit;
            }

            p.open = false;
        }
        
        fn new_contract(&self, new_contract: AccountId) {
            let caller = self.env().caller();
            let contract_addr = self.env().account_id();

            if caller == contract_addr || !self.allowed_recipients.get(new_contract).unwrap_or(false) {
                return;
            }

            if self.env().transfer(new_contract, self.env().balance()).is_err() {
                panic!("unable to transfer to new contract")
            }
        }

        #[ink(message)]
        pub fn change_proposal_deposit(&mut self, proposal_deposit: Balance) -> Result<()> {
            let caller = self.env().caller();
            let contract_addr = self.env().account_id();

            if caller != contract_addr || proposal_deposit > (self.actual_balance() / MAX_DEPOSIT_DIVISOR){
                return Err(Error::UnableToChangeDeposit);
            }

            self.proposal_deposit = proposal_deposit;
            Ok(())
        }

        #[ink(message)]
        pub fn change_allowed_recipients(&mut self, recipient: AccountId, allowed: bool) -> Result<()> {
            let caller = self.env().caller();

            if caller != self.curator{
                return Err(Error::CallerIsCurator);
            }

            self.allowed_recipients.insert(recipient, &allowed);

            // self.env().emit_event(AllowedRecipientChanged {
            //     recipient,
            //     allowed,
            // });

            return Ok(())
        }

        // Invoke a confirmed execution without getting its output.
        //
        // If the transaction which is invoked transfers value, this value has
        // to be sent as payment with this call. The method will fail otherwise,
        // and the transaction would then be reverted.
        //
        // Its return value indicates whether the called transaction was successful.
        // This can be called by anyone.
        // 
        // https://github.com/paritytech/ink/blob/master/examples/multisig/lib.rs
        fn invoke_transaction(
            &mut self,
            proposal_id: u64, function_selector: &[u8; 4], transaction_data: &Vec<u8>, gas_limit: &u64) -> Result<()> {
            let p = &self.proposals[proposal_id as usize];
            
            let result = build_call::<<Self as ::ink_lang::reflect::ContractEnv>::Env>()
                .call_type(
                    Call::new()
                        .callee(p.recipient) //contract to call
                        .gas_limit(*gas_limit)
                        .transferred_value(p.amount), //value to transfer with call
                )
                .exec_input(
                    ExecutionInput::new(Selector::from(*function_selector)).push_arg(CallInput(transaction_data)), //SCALE encoded parameters
                )
                .returns::<()>()
                .fire()
                .map_err(|_| Error::TransactionFailed);
            result
        }

        fn actual_balance(&self) -> u128 {
            return self.env().balance() - self.sum_of_proposal_deposits;
        }

        fn min_quorum(&self, value: u128) -> u128 {
            let total_supply = self.token.total_supply();
            return total_supply / self.min_quorum_divisor +
                (value * total_supply) / (3 * (self.actual_balance()));
        }

        #[ink(message)]
        pub fn halve_min_quorum(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let now = self.env().block_timestamp();
            // this can only be called after `quorumHalvingPeriod` has passed or at anytime after
            // fueling by the curator with a delay of at least `minProposalDebatePeriod`
            // between the calls
            if (self.last_time_min_quorum_met < ( now - QUORUM_HALVING_PERIOD) || caller == self.curator) 
                && self.last_time_min_quorum_met < (now - MIN_PROPOSAL_DEBATE_PERIOD)
                && self.proposals.len() > 1 {
                self.last_time_min_quorum_met = now;
                self.min_quorum_divisor *= 2;
                return Ok(());
            }

            Err(Error::UnableToHalveQuorum)
        }

        #[ink(message)]
        pub fn number_of_proposals(&self) -> u64 {
            return (self.proposals.len() - 1) as u64;
        }

        fn get_or_modify_blocked(&mut self, account: AccountId) -> bool {
            let prop_id = self.blocked.get(account).unwrap_or(0);
            if prop_id == 0 {
                return false
            }

            let p = &self.proposals[prop_id as usize];
            if !p.open{
                self.blocked.insert(account, &0);
                return false;
            }

            true
        }

        #[ink(message)]
        pub fn unblock_me(&mut self) -> bool {
            self.get_or_modify_blocked(self.env().caller())
        }

        //only compiles when *not* running tests
        #[cfg(not(test))]
        fn get_token_balance(&self, caller: &AccountId) -> Balance {
            self.token.balance_of(*caller)
        }

        //only compiles when running tests
        #[cfg(test)]
        fn get_token_balance(&self, _: &AccountId) -> Balance {
            1
        }

        //NOTE: is a modifer in Solidity. Will panic! if 
        //not a tokenholder
        fn ensure_tokenholder(&self, caller: &AccountId) {
            assert!(self.get_token_balance(caller) != 0);
        }

        //NOTE: this function is for debugging on-chain. Not a part of 
        //the original contract.
        #[ink(message)]
        pub fn get_proposal(&self, prop_id: u64) -> Proposal {
            self.proposals[prop_id as usize].clone()
        }

        //NOTE: this function is for confirming the ERC20 cross-contract call
        //is working. It is not a part of the original contract
        #[ink(message)]
        pub fn get_total_supply(&self) -> Balance {
            self.token.total_supply()
        }

    }

    //helper function for to hash the proposal
    fn hash_proposal(recipient: &AccountId, amount: &Balance, transaction_data: &Vec<u8>) -> Hash {
        let encodable = (recipient, amount, transaction_data); // Implements `scale::Encode`
        let mut output = <Keccak256 as HashOutput>::Type::default(); // 256-bit buffer
        ink_env::hash_encoded::<Keccak256, _>(&encodable, &mut output);
        return Hash::from(output);
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use ink_env::Clear;
        use ink_lang as ink;

        /// The default constructor does its job.
        #[ink::test]
        fn new_works() {
            let accounts =
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            // Constructor works.
            let dao = Dao::new(accounts.alice, 7, AccountId::from([0x01; 32]));
            //the proposals should start at length 1
            assert_eq!(dao.proposals.len(), 1);
            assert_eq!(dao.curator, accounts.alice);
            assert_eq!(dao.proposal_deposit, 7);
            // timestamp check: https://substrate.stackexchange.com/questions/2966/manipulate-block-timestamp-for-ink-integration-tests
            //TODO: assert_eq!(dao.last_time_min_quorum_met, ...)
            assert_eq!(dao.min_quorum_divisor, 7);
            assert_eq!(dao.allowed_recipients.get(accounts.alice).unwrap(), true);
            assert_eq!(dao.allowed_recipients.get(accounts.bob).unwrap_or(false), false);
            //TODO: assert_eq!(dao.allowed_recipients.get(<contract address>).unwrap(), true)
        }

        #[ink::test]
        fn new_proposal_works(){
            let accounts =
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();

            // Constructor works.
            let mut dao = Dao::new(accounts.alice, 1, AccountId::from([0x01; 32]));
            // // set bob as the contract caller
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);

            ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(2);
            
            assert_eq!(dao.new_proposal(AccountId::from([0x01; 32]), 5, Vec::<u8>::from("prop 1"), vec![0x02; 5], 2 * WEEK), Ok(1));
            let p = &dao.proposals[1];

            assert_eq!(p.recipient, AccountId::from([0x01; 32]));
            assert_eq!(p.amount, 5);
            assert_eq!(p.description, Vec::<u8>::from("prop 1"));
            //TODO: check all fields -- if worth the time
        }

        #[ink::test]
        fn check_proposal_code_works(){ 
            let accounts =
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut dao = Dao::new(accounts.alice, 1, AccountId::from([0x01; 32]));
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);
            ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(2);
            let recipient = AccountId::from([0x01; 32]);
            let amount = 5;
            let transaction_data = vec![0x02; 5];
            dao.new_proposal(recipient, amount, Vec::<u8>::from("prop 1"), transaction_data.clone(), 2 * WEEK).unwrap();
            
            assert_eq!(dao.check_proposal_code(1, recipient, amount, transaction_data), true);
        }

        #[ink::test]
        fn check_vote_works(){ 
            let accounts =
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut dao = Dao::new(accounts.alice, 1, AccountId::from([0x01; 32]));
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);
            ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(2);
            let recipient = AccountId::from([0x01; 32]);
            let amount = 5;
            let transaction_data = vec![0x02; 5];
            dao.new_proposal(recipient, amount, Vec::<u8>::from("prop 1"), transaction_data.clone(), 2 * WEEK).unwrap();

            dao.vote(1, true);
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.charlie);
            dao.vote(1, false);

            let p = &dao.proposals[1];

            assert_eq!(p.yea, 1);
            assert_eq!(p.nay, 1);
            assert_eq!(*p.voted_yes.get(&accounts.bob).unwrap(), true);
            assert_eq!(*p.voted_no.get(&accounts.charlie).unwrap(), true);
        }

        #[ink::test]
        fn check_un_vote_works(){ 
            let accounts =
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut dao = Dao::new(accounts.alice, 1, AccountId::from([0x01; 32]));
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);
            ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(2);
            let recipient = AccountId::from([0x01; 32]);
            let amount = 5;
            let transaction_data = vec![0x02; 5];
            dao.new_proposal(recipient, amount, Vec::<u8>::from("prop 1"), transaction_data.clone(), 2 * WEEK).unwrap();

            dao.vote(1, true);
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.charlie);
            dao.vote(1, false);

            dao.un_vote(1);
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);
            dao.un_vote(1);

            let p = &dao.proposals[1];
            assert_eq!(p.yea, 0);
            assert_eq!(p.nay, 0);
            assert_eq!(*p.voted_yes.get(&accounts.bob).unwrap(), false);
            assert_eq!(*p.voted_no.get(&accounts.charlie).unwrap(), false);
        }

        #[ink::test]
        fn check_un_vote_all_works(){ 
            let accounts =
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut dao = Dao::new(accounts.alice, 1, AccountId::from([0x01; 32]));
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);
            ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(2);
            let recipient = AccountId::from([0x01; 32]);
            let amount = 5;
            let transaction_data = vec![0x02; 5];
            dao.new_proposal(recipient.clone(), amount, Vec::<u8>::from("prop 1"), transaction_data.clone(), 2 * WEEK).unwrap();
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.charlie);
            dao.new_proposal(recipient, amount + 2, Vec::<u8>::from("prop 2"), transaction_data.clone(), 2 * WEEK).unwrap();

            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);

            dao.vote(1, true);
            dao.vote(2, true);

            dao.un_vote_all();

            let p1 = &dao.proposals[1];
            let p2 = &dao.proposals[2];
            assert_eq!(p1.yea, 0);
            assert_eq!(p1.nay, 0);
            assert_eq!(p2.yea, 0);
            assert_eq!(p2.nay, 0);
            assert_eq!(*p1.voted_yes.get(&accounts.bob).unwrap(), false);
            assert_eq!(*p2.voted_yes.get(&accounts.bob).unwrap(), false);
        }

        #[ink::test]
        #[should_panic]
        fn execute_proposal_works(){ 
            let accounts =
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut dao = Dao::new(accounts.alice, 1, AccountId::from([0x01; 32]));
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);
            ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(5);
            let recipient = AccountId::from([0x01; 32]);
            let amount =1;
            let transaction_data = vec![0x02; 5];
            dao.new_proposal(recipient, amount, Vec::<u8>::from("prop 1"), transaction_data.clone(), 2 * WEEK).unwrap();
            
            dao.vote(1, true);
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.charlie);
            dao.vote(1, true);

            //verify pre_support before increasing timestamp
            dao.verify_pre_support(1);

            //increase timestamp
            for _ in 0..300000{
                ink_env::test::advance_block::<ink_env::DefaultEnvironment>();
            }

            //will panic because "contract invocation" is not supported in an off-chain enviroment
            let res = dao.execute_proposal(1, vec![1,2,3,4], transaction_data, 1000);
        }

        #[ink::test]
        fn close_proposal_works(){
            let accounts =
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut dao = Dao::new(accounts.alice, 1, AccountId::from([0x01; 32]));
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);
            ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(5);
            let recipient = AccountId::from([0x01; 32]);
            let amount =1;
            let transaction_data = vec![0x02; 5];
            dao.new_proposal(recipient, amount, Vec::<u8>::from("prop 1"), transaction_data.clone(), 2 * WEEK).unwrap();
            
            assert_eq!(dao.sum_of_proposal_deposits, 5);

            dao.close_proposal(1);
            let p = &dao.proposals[1];
            assert_eq!(p.open, false);
            assert_eq!(dao.sum_of_proposal_deposits, 0);
        }

        #[ink::test]
        fn unblock_me_works(){
            let accounts =
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut dao = Dao::new(accounts.alice, 1, AccountId::from([0x01; 32]));
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);
            ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(5);
            let recipient = AccountId::from([0x01; 32]);
            let amount =1;
            let transaction_data = vec![0x02; 5];
            dao.new_proposal(recipient, amount, Vec::<u8>::from("prop 1"), transaction_data.clone(), 2 * WEEK).unwrap();

            //should be false before a vote takes place
            assert_eq!(dao.unblock_me(), false);
            dao.vote(1, true);
            assert_eq!(dao.unblock_me(), true);

        }


    }
}


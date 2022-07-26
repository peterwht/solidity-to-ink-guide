# Solidity to ink! Guide

## What is ink!

ink! is the smart contract language used in Substrate. It is built from Rust -- meaning that the great features of Rust are included in ink!. These features are ideal for a smart contract language. Furthermore, ink! is compiled to WebAssembly, allowing for high-performant, consistent, and very well-researched and developed code. ink! smart contracts use Rust's no_std. So, some common Rust implementations are not directly supported. However, ink! does have crates that reimplement common Rust code to work in the Substrate runtime.

## Setup

Follow the [first-smart-contract](https://docs.substrate.io/tutorials/smart-contracts/first-smart-contract/) tutorial.

## Converting Solidity Contract to ink!

### 1. Generate New ink! Contract

Run the following to generate ink! boilerplate code (flipper project)

```
cargo contract new <contract-name>
```

### 2. Build ink! Contract

```
cargo +nightly contract build
```

### 3. Convert Solidity class to Rust struct

Solidity is an object oriented language, and uses classes. ink! (Rust) does not use classes.

An example Solidity class looks like:

<!-- Markdown syntax highlighting does not support Solidity. C++ seems to be the best match -->

```c++
contract MyContract {
    bool _theBool;
    event UpdatedBool(bool indexed theBool);

    constructor(bool _someBool) {
        require(_someBool == true, "someBool must start as true");

        _theBool = _someBool;
    }

    function setBool(bool _newBool) public returns (bool _boolChanged) {
        if _theBool == _newBool{
               _boolChanged = false;
        }else{
            _boolChanged = true;
        }

        _theBool = _newBool;
        //emit event
        UpdatedBool(newBool);
    }
}
```

And the equivalent contract in ink! looks like:

```rust
#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod mycontract {
    #[ink(storage)]
    pub struct MyContract {
        the_bool: bool,
    }

    #[ink(event)]
    pub struct UpdatedBool {
        #[ink(topic)] //-> indexed
        the_bool: bool,
    }

    impl MyContract {
        #[ink(constructor)]
        pub fn new(some_bool: bool) -> Self {
            assert!(some_bool == true, "some_bool must start as true");
            Self { the_bool: some_bool }
        }

        #[ink(message)]
        pub fn set_bool(&mut self, new_bool: bool) -> bool{
            let bool_changed = true;

            if self.the_bool == new_bool{
                bool_changed = false;
            }else{
                bool_changed = true;
            }

            self.the_bool = new_bool;

            self.env().emit_event(UpdatedBool {
                the_bool: new_bool
            });

            //return
            bool_changed
        }
    }
}
```

A few key differences are:

- Solidity class variables / members will be placed in the contract struct in ink!
- All class methods in Solidity are `impl`emented for the contract struct in ink!
- Solidity frequently prefixes variables with an underscore (`_name`). ink! / Rust only prefixes with an underscore for _unused_ variables.
- Solidity uses camelCase. ink! uses snake_case.
- In Solidity, the variable type comes before the variable name (e.g. bool myVar). While ink! specifies var type after the var name (e.g. my_var: bool)

## Best Practices

- If the Solidity contract uses a `string`, it is recommended to use a `Vec<u8>` to avoid the overhead of a `String`. See [here](https://substrate.stackexchange.com/questions/1174/why-is-it-a-bad-idea-to-use-string-in-an-ink-smart-contract) for more details on why. The smart contract should only contain the information that strictly needs to be placed on the blockchain and go through consensus. The UI should be used for displaying strings.

## Syntax Equivalencies

- `public function`

```c++
// solidity
function fnName() public {}
//or
//by default, functions are public
function fnName() {}
```

```rust
// ink!
#[ink(message)]
pub fn fn_name(&self) {}
```

- `mapping declaration`

```c++
// solidity
mapping(address => uint128) private mapName;
```

```rust
//ink!
use ink_storage::{
    traits::SpreadAllocate,
    Mapping,
};

#[ink(storage)]
#[derive(SpreadAllocate)]
pub struct ContractName {
    map_name: Mapping<AccountId, u128>,
}
```

when using a map in ink!, `ink_lang::utils::initialize_contract` must be used in the constructor. See [here](https://ink.substrate.io/datastructures/mapping) for more details.

- `mapping usage`

```c++
// solidity

//insert / update
aMap[aKey] = aValue;

// get
aMap[aKey]
```

```rust
// ink!

//insert / update
self.a_map.insert(&a_key, &a_value);

// get
self.a_map.get(a_key).unwrap()
```

- `struct`

```c++
// solidity
struct MyPerson{
    address person;
    u64 favNum;
}
```

```rust
// ink!
struct MyPerson {
    person: AccountId,
    fav_num: u64,
}
```

- `assertions / requires`

```c++
// solidity
require(someValue < 10, "someValue is not less than 10");
```

```rust
// ink!
assert!(some_value < 10, "some_value is not less than 10");
```

- `virtual` or `override`  
  virtual and override are modifiers in Solidity that allow a function to override another. ink! (Rust) does not have this ability as overriding may create ambiguity.

- `timestamp`

```c++
// solidity
block.timestamp
// or
now
```

```rust
// ink!
self.env().block_timestamp()
```

- `contract caller`

```c++
// solidity
address caller = msg.sender;
```

```rust
// ink!
let caller = self.env().caller();
```

- `events`

```c++
// solidity

// example declaration
event SomeEvent(uint128 value);

// usage
emit SomeEvent(someValue);
```

```rust
// ink!

// declaration
#[ink(event)]
pub struct SomeEvent{
    #[ink(topic)]
    value: u128,
}

// usage
self.env().emit_event(SomeEvent {
    value: some_value,
});
```

- `contract's address`

```c++
// solidity
address(this)
```

```rust
// ink!
self.env().account_id()
```

- `bytes`  
  Solidity has a type `bytes`. `bytes` is (essentially) equivalent to an array of uint8. So, `bytes` in Solidity => `Vec<u8>` in ink!. See [here](https://ethereum.stackexchange.com/questions/91119/difference-between-byte-and-uint8-datatypes-in-solidity) for more details. If desired, a `bytes` struct can be created in ink! to replicate the `bytes` type in Solidity.
- `uint256` TODO
- `payable`

```c++
// solidity
function myFunction() payable returns (uint64) {}
```

```rust
#[ink(message, payable)]
pub fn my_function() -> (u64) {}
```

- `received deposit`

```C++
// solidity
msg.value
```

```rust
// ink!
self.env().transferred_value()
```

- `contract balance`

```c++
// solidity
this.balance
```

```rust
// ink!
self.env().balance()
```

- `transfer from contract`

```c++
// solidity
recipient.send(amount)
```

```rust
//ink!
if self.env().transfer(recipient, amount).is_err() {
    panic!("error transferring")
}
```

- `events & indexed`

```c++
// solidity

event MyCoolEvent(
    u128 indexed indexedValue,
    u128 notIndexedValue,
);

//emit event
MyCoolEvent (someValue, someOtherValue)
```

```rust
// ink!

#[ink(event)]
pub struct MyCoolEvent {
    #[ink(topic)]
    indexed_value: u128,

    not_indexed_value: u128,
}

// emit event
self.env().emit_event(MyCoolEvent {
    indexed_value: some_value,
    not_indexed_value: some_other_value
});
```

- `nested mappings + custom / advanced structures`  
  In Solidity, it is easy to do nested mappings. It is not as straightforward in ink!.

imagine the following scenario

```c++
// solidity
contract Dao {
    struct Proposal {
        mapping (address => bool) votedYes
    }

    mapping (address => bool) public isWhitelisted;
    Proposal[] public proposals;
}
```

in ink! this _seems_ like it could be represented like so:

```rust
#[ink::contract]
mod dao {

    #[derive(SpreadAllocate)]
    pub struct Proposal {
        voted_yes: Mapping<AccountId, bool>,
    }

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Dao {
        proposals: Vec<Proposal>,
        is_whitelisted: Mapping<AccountId, bool>,
    }

    impl Dao{
        #[ink(constructor)]
        pub fn new(/*...*/) -> Self {
            //required for mappings
            ink_lang::utils::initialize_contract(|contract| {/*...*/})
        }
    }
}
```

[This answer](https://substrate.stackexchange.com/questions/1659/how-to-have-a-mapping-in-a-custom-structure-inside-an-ink-contract) explains in detail why nested mappings are not allowed)

So, as of now, to get around this issue an alternate data structure will need to be used. A data-structure that can be interchanged with the `Mapping` syntax and with minimal additional implementations is the `BTreeMap`. `BTreeMap` is less efficient than `Mapping`. This will be used in the nested struct. Additional `derive`s will need to be added to be compatible with the #[ink(storage)] struct (see below).

```rust
#[ink::contract]
mod dao {

    use ink_prelude::collections::BTreeMap;

    #[derive(
        scale::Encode,
        scale::Decode,
        SpreadLayout,
        PackedLayout,
        SpreadAllocate,
    )]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink_storage::traits::StorageLayout)
    )]
    pub struct Proposal {
        voted_yes: BTreeMap<AccountId, bool>,
    }

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Dao {
        proposals: Vec<Proposal>,
        is_whitelisted: Mapping<AccountId, bool>,
    }

    impl Dao{
        #[ink(constructor)]
        pub fn new(/*...*/) -> Self {
            //required for mappings
            ink_lang::utils::initialize_contract(|contract| {/*...*/})
        }
    }
}
```

This almost works as expected. However, there is still one issue. `SpreadAllocate` (used with `Mapping`) requires that `Vec<Proposal>` implements `PackedAllocate`. To fix this, `Proposal` needs to implement `PackedAllocate`. See [here](https://paritytech.github.io/ink/ink_storage/traits/trait.PackedAllocate.html) for details + examples. See the following for this example:

```rust
    use ink_primitives::Key;

    pub struct Proposal {
        voted_yes: BTreeMap<AccountId, bool>,
    }

    impl ink_storage::traits::PackedAllocate for Proposal {
        fn allocate_packed(&mut self, at: &Key){
            PackedAllocate::allocate_packed(&mut *self, at)
        }
    }
```

- `cross-contract calling`

```c++
// solidity
```

```rust
// ink!
```

In ink!

- `submit generic transaction / dynamic cross-contract calling`

```c++
// solidity

// invokes function found at`addr`, sends the `_amount` to the `addr`, and the `_transactionData` payload.
addr.call.value(_amount)(_transactionData)
```

```rust
// ink!

// ...

use ink_env::call::{
    build_call,
    Call,
    ExecutionInput,
    Selector,
};

/// A wrapper that allows us to encode a blob of bytes.
///
/// We use this to pass the set of untyped (bytes) parameters to the `CallBuilder`.
struct CallInput<'a>(&'a [u8]);

impl<'a> scale::Encode for CallInput<'a> {
    fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
        dest.write(self.0);
    }
}

// ...

// see: https://github.com/paritytech/ink/blob/master/examples/multisig/lib.rs#L535
fn invoke_transaction(
    &mut self,
    callee: AccountId,
    transfer_amount: u128,
    function_selector: &[u8; 4],
    transaction_data: &Vec<u8>,
    gas_limit: &u64) -> Result<()> {

    let result = build_call::<<Self as ::ink_lang::reflect::ContractEnv>::Env>()
        .call_type(
            Call::new()
                .callee(callee) //contract to call
                .gas_limit(*gas_limit)
                .transferred_value(transfer_amount), //value to transfer with call
        )
        .exec_input(
            ExecutionInput::new(Selector::from(*function_selector)).push_arg(CallInput(transaction_data)), //SCALE encoded parameters
        )
        .returns::<()>()
        .fire()
        .map_err(|_| Error::TransactionFailed);
    result
}

```

Note: the `function_selector` bytes can be found in the generated `target/ink/metadata.json`

## Limitations of ink!

- Multi-file projects are not supported with pure ink!
  - implementing traits / interfaces will not work
  - There are alternatives that do add this functionality such as OpenBrush
- Nested structs and data structures can be difficult to use
- Cross-contract calling prevents events from being emitted. See [here](https://github.com/paritytech/ink/issues/1000) for details

## Troubleshooting Errors

- `ERROR: Validation of the Wasm failed.`

```
ERROR: Validation of the Wasm failed.

ERROR: An unexpected panic function import was found in the contract Wasm.
This typically goes back to a known bug in the Rust compiler:
https://github.com/rust-lang/rust/issues/78744

As a workaround try to insert `overflow-checks = false` into your `Cargo.toml`.
This will disable safe math operations, but unfortunately we are currently not
aware of a better workaround until the bug in the compiler is fixed.
```

**Solution**  
Add the following to contract the Cargo.toml:

```
[profile.release]
overflow-checks = false
```

## ink! Tips + Usage

### unit testing (off-chain)

- Unit tests are an excellent way to ensure your code works before attempting on-chain testing.
- To run ink! tests, do _not_ use `cargo +nightly contract test`. Use `cargo +nightly test`. See [here](https://substrate.stackexchange.com/questions/3197/how-to-understand-which-test-failed-in-ink) for more info why.
- From the contract module, make sure to `use` the contract struct and anything else that is going to be used in the unit tests. For example:

```rust
// top of file
#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

pub use self::mycontract::{
    MyContract
};
```

- useful code to interact + modify contract enviroment for testing

[ink_env docs](https://paritytech.github.io/ink/ink_env/test/index.html)

```rust
// get the default accounts (alice, bob, ...)
let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
accounts.alice //usage example

// set which account calls the contract
ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);

//get the contract's address
let callee = ink_env::account_id::<ink_env::DefaultEnvironment>();

// set the contracts address.
// by default, this is alice's account
ink_env::test::set_callee::<ink_env::DefaultEnvironment>(callee);

// transfer native currency to the contract
ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(2);

// increase block number (and block timestamp).
// this can be placed in a loop to advance the block # many times
ink_env::test::advance_block::<ink_env::DefaultEnvironment>();

//generate arbitrary AccountId
AccountId::from([0x01; 32]);

//generate arbitrary Hash
Hash::from([0x01; 32])
```

## Misc Notes (needs to be organized)

- - cargo +nightly contract build
- - solidity does \_parameters
- ink! does not have modifiers like solidity, so the modifiers should be written as functions in ink! and checked at the beginning of the function
- Mapping seems like it can only be used in the #[ink(storage)] struct.
  - nested mappings are not allowed
  - https://substrate.stackexchange.com/questions/1659/how-to-have-a-mapping-in-a-custom-structure-inside-an-ink-contract
  - BTreeMap is a workaround to using a map still -- although less efficient
- Using custom structures may require some additional trait implementations
  - if you have a storage struct with a Vec<SomeStruct>, an error may be thrown saying that `PackedAllocate` is not implemented for `SomeStruct`. To fix this, implement `PackedAllocate` for that struct
  - https://paritytech.github.io/ink/ink_storage/traits/trait.PackedAllocate.html#impl-PackedAllocate-for-StdLinkedList%3CT%3E
- `p.recipient.call.value(p.amount)(_transactionData)` calls the contract with address `recipient` and sets the value to send with the call, and `transactionData` is the payload passed into the contract
- tests should be run with `cargo +nightly test` otherwise the tests do no give clear errors and debugging prints do not work
- alice is by default the contract address
- add a part on how Solidity has defaults, and care should be taken in rust to ensure the proper default

### misc steps

- place Solidity class variables to proper places in to the contract mod
  - e.g. in struct, in constants
- convert datatypes to rust compatible, and swap from datatype first to datatype after (e.g., uint128 var -> var: u128)

https://paritytech.github.io/ink/ink_prelude/collections/btree_map/struct.BTreeMap.html
https://paritytech.github.io/ink/src/ink_storage/traits/packed.rs.html#22-29

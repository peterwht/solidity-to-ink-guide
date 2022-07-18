# Solidity to ink! Guide

## Setup

Follow the [first-smart-contract](https://docs.substrate.io/tutorials/smart-contracts/first-smart-contract/) tutorial.

## Converting Solidity Contract to ink!

### 1. Start New ink! Contract

Run the following to generate ink! boilerplate code (flipper project)

```
cargo contract new <contract-name>
```

## Syntax Equivalencies

- `public function`

```
// solidity
function fnName() public {}
```

```rust
// ink!
#[ink(message)]
pub fn fn_name(&self) {}
```

- `mapping declaration`

```
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

```
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
self.a_map.get(a_key).unwrap_or_default()
```

- `assertions / requires`

```
// solidity
require(someValue < 10, "someValue is not less than 10");
```

```rust
// ink!
assert!(some_value < 10, "some_value is not less than 10");
```

- `virtual` or `override`  
  virtual and override are modifiers in Solidity that allow a function to override another. ink! (Rust) does not have this ability as it may potentially create ambiguity.

- `timestamp`

```
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

```
// solidity
address caller = msg.sender;
```

```rust
// ink!
let caller = self.env().caller();
```

- `events`

```
// solidity

// example declaration
event SomeEvent(uint128 value);

// usage
{
    emit SomeEvent(someValue);
}
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
{
    self.env().emit_event(SomeEvent {
        value: some_value,
    });
}
```

- `contract address`

```
// solidity
address(this)
```

```rust
// ink!
self.env().account_id()
```

- `bytes32`
- `bytes`  
  bytes can be represented as a [u8]
- `uint256`
- `received deposit`

```
msg.value
```

```rust
self.env().transferred_value()
```

- `events & indexed`

```
// solidity

event MyCoolEvent(
    u128 indexed indexedValue,
    u128 notIndexedValue,
);

//emit event
{
    MyCoolEvent (
        someValue,
        someOtherValue
    )
}
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
{
    self.env().emit_event(MyCoolEvent {
        indexed_value: some_value,
        not_indexed_value: some_other_value
    });
}
```

## Misc Notes (needs to be organized)

- payable => #[ink(payable)]
- cargo +nightly contract build
- solidity does \_parameters
- ink! does not have modifiers like solidity, so the modifiers should be written as functions in ink! and checked at the beginning of the function
- Mapping seems like it can only be used in the #[ink(storage)] struct.
  - nested mappings are not allowed
  - https://substrate.stackexchange.com/questions/1659/how-to-have-a-mapping-in-a-custom-structure-inside-an-ink-contract
  - BTreeMap is a workaround to using a map still -- although less efficient
- Using custom structures may require some additional trait implementations
  - if you have a storage struct with a Vec<SomeStruct>, an error may be thrown saying that `PackedAllocate` is not implemented for `SomeStruct`. To fix this, implement `PackedAllocate` for that struct
-

### misc steps

- place Solidity class variables to proper places in to the contract mod
  - e.g. in struct, in constants
- convert datatypes to rust compatible, and swap from datatype first to datatype after (e.g., uint128 var -> var: u128)

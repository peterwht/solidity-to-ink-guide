# Payment Splitter

Based on https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/finance/PaymentSplitter.sol

## Overview

This contract allows payments (based on the local chain currency) to be split among a group of accounts.

Every ink! contract has an associated address (an `AccountID`). This address can be paid like any
normal address. Payments sent to this contract address will be held by the contract (not automatically distributed).
Shareholders are able to request that their portion of the funds are `released` (tranferred) from the contract
into their own account.

Contract instantiation requires a list of `payees` and a list of `shares`. `payees` and `shares` need to have the same length
(`payee[0]` will be assigned `shares[0]`). The amount of shares each payee has determines what portion of the payments
belongs to the payee. Adding payees is only possible during contract instantiation

## Notes

This contract is missing some of the functionality present in OpenZeppelin's version.
In addition to the normal payments, OpenZeppelin's contract implements an ERC20 interface.
This interface allows payees to also receive ERC20 tokens (based on the ERC20 contract's address).

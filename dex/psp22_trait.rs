//! PSP22 Trait Reference for Cross-Contract Calls
//!
//! Defines the PSP22 token interface for making cross-contract calls from DEX contracts.

use ink::primitives::AccountId;

/// PSP22 Error types
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum PSP22Error {
    InsufficientBalance,
    InsufficientAllowance,
    InvalidRecipient,
    Custom(ink::prelude::string::String),
}

pub type Result<T> = core::result::Result<T, PSP22Error>;

/// PSP22 Token Interface
///
/// Minimal interface needed for DEX operations
#[ink::trait_definition]
pub trait PSP22 {
    /// Returns the balance of the given account
    #[ink(message)]
    fn balance_of(&self, owner: AccountId) -> u128;

    /// Transfers tokens to the recipient
    #[ink(message)]
    fn transfer(&mut self, to: AccountId, value: u128) -> Result<()>;

    /// Transfers tokens from one account to another (requires allowance)
    #[ink(message)]
    fn transfer_from(&mut self, from: AccountId, to: AccountId, value: u128) -> Result<()>;

    /// Approves spender to spend tokens
    #[ink(message)]
    fn approve(&mut self, spender: AccountId, value: u128) -> Result<()>;

    /// Returns the allowance granted by owner to spender
    #[ink(message)]
    fn allowance(&self, owner: AccountId, spender: AccountId) -> u128;
}

#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// # DALLA Token - PSP22 Compliant
///
/// The official wrapped token for BelizeChain's native DALLA currency.
/// Implements the PSP22 standard (Polkadot's ERC20 equivalent).
///
/// ## Features
/// - PSP22 standard compliance (transfer, approve, transferFrom)
/// - Minting and burning (controlled by owner)
/// - Total supply tracking
/// - Event emission for all operations
/// - Allowance management
///
/// ## Economics
/// - Symbol: DALLA
/// - Decimals: 12 (same as native DALLA)
/// - Max Supply: 100 million DALLA (100_000_000 * 10^12)
/// - Initial Supply: 21 million DALLA (like Bitcoin's 21M)

#[ink::contract]
mod dalla_token {
    use ink::prelude::string::String;
    use ink::storage::Mapping;

    /// The DALLA token error types
    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        /// Insufficient balance for transfer
        InsufficientBalance,
        /// Insufficient allowance for transfer_from
        InsufficientAllowance,
        /// Transfer to zero address
        InvalidRecipient,
        /// Mint/burn only allowed by owner
        UnauthorizedAccess,
        /// Minting would exceed max supply
        ExceedsMaxSupply,
        /// Arithmetic overflow
        Overflow,
    }

    /// Result type for DALLA operations
    pub type Result<T> = core::result::Result<T, Error>;

    /// The DALLA token storage
    #[ink(storage)]
    pub struct DallaToken {
        /// Total supply of DALLA tokens
        total_supply: u128,
        /// Maximum supply cap (100M DALLA)
        max_supply: u128,
        /// Mapping from account to balance
        balances: Mapping<AccountId, u128>,
        /// Mapping from (owner, spender) to allowance
        allowances: Mapping<(AccountId, AccountId), u128>,
        /// Contract owner (can mint/burn)
        owner: AccountId,
    }

    /// Event emitted when tokens are transferred
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: u128,
    }

    /// Event emitted when allowance is approved
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: u128,
    }

    impl DallaToken {
        /// Creates a new DALLA token contract with initial supply
        #[ink(constructor)]
        pub fn new(initial_supply: u128) -> Self {
            let caller = Self::env().caller();
            let max_supply = 100_000_000_000_000_000_000_u128; // 100M DALLA

            let mut balances = Mapping::default();
            balances.insert(caller, &initial_supply);

            Self::env().emit_event(Transfer {
                from: None,
                to: Some(caller),
                value: initial_supply,
            });

            Self {
                total_supply: initial_supply,
                max_supply,
                balances,
                allowances: Mapping::default(),
                owner: caller,
            }
        }

        /// Returns the token name
        #[ink(message)]
        pub fn token_name(&self) -> String {
            String::from("DALLA Token")
        }

        /// Returns the token symbol
        #[ink(message)]
        pub fn token_symbol(&self) -> String {
            String::from("DALLA")
        }

        /// Returns the token decimals
        #[ink(message)]
        pub fn token_decimals(&self) -> u8 {
            12
        }

        /// Returns the total supply
        #[ink(message)]
        pub fn total_supply(&self) -> u128 {
            self.total_supply
        }

        /// Returns the maximum supply cap
        #[ink(message)]
        pub fn max_supply(&self) -> u128 {
            self.max_supply
        }

        /// Returns the balance of an account
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u128 {
            self.balances.get(owner).unwrap_or(0)
        }

        /// Returns the allowance granted by owner to spender
        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> u128 {
            self.allowances.get((owner, spender)).unwrap_or(0)
        }

        /// Transfers tokens from caller to recipient
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: u128) -> Result<()> {
            let from = self.env().caller();
            self.transfer_from_to(from, to, value)
        }

        /// Approves spender to spend tokens on behalf of caller
        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, value: u128) -> Result<()> {
            let owner = self.env().caller();
            self.allowances.insert((owner, spender), &value);

            self.env().emit_event(Approval {
                owner,
                spender,
                value,
            });

            Ok(())
        }

        /// Transfers tokens from one account to another using allowance
        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, value: u128) -> Result<()> {
            let caller = self.env().caller();
            let allowance = self.allowance(from, caller);

            if allowance < value {
                return Err(Error::InsufficientAllowance);
            }

            self.transfer_from_to(from, to, value)?;

            // Decrease allowance
            let new_allowance = allowance.saturating_sub(value);
            self.allowances.insert((from, caller), &new_allowance);

            Ok(())
        }

        /// Increases the allowance granted to spender
        #[ink(message)]
        pub fn increase_allowance(&mut self, spender: AccountId, delta: u128) -> Result<()> {
            let owner = self.env().caller();
            let allowance = self.allowance(owner, spender);
            let new_allowance = allowance.checked_add(delta).ok_or(Error::Overflow)?;

            self.allowances.insert((owner, spender), &new_allowance);

            self.env().emit_event(Approval {
                owner,
                spender,
                value: new_allowance,
            });

            Ok(())
        }

        /// Decreases the allowance granted to spender
        #[ink(message)]
        pub fn decrease_allowance(&mut self, spender: AccountId, delta: u128) -> Result<()> {
            let owner = self.env().caller();
            let allowance = self.allowance(owner, spender);

            if allowance < delta {
                return Err(Error::InsufficientAllowance);
            }

            let new_allowance = allowance.saturating_sub(delta);
            self.allowances.insert((owner, spender), &new_allowance);

            self.env().emit_event(Approval {
                owner,
                spender,
                value: new_allowance,
            });

            Ok(())
        }

        /// Mints new tokens (owner only)
        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, value: u128) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::UnauthorizedAccess);
            }

            let new_supply = self
                .total_supply
                .checked_add(value)
                .ok_or(Error::Overflow)?;
            if new_supply > self.max_supply {
                return Err(Error::ExceedsMaxSupply);
            }

            let balance = self.balance_of(to);
            let new_balance = balance.checked_add(value).ok_or(Error::Overflow)?;

            self.balances.insert(to, &new_balance);
            self.total_supply = new_supply;

            self.env().emit_event(Transfer {
                from: None,
                to: Some(to),
                value,
            });

            Ok(())
        }

        /// Burns tokens from caller's balance
        #[ink(message)]
        pub fn burn(&mut self, value: u128) -> Result<()> {
            let caller = self.env().caller();
            let balance = self.balance_of(caller);

            if balance < value {
                return Err(Error::InsufficientBalance);
            }

            let new_balance = balance.saturating_sub(value);
            self.balances.insert(caller, &new_balance);
            self.total_supply = self.total_supply.saturating_sub(value);

            self.env().emit_event(Transfer {
                from: Some(caller),
                to: None,
                value,
            });

            Ok(())
        }

        /// Transfers ownership of the contract
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::UnauthorizedAccess);
            }

            self.owner = new_owner;
            Ok(())
        }

        /// Returns the contract owner
        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner
        }

        /// Internal transfer function
        fn transfer_from_to(&mut self, from: AccountId, to: AccountId, value: u128) -> Result<()> {
            let from_balance = self.balance_of(from);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            let to_balance = self.balance_of(to);
            let new_to_balance = to_balance.checked_add(value).ok_or(Error::Overflow)?;

            let new_from_balance = from_balance.saturating_sub(value);
            self.balances.insert(from, &new_from_balance);
            self.balances.insert(to, &new_to_balance);

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value,
            });

            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn new_works() {
            let initial_supply = 21_000_000_000_000_000_000_u128; // 21M DALLA
            let token = DallaToken::new(initial_supply);

            assert_eq!(token.total_supply(), initial_supply);
            assert_eq!(token.token_symbol(), "DALLA");
            assert_eq!(token.token_decimals(), 12);
        }

        #[ink::test]
        fn balance_of_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let initial_supply = 1_000_000_000_000_u128; // 1M DALLA
            let token = DallaToken::new(initial_supply);

            assert_eq!(token.balance_of(accounts.alice), initial_supply);
            assert_eq!(token.balance_of(accounts.bob), 0);
        }

        #[ink::test]
        fn transfer_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.transfer(accounts.bob, 100_000_000_000).is_ok());
            assert_eq!(token.balance_of(accounts.alice), 900_000_000_000);
            assert_eq!(token.balance_of(accounts.bob), 100_000_000_000);
        }

        #[ink::test]
        fn transfer_fails_insufficient_balance() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut token = DallaToken::new(100_000_000_000_u128);

            let result = token.transfer(accounts.bob, 200_000_000_000);
            assert_eq!(result, Err(Error::InsufficientBalance));
        }

        #[ink::test]
        fn approve_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.approve(accounts.bob, 100_000_000_000).is_ok());
            assert_eq!(
                token.allowance(accounts.alice, accounts.bob),
                100_000_000_000
            );
        }

        #[ink::test]
        fn transfer_from_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            // Alice approves Bob to spend 100 DALLA
            assert!(token.approve(accounts.bob, 100_000_000_000).is_ok());

            // Change caller to Bob
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

            // Bob transfers from Alice to Charlie
            assert!(token
                .transfer_from(accounts.alice, accounts.charlie, 50_000_000_000)
                .is_ok());

            assert_eq!(token.balance_of(accounts.alice), 950_000_000_000);
            assert_eq!(token.balance_of(accounts.charlie), 50_000_000_000);
            assert_eq!(
                token.allowance(accounts.alice, accounts.bob),
                50_000_000_000
            );
        }

        #[ink::test]
        fn mint_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.mint(accounts.bob, 500_000_000_000).is_ok());
            assert_eq!(token.total_supply(), 1_500_000_000_000);
            assert_eq!(token.balance_of(accounts.bob), 500_000_000_000);
        }

        #[ink::test]
        fn mint_fails_exceeds_max_supply() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            let result = token.mint(accounts.bob, 100_000_000_000_000_000_000_u128);
            assert_eq!(result, Err(Error::ExceedsMaxSupply));
        }

        #[ink::test]
        fn burn_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.burn(200_000_000_000).is_ok());
            assert_eq!(token.total_supply(), 800_000_000_000);
            assert_eq!(token.balance_of(accounts.alice), 800_000_000_000);
        }

        #[ink::test]
        fn increase_allowance_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.approve(accounts.bob, 100_000_000_000).is_ok());
            assert!(token
                .increase_allowance(accounts.bob, 50_000_000_000)
                .is_ok());
            assert_eq!(
                token.allowance(accounts.alice, accounts.bob),
                150_000_000_000
            );
        }

        #[ink::test]
        fn decrease_allowance_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.approve(accounts.bob, 100_000_000_000).is_ok());
            assert!(token
                .decrease_allowance(accounts.bob, 30_000_000_000)
                .is_ok());
            assert_eq!(
                token.allowance(accounts.alice, accounts.bob),
                70_000_000_000
            );
        }
    }
}

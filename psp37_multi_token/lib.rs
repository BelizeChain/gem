#![cfg_attr(not(feature = "std"), no_std, no_main)]

//! # PSP37 Multi-Token Standard
//!
//! A multi-token standard that can represent multiple fungible and non-fungible tokens
//! in a single contract. Similar to ERC1155 in Ethereum.
//!
//! ## Features
//! - Single contract for multiple token types
//! - Mixed fungible and non-fungible tokens
//! - Batch transfer operations (gas efficient)
//! - Approval for operators (delegates)
//! - Token URI metadata support
//!
//! ## Use Cases
//! - Game items (100 swords, 50 shields, 1 legendary sword)
//! - Event tickets (1000 general, 100 VIP, 10 backstage passes)
//! - Multi-asset platforms
//! - Fractionalized NFTs

#[ink::contract]
mod psp37_multi_token {
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use scale::{Decode, Encode};

    /// Token ID type (u128 allows 2^128 unique token types)
    pub type TokenId = u128;

    /// Amount type for token balances (use u128 for compatibility)
    pub type TokenBalance = u128;

    // ============================================================================
    // Storage
    // ============================================================================

    #[ink(storage)]
    pub struct Psp37MultiToken {
        /// Balances mapping: (owner, token_id) => balance
        balances: Mapping<(AccountId, TokenId), Balance>,

        /// Operator approvals: (owner, operator) => approved
        /// Operators can transfer ANY token on behalf of owner
        operator_approvals: Mapping<(AccountId, AccountId), bool>,

        /// Total supply per token ID
        total_supply: Mapping<TokenId, Balance>,

        /// Token URIs for metadata (optional)
        token_uris: Mapping<TokenId, String>,

        /// Contract owner (for minting control)
        owner: AccountId,

        /// Next token ID for auto-increment
        next_token_id: TokenId,
    }

    // ============================================================================
    // Events
    // ============================================================================

    #[ink(event)]
    pub struct TransferSingle {
        #[ink(topic)]
        operator: Option<AccountId>,
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        token_id: TokenId,
        value: TokenBalance,
    }

    #[ink(event)]
    pub struct TransferBatch {
        #[ink(topic)]
        operator: Option<AccountId>,
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        token_ids: Vec<TokenId>,
        values: Vec<Balance>,
    }

    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        approved: bool,
    }

    #[ink(event)]
    pub struct TokenCreated {
        #[ink(topic)]
        token_id: TokenId,
        initial_supply: TokenBalance,
        uri: Option<String>,
    }

    // ============================================================================
    // Errors
    // ============================================================================

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Insufficient balance for transfer
        InsufficientBalance,
        /// Not authorized to perform operation
        NotAuthorized,
        /// Array length mismatch (token_ids.len() != values.len())
        ArrayLengthMismatch,
        /// Safe transfer rejection (receiver rejected the transfer)
        TransferRejected,
        /// Token ID does not exist
        TokenNotFound,
        /// Zero address not allowed
        ZeroAddress,
        /// Self-approval not allowed
        SelfApproval,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    // ============================================================================
    // Implementation
    // ============================================================================

    impl Psp37MultiToken {
        // ========================================================================
        // Constructor
        // ========================================================================

        /// Create a new PSP37 multi-token contract
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                balances: Mapping::default(),
                operator_approvals: Mapping::default(),
                total_supply: Mapping::default(),
                token_uris: Mapping::default(),
                owner: Self::env().caller(),
                next_token_id: 1,
            }
        }

        // ========================================================================
        // PSP37 Core Functions
        // ========================================================================

        /// Get balance of account for specific token
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId, token_id: TokenId) -> Balance {
            self.balances.get((owner, token_id)).unwrap_or(0)
        }

        /// Get balances for multiple token IDs (batch operation)
        #[ink(message)]
        pub fn balance_of_batch(
            &self,
            owners: Vec<AccountId>,
            token_ids: Vec<TokenId>,
        ) -> Result<Vec<Balance>> {
            if owners.len() != token_ids.len() {
                return Err(Error::ArrayLengthMismatch);
            }

            let mut balances = Vec::new();
            for (owner, token_id) in owners.iter().zip(token_ids.iter()) {
                balances.push(self.balance_of(*owner, *token_id));
            }
            Ok(balances)
        }

        /// Transfer tokens from caller to another account
        #[ink(message)]
        pub fn transfer(
            &mut self,
            to: AccountId,
            token_id: TokenId,
            value: TokenBalance,
        ) -> Result<()> {
            let caller = self.env().caller();
            self._transfer_from(caller, caller, to, token_id, value)
        }

        /// Transfer tokens from one account to another (requires approval)
        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_id: TokenId,
            value: TokenBalance,
        ) -> Result<()> {
            let caller = self.env().caller();
            self._transfer_from(caller, from, to, token_id, value)
        }

        /// Batch transfer multiple tokens at once (gas efficient)
        #[ink(message)]
        pub fn batch_transfer(
            &mut self,
            to: AccountId,
            token_ids: Vec<TokenId>,
            values: Vec<Balance>,
        ) -> Result<()> {
            let caller = self.env().caller();
            self._batch_transfer_from(caller, caller, to, token_ids, values)
        }

        /// Batch transfer from another account (requires approval)
        #[ink(message)]
        pub fn batch_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_ids: Vec<TokenId>,
            values: Vec<Balance>,
        ) -> Result<()> {
            let caller = self.env().caller();
            self._batch_transfer_from(caller, from, to, token_ids, values)
        }

        /// Approve or revoke operator to manage all tokens of caller
        #[ink(message)]
        pub fn set_approval_for_all(&mut self, operator: AccountId, approved: bool) -> Result<()> {
            let caller = self.env().caller();

            if caller == operator {
                return Err(Error::SelfApproval);
            }

            self.operator_approvals
                .insert((caller, operator), &approved);

            self.env().emit_event(ApprovalForAll {
                owner: caller,
                operator,
                approved,
            });

            Ok(())
        }

        /// Check if operator is approved for owner
        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.operator_approvals
                .get((owner, operator))
                .unwrap_or(false)
        }

        // ========================================================================
        // PSP37 Metadata Extension
        // ========================================================================

        /// Get total supply of a specific token
        #[ink(message)]
        pub fn total_supply(&self, token_id: TokenId) -> Balance {
            self.total_supply.get(token_id).unwrap_or(0)
        }

        /// Get token URI (metadata link)
        #[ink(message)]
        pub fn token_uri(&self, token_id: TokenId) -> Option<String> {
            self.token_uris.get(token_id)
        }

        // ========================================================================
        // PSP37 Mintable Extension
        // ========================================================================

        /// Create a new token type and mint initial supply (owner only)
        #[ink(message)]
        pub fn create_token(
            &mut self,
            initial_supply: TokenBalance,
            uri: Option<String>,
        ) -> Result<TokenId> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            let token_id = self.next_token_id;
            self.next_token_id = self.next_token_id.saturating_add(1);

            // Mint initial supply to creator
            if initial_supply > 0 {
                self._mint(caller, token_id, initial_supply)?;
            }

            // Set URI if provided
            if let Some(uri_value) = uri.clone() {
                self.token_uris.insert(token_id, &uri_value);
            }

            self.env().emit_event(TokenCreated {
                token_id,
                initial_supply,
                uri,
            });

            Ok(token_id)
        }

        /// Mint additional tokens of existing type (owner only)
        #[ink(message)]
        pub fn mint(
            &mut self,
            to: AccountId,
            token_id: TokenId,
            amount: TokenBalance,
        ) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            self._mint(to, token_id, amount)
        }

        /// Batch mint multiple tokens (owner only)
        #[ink(message)]
        pub fn batch_mint(
            &mut self,
            to: AccountId,
            token_ids: Vec<TokenId>,
            amounts: Vec<Balance>,
        ) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            if token_ids.len() != amounts.len() {
                return Err(Error::ArrayLengthMismatch);
            }

            for (token_id, amount) in token_ids.iter().zip(amounts.iter()) {
                self._mint(to, *token_id, *amount)?;
            }

            Ok(())
        }

        // ========================================================================
        // PSP37 Burnable Extension
        // ========================================================================

        /// Burn tokens (reduce supply)
        #[ink(message)]
        pub fn burn(&mut self, token_id: TokenId, amount: TokenBalance) -> Result<()> {
            let caller = self.env().caller();
            self._burn(caller, token_id, amount)
        }

        /// Burn tokens from another account (requires approval)
        #[ink(message)]
        pub fn burn_from(
            &mut self,
            from: AccountId,
            token_id: TokenId,
            amount: TokenBalance,
        ) -> Result<()> {
            let caller = self.env().caller();

            // Check authorization
            if caller != from && !self.is_approved_for_all(from, caller) {
                return Err(Error::NotAuthorized);
            }

            self._burn(from, token_id, amount)
        }

        // ========================================================================
        // Admin Functions
        // ========================================================================

        /// Get contract owner
        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner
        }

        /// Transfer ownership (owner only)
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            self.owner = new_owner;
            Ok(())
        }

        /// Set token URI (owner only)
        #[ink(message)]
        pub fn set_token_uri(&mut self, token_id: TokenId, uri: String) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            self.token_uris.insert(token_id, &uri);
            Ok(())
        }

        // ========================================================================
        // Internal Functions
        // ========================================================================

        /// Internal transfer implementation
        fn _transfer_from(
            &mut self,
            operator: AccountId,
            from: AccountId,
            to: AccountId,
            token_id: TokenId,
            value: TokenBalance,
        ) -> Result<()> {
            // Validate addresses
            if to == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            // Check authorization
            if operator != from && !self.is_approved_for_all(from, operator) {
                return Err(Error::NotAuthorized);
            }

            // Check balance
            let from_balance = self.balance_of(from, token_id);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            // Update balances
            self.balances
                .insert((from, token_id), &(from_balance - value));

            let to_balance = self.balance_of(to, token_id);
            self.balances
                .insert((to, token_id), &(to_balance.saturating_add(value)));

            // Emit event
            self.env().emit_event(TransferSingle {
                operator: Some(operator),
                from: Some(from),
                to: Some(to),
                token_id,
                value,
            });

            Ok(())
        }

        /// Internal batch transfer implementation
        fn _batch_transfer_from(
            &mut self,
            operator: AccountId,
            from: AccountId,
            to: AccountId,
            token_ids: Vec<TokenId>,
            values: Vec<Balance>,
        ) -> Result<()> {
            // Validate inputs
            if token_ids.len() != values.len() {
                return Err(Error::ArrayLengthMismatch);
            }

            if to == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            // Check authorization
            if operator != from && !self.is_approved_for_all(from, operator) {
                return Err(Error::NotAuthorized);
            }

            // Transfer each token
            for (token_id, value) in token_ids.iter().zip(values.iter()) {
                let from_balance = self.balance_of(from, *token_id);
                if from_balance < *value {
                    return Err(Error::InsufficientBalance);
                }

                self.balances
                    .insert((from, *token_id), &(from_balance - *value));

                let to_balance = self.balance_of(to, *token_id);
                self.balances
                    .insert((to, *token_id), &(to_balance.saturating_add(*value)));
            }

            // Emit event
            self.env().emit_event(TransferBatch {
                operator: Some(operator),
                from: Some(from),
                to: Some(to),
                token_ids,
                values,
            });

            Ok(())
        }

        /// Internal mint implementation
        fn _mint(&mut self, to: AccountId, token_id: TokenId, amount: TokenBalance) -> Result<()> {
            if to == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            // Update balance
            let balance = self.balance_of(to, token_id);
            self.balances
                .insert((to, token_id), &(balance.saturating_add(amount)));

            // Update total supply
            let supply = self.total_supply(token_id);
            self.total_supply
                .insert(token_id, &(supply.saturating_add(amount)));

            // Emit event
            self.env().emit_event(TransferSingle {
                operator: Some(self.env().caller()),
                from: None,
                to: Some(to),
                token_id,
                value: amount,
            });

            Ok(())
        }

        /// Internal burn implementation
        fn _burn(
            &mut self,
            from: AccountId,
            token_id: TokenId,
            amount: TokenBalance,
        ) -> Result<()> {
            // Check balance
            let balance = self.balance_of(from, token_id);
            if balance < amount {
                return Err(Error::InsufficientBalance);
            }

            // Update balance
            self.balances.insert((from, token_id), &(balance - amount));

            // Update total supply
            let supply = self.total_supply(token_id);
            self.total_supply.insert(token_id, &(supply - amount));

            // Emit event
            self.env().emit_event(TransferSingle {
                operator: Some(self.env().caller()),
                from: Some(from),
                to: None,
                token_id,
                value: amount,
            });

            Ok(())
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn new_works() {
            let contract = Psp37MultiToken::new();
            assert_eq!(contract.next_token_id, 1);
        }

        #[ink::test]
        fn create_token_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let token_id = contract
                .create_token(1000, Some("https://example.com/token/1".into()))
                .unwrap();

            assert_eq!(token_id, 1);
            assert_eq!(contract.balance_of(accounts.alice, token_id), 1000);
            assert_eq!(contract.total_supply(token_id), 1000);
            assert_eq!(
                contract.token_uri(token_id),
                Some("https://example.com/token/1".into())
            );
        }

        #[ink::test]
        fn transfer_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let token_id = contract.create_token(1000, None).unwrap();

            // Transfer 100 tokens
            assert!(contract.transfer(accounts.bob, token_id, 100).is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token_id), 900);
            assert_eq!(contract.balance_of(accounts.bob, token_id), 100);
        }

        #[ink::test]
        fn transfer_fails_insufficient_balance() {
            let mut contract = Psp37MultiToken::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let token_id = contract.create_token(100, None).unwrap();

            // Try to transfer more than balance
            assert_eq!(
                contract.transfer(accounts.bob, token_id, 1000),
                Err(Error::InsufficientBalance)
            );
        }

        #[ink::test]
        fn approval_for_all_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Approve operator
            assert!(contract.set_approval_for_all(accounts.bob, true).is_ok());
            assert!(contract.is_approved_for_all(accounts.alice, accounts.bob));

            // Revoke approval
            assert!(contract.set_approval_for_all(accounts.bob, false).is_ok());
            assert!(!contract.is_approved_for_all(accounts.alice, accounts.bob));
        }

        #[ink::test]
        fn transfer_from_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let token_id = contract.create_token(1000, None).unwrap();

            // Approve operator
            contract.set_approval_for_all(accounts.bob, true).unwrap();

            // Change caller to Bob
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

            // Bob transfers Alice's tokens
            assert!(contract
                .transfer_from(accounts.alice, accounts.charlie, token_id, 100)
                .is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token_id), 900);
            assert_eq!(contract.balance_of(accounts.charlie, token_id), 100);
        }

        #[ink::test]
        fn batch_transfer_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Create multiple token types
            let token1 = contract.create_token(1000, None).unwrap();
            let token2 = contract.create_token(2000, None).unwrap();

            // Batch transfer
            let token_ids = vec![token1, token2];
            let values = vec![100, 200];

            assert!(contract
                .batch_transfer(accounts.bob, token_ids.clone(), values.clone())
                .is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token1), 900);
            assert_eq!(contract.balance_of(accounts.alice, token2), 1800);
            assert_eq!(contract.balance_of(accounts.bob, token1), 100);
            assert_eq!(contract.balance_of(accounts.bob, token2), 200);
        }

        #[ink::test]
        fn batch_mint_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Create token types without initial supply
            let token1 = contract.create_token(0, None).unwrap();
            let token2 = contract.create_token(0, None).unwrap();

            // Batch mint
            let token_ids = vec![token1, token2];
            let amounts = vec![500, 1000];

            assert!(contract
                .batch_mint(accounts.bob, token_ids, amounts)
                .is_ok());
            assert_eq!(contract.balance_of(accounts.bob, token1), 500);
            assert_eq!(contract.balance_of(accounts.bob, token2), 1000);
        }

        #[ink::test]
        fn burn_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let token_id = contract.create_token(1000, None).unwrap();

            // Burn 100 tokens
            assert!(contract.burn(token_id, 100).is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token_id), 900);
            assert_eq!(contract.total_supply(token_id), 900);
        }

        #[ink::test]
        fn balance_of_batch_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let token1 = contract.create_token(1000, None).unwrap();
            let token2 = contract.create_token(2000, None).unwrap();

            let owners = vec![accounts.alice, accounts.alice];
            let token_ids = vec![token1, token2];

            let balances = contract.balance_of_batch(owners, token_ids).unwrap();
            assert_eq!(balances, vec![1000, 2000]);
        }
    }
}

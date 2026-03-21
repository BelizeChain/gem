#![cfg_attr(not(feature = "std"), no_std, no_main)]

//! # PSP37 Multi-Token Standard
//!
//! A multi-token standard that can represent multiple fungible and non-fungible tokens
//! in a single contract. Similar to ERC1155 in Ethereum.
//!
//! ## Features
//! - Single contract for multiple token types
//! - Mixed fungible and non-fungible tokens with enforced type invariants
//! - Batch transfer operations (gas efficient, size-capped)
//! - Per-token-ID amount-based approvals (PSP37 compliant)
//! - Operator approvals for blanket delegation
//! - Token URI metadata support
//! - Maximum supply caps per token ID
//! - Two-step ownership transfer
//!
//! ## Use Cases
//! - Game items (100 swords, 50 shields, 1 legendary sword)
//! - Event tickets (1000 general, 100 VIP, 10 backstage passes)
//! - Multi-asset platforms
//! - Fractionalized NFTs
//!
//! ## Security Properties
//! - Non-fungible tokens enforce balance ∈ {0, 1} per owner and total_supply ≤ 1
//! - Token type is immutable after creation
//! - All arithmetic uses checked operations — overflow returns an error
//! - Batch operations are capped at MAX_BATCH_SIZE (50) entries
//! - Burn-from requires separate burn approval, not transfer approval

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

    /// Maximum number of items in any batch operation (GEM-05-H01)
    const MAX_BATCH_SIZE: usize = 50;

    // ============================================================================
    // Token Type (GEM-05-C01)
    // ============================================================================

    /// Distinguishes fungible from non-fungible token IDs.
    /// Immutable after creation — a token's type cannot be changed.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub enum TokenType {
        /// Fungible token — balance can be any value up to max_supply
        Fungible,
        /// Non-fungible token — exactly one unit exists, balance ∈ {0, 1} per owner
        NonFungible,
    }

    // ============================================================================
    // Storage
    // ============================================================================

    #[ink(storage)]
    pub struct Psp37MultiToken {
        /// Balances mapping: (owner, token_id) => balance
        balances: Mapping<(AccountId, TokenId), Balance>,

        /// Operator approvals: (owner, operator) => approved for ALL tokens
        operator_approvals: Mapping<(AccountId, AccountId), bool>,

        /// Per-token-ID amount-based approvals: (owner, operator, token_id) => allowance (GEM-05-C02)
        #[allow(clippy::type_complexity)]
        token_approvals: Mapping<(AccountId, AccountId, TokenId), Balance>,

        /// Burn approvals: (owner, operator) => approved to burn (GEM-05-H03)
        /// Separate from transfer approvals — an operator approved for transfers
        /// cannot burn unless explicitly granted burn approval.
        burn_approvals: Mapping<(AccountId, AccountId), bool>,

        /// Total supply per token ID
        total_supply: Mapping<TokenId, Balance>,

        /// Token type registry: token_id => TokenType (GEM-05-C01)
        /// Immutable after creation.
        token_types: Mapping<TokenId, TokenType>,

        /// Maximum supply cap per token ID (GEM-05-M01)
        /// None = no cap (fungible default). NonFungible tokens always have implicit cap of 1.
        max_supply: Mapping<TokenId, Balance>,

        /// Token URIs for metadata (optional)
        token_uris: Mapping<TokenId, String>,

        /// Contract owner (for minting control)
        owner: AccountId,

        /// Pending owner for two-step ownership transfer (GEM-05-M02)
        pending_owner: Option<AccountId>,

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

    /// Emitted when a per-token-ID allowance is set (GEM-05-C02)
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        token_id: TokenId,
        value: Balance,
    }

    /// Emitted when burn approval is granted or revoked (GEM-05-H03)
    #[ink(event)]
    pub struct BurnApproval {
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
        token_type: TokenType,
        initial_supply: TokenBalance,
        max_supply: Option<Balance>,
        uri: Option<String>,
    }

    /// Emitted on ownership transfer (GEM-05-M04)
    #[ink(event)]
    pub struct OwnershipTransferred {
        #[ink(topic)]
        previous_owner: AccountId,
        #[ink(topic)]
        new_owner: AccountId,
    }

    /// Emitted on code hash upgrade (GEM-05-M03)
    #[ink(event)]
    pub struct CodeHashUpdated {
        #[ink(topic)]
        new_code_hash: Hash,
        caller: AccountId,
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
        /// Token ID does not exist (not created via create_token)
        TokenNotFound,
        /// Zero address not allowed
        ZeroAddress,
        /// Self-approval not allowed
        SelfApproval,
        /// Batch exceeds MAX_BATCH_SIZE (GEM-05-H01)
        BatchTooLarge,
        /// Arithmetic overflow on balance or supply (GEM-05-H02)
        Overflow,
        /// Minting would exceed the token's maximum supply cap (GEM-05-M01)
        SupplyCapExceeded,
        /// Token ID counter exhausted — cannot create more tokens (GEM-05-L04)
        TokenIdOverflow,
        /// Non-fungible token already minted — total supply must not exceed 1 (GEM-05-C01)
        NonFungibleDuplicate,
        /// Non-fungible transfer value must be exactly 1 (GEM-05-C01)
        NonFungibleValueInvalid,
        /// Insufficient per-token allowance for transfer_from (GEM-05-C02)
        InsufficientAllowance,
        /// Not authorized to burn from another account (GEM-05-H03)
        BurnNotAuthorized,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    // ============================================================================
    // Implementation
    // ============================================================================

    #[allow(clippy::new_without_default)]
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
                token_approvals: Mapping::default(),
                burn_approvals: Mapping::default(),
                total_supply: Mapping::default(),
                token_types: Mapping::default(),
                max_supply: Mapping::default(),
                token_uris: Mapping::default(),
                owner: Self::env().caller(),
                pending_owner: None,
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
            if owners.len() > MAX_BATCH_SIZE {
                return Err(Error::BatchTooLarge);
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

        /// Transfer tokens from one account to another (requires per-token allowance or operator approval)
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

        /// Batch transfer multiple tokens at once (gas efficient, capped at MAX_BATCH_SIZE)
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

        /// Batch transfer from another account (requires approval, capped at MAX_BATCH_SIZE)
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

        // ========================================================================
        // PSP37 Approval Functions (GEM-05-C02)
        // ========================================================================

        /// Approve or revoke operator to manage ALL tokens of caller (blanket approval)
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

        /// Approve a specific allowance for an operator on a specific token ID.
        /// Set value to 0 to revoke. This does NOT grant burn rights — use
        /// `set_burn_approval` separately.
        #[ink(message)]
        pub fn approve(
            &mut self,
            operator: AccountId,
            token_id: TokenId,
            value: Balance,
        ) -> Result<()> {
            let caller = self.env().caller();

            if caller == operator {
                return Err(Error::SelfApproval);
            }

            self.token_approvals
                .insert((caller, operator, token_id), &value);

            self.env().emit_event(Approval {
                owner: caller,
                operator,
                token_id,
                value,
            });

            Ok(())
        }

        /// Query the per-token allowance for an operator
        #[ink(message)]
        pub fn allowance(
            &self,
            owner: AccountId,
            operator: AccountId,
            token_id: TokenId,
        ) -> Balance {
            self.token_approvals
                .get((owner, operator, token_id))
                .unwrap_or(0)
        }

        /// Check if operator is approved for all tokens of owner (blanket approval)
        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.operator_approvals
                .get((owner, operator))
                .unwrap_or(false)
        }

        /// Approve or revoke an operator's right to burn caller's tokens (GEM-05-H03).
        /// This is separate from transfer approval.
        #[ink(message)]
        pub fn set_burn_approval(&mut self, operator: AccountId, approved: bool) -> Result<()> {
            let caller = self.env().caller();

            if caller == operator {
                return Err(Error::SelfApproval);
            }

            self.burn_approvals.insert((caller, operator), &approved);

            self.env().emit_event(BurnApproval {
                owner: caller,
                operator,
                approved,
            });

            Ok(())
        }

        /// Check if operator is approved to burn owner's tokens
        #[ink(message)]
        pub fn is_burn_approved(&self, owner: AccountId, operator: AccountId) -> bool {
            self.burn_approvals.get((owner, operator)).unwrap_or(false)
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

        /// Get the token type (Fungible or NonFungible). Returns None if token doesn't exist.
        #[ink(message)]
        pub fn token_type(&self, token_id: TokenId) -> Option<TokenType> {
            self.token_types.get(token_id)
        }

        /// Get the maximum supply cap for a token. Returns None if no cap is set.
        /// NonFungible tokens always have an implicit cap of 1.
        #[ink(message)]
        pub fn get_max_supply(&self, token_id: TokenId) -> Option<Balance> {
            match self.token_types.get(token_id) {
                Some(TokenType::NonFungible) => Some(1),
                Some(TokenType::Fungible) => self.max_supply.get(token_id),
                None => None,
            }
        }

        // ========================================================================
        // PSP37 Mintable Extension
        // ========================================================================

        /// Create a new token type and mint initial supply (owner only).
        ///
        /// `token_type` — Fungible or NonFungible. Immutable after creation.
        /// `max_supply_cap` — Optional maximum supply for fungible tokens. Ignored for
        ///   NonFungible tokens (always capped at 1). None = no cap.
        /// `initial_supply` — For NonFungible, must be 0 or 1.
        /// `uri` — Optional metadata URI.
        #[ink(message)]
        pub fn create_token(
            &mut self,
            token_type: TokenType,
            initial_supply: TokenBalance,
            max_supply_cap: Option<Balance>,
            uri: Option<String>,
        ) -> Result<TokenId> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            // Allocate token ID with overflow check (GEM-05-L04)
            let token_id = self.next_token_id;
            self.next_token_id = self
                .next_token_id
                .checked_add(1)
                .ok_or(Error::TokenIdOverflow)?;

            // Register token type — immutable after this point (GEM-05-C01)
            self.token_types.insert(token_id, &token_type);

            // Set max supply cap (GEM-05-M01)
            match token_type {
                TokenType::NonFungible => {
                    // NFTs have implicit cap of 1 — validate initial_supply
                    if initial_supply > 1 {
                        return Err(Error::NonFungibleDuplicate);
                    }
                    // Store cap = 1 explicitly for NFTs
                    self.max_supply.insert(token_id, &1);
                }
                TokenType::Fungible => {
                    if let Some(cap) = max_supply_cap {
                        if initial_supply > cap {
                            return Err(Error::SupplyCapExceeded);
                        }
                        self.max_supply.insert(token_id, &cap);
                    }
                }
            }

            // Mint initial supply to creator
            if initial_supply > 0 {
                self._mint_internal(caller, token_id, initial_supply)?;
            }

            // Set URI if provided
            if let Some(uri_value) = uri.clone() {
                self.token_uris.insert(token_id, &uri_value);
            }

            self.env().emit_event(TokenCreated {
                token_id,
                token_type,
                initial_supply,
                max_supply: max_supply_cap,
                uri,
            });

            Ok(token_id)
        }

        /// Mint additional tokens of an existing type (owner only).
        /// Token must have been created via `create_token`. (GEM-05-C03)
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

            // Verify token exists (GEM-05-C03)
            if self.token_types.get(token_id).is_none() {
                return Err(Error::TokenNotFound);
            }

            self._mint(to, token_id, amount)
        }

        /// Batch mint multiple tokens (owner only, capped at MAX_BATCH_SIZE)
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
            if token_ids.len() > MAX_BATCH_SIZE {
                return Err(Error::BatchTooLarge);
            }

            // Verify all tokens exist before any mutation (GEM-05-C03)
            for token_id in token_ids.iter() {
                if self.token_types.get(*token_id).is_none() {
                    return Err(Error::TokenNotFound);
                }
            }

            if to == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            // Mint each token
            for (token_id, amount) in token_ids.iter().zip(amounts.iter()) {
                if *amount == 0 {
                    continue;
                }
                self._mint_internal(to, *token_id, *amount)?;
            }

            // Emit single TransferBatch event (GEM-05-L02)
            self.env().emit_event(TransferBatch {
                operator: Some(caller),
                from: None,
                to: Some(to),
                token_ids,
                values: amounts,
            });

            Ok(())
        }

        // ========================================================================
        // PSP37 Burnable Extension
        // ========================================================================

        /// Burn tokens (reduce supply) — self-burn only
        #[ink(message)]
        pub fn burn(&mut self, token_id: TokenId, amount: TokenBalance) -> Result<()> {
            let caller = self.env().caller();
            self._burn(caller, token_id, amount)
        }

        /// Burn tokens from another account.
        /// Requires BURN approval (set_burn_approval), NOT transfer approval. (GEM-05-H03)
        #[ink(message)]
        pub fn burn_from(
            &mut self,
            from: AccountId,
            token_id: TokenId,
            amount: TokenBalance,
        ) -> Result<()> {
            let caller = self.env().caller();

            // Burn requires explicit burn approval, separate from transfer approval (GEM-05-H03)
            if caller != from && !self.is_burn_approved(from, caller) {
                return Err(Error::BurnNotAuthorized);
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

        /// Get the pending owner (if a two-step transfer is in progress)
        #[ink(message)]
        pub fn pending_owner(&self) -> Option<AccountId> {
            self.pending_owner
        }

        /// Upgrades the contract code hash (owner only).
        /// Emits CodeHashUpdated event for off-chain monitoring. (GEM-05-M03)
        #[ink(message)]
        pub fn set_code_hash(&mut self, new_code_hash: Hash) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            self.env().emit_event(CodeHashUpdated {
                new_code_hash,
                caller,
            });

            ink::env::set_code_hash::<Environment>(&new_code_hash)
                .map_err(|_| Error::NotAuthorized)?;
            Ok(())
        }

        /// Propose ownership transfer (owner only). Two-step: new_owner must
        /// call `accept_ownership` to complete. (GEM-05-M02)
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            if new_owner == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            self.pending_owner = Some(new_owner);
            Ok(())
        }

        /// Accept a pending ownership transfer. Must be called by the pending owner. (GEM-05-M02)
        #[ink(message)]
        pub fn accept_ownership(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let pending = self.pending_owner.ok_or(Error::NotAuthorized)?;

            if caller != pending {
                return Err(Error::NotAuthorized);
            }

            let previous_owner = self.owner;
            self.owner = caller;
            self.pending_owner = None;

            // Emit ownership transferred event (GEM-05-M04)
            self.env().emit_event(OwnershipTransferred {
                previous_owner,
                new_owner: caller,
            });

            Ok(())
        }

        /// Cancel a pending ownership transfer (owner only)
        #[ink(message)]
        pub fn cancel_ownership_transfer(&mut self) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            self.pending_owner = None;
            Ok(())
        }

        /// Set token URI (owner only)
        #[ink(message)]
        pub fn set_token_uri(&mut self, token_id: TokenId, uri: String) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotAuthorized);
            }

            // Verify token exists
            if self.token_types.get(token_id).is_none() {
                return Err(Error::TokenNotFound);
            }

            self.token_uris.insert(token_id, &uri);
            Ok(())
        }

        // ========================================================================
        // Internal Functions
        // ========================================================================

        /// Internal transfer implementation with per-token allowance support (GEM-05-C02)
        fn _transfer_from(
            &mut self,
            operator: AccountId,
            from: AccountId,
            to: AccountId,
            token_id: TokenId,
            value: TokenBalance,
        ) -> Result<()> {
            // Skip zero-value transfers (GEM-05-L01)
            if value == 0 {
                return Ok(());
            }

            // Validate addresses
            if to == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            // Verify token exists
            let tt = self.token_types.get(token_id).ok_or(Error::TokenNotFound)?;

            // Enforce non-fungible transfer constraint (GEM-05-C01)
            if tt == TokenType::NonFungible && value != 1 {
                return Err(Error::NonFungibleValueInvalid);
            }

            // Check authorization: self, operator approval, or per-token allowance (GEM-05-C02)
            if operator != from {
                if self.is_approved_for_all(from, operator) {
                    // Blanket operator approval — no decrement needed
                } else {
                    // Check per-token allowance and decrement
                    let current_allowance = self.allowance(from, operator, token_id);
                    if current_allowance < value {
                        return Err(Error::InsufficientAllowance);
                    }
                    let new_allowance = current_allowance
                        .checked_sub(value)
                        .ok_or(Error::Overflow)?;
                    self.token_approvals
                        .insert((from, operator, token_id), &new_allowance);
                }
            }

            // Check balance
            let from_balance = self.balance_of(from, token_id);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            // Update balances with checked arithmetic (GEM-05-H02)
            let new_from_balance = from_balance.checked_sub(value).ok_or(Error::Overflow)?;
            self.balances.insert((from, token_id), &new_from_balance);

            let to_balance = self.balance_of(to, token_id);
            let new_to_balance = to_balance.checked_add(value).ok_or(Error::Overflow)?;
            self.balances.insert((to, token_id), &new_to_balance);

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

        /// Internal batch transfer implementation with per-token allowance support
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

            // Enforce batch size limit (GEM-05-H01)
            if token_ids.len() > MAX_BATCH_SIZE {
                return Err(Error::BatchTooLarge);
            }

            if to == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            // Determine authorization mode once
            let is_self = operator == from;
            let is_operator = !is_self && self.is_approved_for_all(from, operator);

            // Transfer each token
            for (token_id, value) in token_ids.iter().zip(values.iter()) {
                // Skip zero-value entries (GEM-05-L01)
                if *value == 0 {
                    continue;
                }

                // Verify token exists
                let tt = self
                    .token_types
                    .get(*token_id)
                    .ok_or(Error::TokenNotFound)?;

                // Enforce non-fungible transfer constraint (GEM-05-C01)
                if tt == TokenType::NonFungible && *value != 1 {
                    return Err(Error::NonFungibleValueInvalid);
                }

                // Check per-token allowance if not self and not blanket operator (GEM-05-C02)
                if !is_self && !is_operator {
                    let current_allowance = self.allowance(from, operator, *token_id);
                    if current_allowance < *value {
                        return Err(Error::InsufficientAllowance);
                    }
                    let new_allowance = current_allowance
                        .checked_sub(*value)
                        .ok_or(Error::Overflow)?;
                    self.token_approvals
                        .insert((from, operator, *token_id), &new_allowance);
                }

                let from_balance = self.balance_of(from, *token_id);
                if from_balance < *value {
                    return Err(Error::InsufficientBalance);
                }

                // Update balances with checked arithmetic (GEM-05-H02)
                let new_from_balance = from_balance.checked_sub(*value).ok_or(Error::Overflow)?;
                self.balances.insert((from, *token_id), &new_from_balance);

                let to_balance = self.balance_of(to, *token_id);
                let new_to_balance = to_balance.checked_add(*value).ok_or(Error::Overflow)?;
                self.balances.insert((to, *token_id), &new_to_balance);
            }

            // Emit single batch event
            self.env().emit_event(TransferBatch {
                operator: Some(operator),
                from: Some(from),
                to: Some(to),
                token_ids,
                values,
            });

            Ok(())
        }

        /// Internal mint — validates type constraints, supply cap, and uses checked arithmetic.
        /// Does NOT emit events — callers are responsible for event emission.
        fn _mint_internal(
            &mut self,
            to: AccountId,
            token_id: TokenId,
            amount: TokenBalance,
        ) -> Result<()> {
            if to == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            let tt = self.token_types.get(token_id).ok_or(Error::TokenNotFound)?;

            // Enforce non-fungible constraints (GEM-05-C01)
            if tt == TokenType::NonFungible {
                if amount != 1 {
                    return Err(Error::NonFungibleDuplicate);
                }
                let current_supply = self.total_supply(token_id);
                if current_supply >= 1 {
                    return Err(Error::NonFungibleDuplicate);
                }
            }

            // Compute new supply with checked arithmetic (GEM-05-H02)
            let supply = self.total_supply(token_id);
            let new_supply = supply.checked_add(amount).ok_or(Error::Overflow)?;

            // Enforce max supply cap (GEM-05-M01)
            if let Some(cap) = self.max_supply.get(token_id) {
                if new_supply > cap {
                    return Err(Error::SupplyCapExceeded);
                }
            }

            // Update balance with checked arithmetic (GEM-05-H02)
            let balance = self.balance_of(to, token_id);
            let new_balance = balance.checked_add(amount).ok_or(Error::Overflow)?;

            // Enforce non-fungible balance ∈ {0, 1} (GEM-05-C01)
            if tt == TokenType::NonFungible && new_balance > 1 {
                return Err(Error::NonFungibleDuplicate);
            }

            self.balances.insert((to, token_id), &new_balance);
            self.total_supply.insert(token_id, &new_supply);

            Ok(())
        }

        /// Public-facing mint helper — validates, mints, and emits TransferSingle event.
        fn _mint(&mut self, to: AccountId, token_id: TokenId, amount: TokenBalance) -> Result<()> {
            if amount == 0 {
                return Ok(());
            }

            self._mint_internal(to, token_id, amount)?;

            self.env().emit_event(TransferSingle {
                operator: Some(self.env().caller()),
                from: None,
                to: Some(to),
                token_id,
                value: amount,
            });

            Ok(())
        }

        /// Internal burn implementation with checked arithmetic (GEM-05-M05)
        fn _burn(
            &mut self,
            from: AccountId,
            token_id: TokenId,
            amount: TokenBalance,
        ) -> Result<()> {
            // Skip zero-value burns (GEM-05-L01)
            if amount == 0 {
                return Ok(());
            }

            // Verify token exists
            if self.token_types.get(token_id).is_none() {
                return Err(Error::TokenNotFound);
            }

            // Check balance
            let balance = self.balance_of(from, token_id);
            if balance < amount {
                return Err(Error::InsufficientBalance);
            }

            // Update balance with checked arithmetic (GEM-05-H02)
            let new_balance = balance.checked_sub(amount).ok_or(Error::Overflow)?;
            self.balances.insert((from, token_id), &new_balance);

            // Update total supply with checked arithmetic (GEM-05-M05)
            let supply = self.total_supply(token_id);
            let new_supply = supply.checked_sub(amount).ok_or(Error::Overflow)?;
            self.total_supply.insert(token_id, &new_supply);

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

        fn default_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        // ====================================================================
        // Constructor
        // ====================================================================

        #[ink::test]
        fn new_works() {
            let contract = Psp37MultiToken::new();
            assert_eq!(contract.next_token_id, 1);
            assert_eq!(contract.pending_owner, None);
        }

        // ====================================================================
        // Token Creation
        // ====================================================================

        #[ink::test]
        fn create_fungible_token_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_id = contract
                .create_token(
                    TokenType::Fungible,
                    1000,
                    Some(10_000),
                    Some("https://example.com/token/1".into()),
                )
                .unwrap();

            assert_eq!(token_id, 1);
            assert_eq!(contract.balance_of(accounts.alice, token_id), 1000);
            assert_eq!(contract.total_supply(token_id), 1000);
            assert_eq!(contract.token_type(token_id), Some(TokenType::Fungible));
            assert_eq!(contract.get_max_supply(token_id), Some(10_000));
            assert_eq!(
                contract.token_uri(token_id),
                Some("https://example.com/token/1".into())
            );
        }

        #[ink::test]
        fn create_nft_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_id = contract
                .create_token(TokenType::NonFungible, 1, None, None)
                .unwrap();

            assert_eq!(token_id, 1);
            assert_eq!(contract.balance_of(accounts.alice, token_id), 1);
            assert_eq!(contract.total_supply(token_id), 1);
            assert_eq!(contract.token_type(token_id), Some(TokenType::NonFungible));
            assert_eq!(contract.get_max_supply(token_id), Some(1));
        }

        #[ink::test]
        fn create_nft_rejects_supply_over_1() {
            let mut contract = Psp37MultiToken::new();
            assert_eq!(
                contract.create_token(TokenType::NonFungible, 2, None, None),
                Err(Error::NonFungibleDuplicate)
            );
        }

        // ====================================================================
        // C01 — NFT Uniqueness Enforcement
        // ====================================================================

        #[ink::test]
        fn nft_cannot_be_minted_twice() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let nft_id = contract
                .create_token(TokenType::NonFungible, 1, None, None)
                .unwrap();

            // Attempt to mint a second unit — must fail
            assert_eq!(
                contract.mint(accounts.bob, nft_id, 1),
                Err(Error::NonFungibleDuplicate)
            );
        }

        #[ink::test]
        fn nft_batch_mint_same_id_fails() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            // Create an NFT with no initial supply
            let nft_id = contract
                .create_token(TokenType::NonFungible, 0, None, None)
                .unwrap();

            // Batch mint same NFT ID twice — must fail on second
            assert_eq!(
                contract.batch_mint(accounts.alice, vec![nft_id, nft_id], vec![1, 1]),
                Err(Error::NonFungibleDuplicate)
            );
            // Supply must still be 0 (ink! reverts all on Err)
        }

        // ====================================================================
        // C02 — Per-Token Approvals
        // ====================================================================

        #[ink::test]
        fn per_token_approval_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_id = contract
                .create_token(TokenType::Fungible, 1000, None, None)
                .unwrap();

            // Approve Bob for 200 of token_id
            contract.approve(accounts.bob, token_id, 200).unwrap();
            assert_eq!(
                contract.allowance(accounts.alice, accounts.bob, token_id),
                200
            );

            // Bob transfers 100
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contract
                .transfer_from(accounts.alice, accounts.charlie, token_id, 100)
                .unwrap();

            // Allowance decremented
            assert_eq!(
                contract.allowance(accounts.alice, accounts.bob, token_id),
                100
            );
            assert_eq!(contract.balance_of(accounts.alice, token_id), 900);
            assert_eq!(contract.balance_of(accounts.charlie, token_id), 100);
        }

        #[ink::test]
        fn per_token_approval_insufficient_fails() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_id = contract
                .create_token(TokenType::Fungible, 1000, None, None)
                .unwrap();

            // Approve Bob for only 50
            contract.approve(accounts.bob, token_id, 50).unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                contract.transfer_from(accounts.alice, accounts.charlie, token_id, 100),
                Err(Error::InsufficientAllowance)
            );
        }

        // ====================================================================
        // C03 — Mint Requires Token Existence
        // ====================================================================

        #[ink::test]
        fn mint_nonexistent_token_fails() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            // Token 999 was never created
            assert_eq!(
                contract.mint(accounts.alice, 999, 100),
                Err(Error::TokenNotFound)
            );
        }

        // ====================================================================
        // H01 — Batch Size Limit
        // ====================================================================

        #[ink::test]
        fn batch_exceeds_max_size_fails() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_ids: Vec<TokenId> = (0..51).collect();
            let values: Vec<Balance> = vec![1; 51];

            assert_eq!(
                contract.batch_transfer(accounts.bob, token_ids, values),
                Err(Error::BatchTooLarge)
            );
        }

        // ====================================================================
        // H03 — Burn Approval Separation
        // ====================================================================

        #[ink::test]
        fn burn_from_requires_burn_approval() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_id = contract
                .create_token(TokenType::Fungible, 1000, None, None)
                .unwrap();

            // Approve Bob for transfers (NOT burns)
            contract.set_approval_for_all(accounts.bob, true).unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

            // Bob can transfer
            assert!(contract
                .transfer_from(accounts.alice, accounts.charlie, token_id, 100)
                .is_ok());

            // Bob CANNOT burn — only has transfer approval
            assert_eq!(
                contract.burn_from(accounts.alice, token_id, 100),
                Err(Error::BurnNotAuthorized)
            );
        }

        #[ink::test]
        fn burn_from_with_burn_approval_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_id = contract
                .create_token(TokenType::Fungible, 1000, None, None)
                .unwrap();

            // Grant Bob burn approval
            contract.set_burn_approval(accounts.bob, true).unwrap();
            assert!(contract.is_burn_approved(accounts.alice, accounts.bob));

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert!(contract.burn_from(accounts.alice, token_id, 100).is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token_id), 900);
            assert_eq!(contract.total_supply(token_id), 900);
        }

        // ====================================================================
        // M01 — Supply Cap
        // ====================================================================

        #[ink::test]
        fn supply_cap_enforced() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_id = contract
                .create_token(TokenType::Fungible, 500, Some(1000), None)
                .unwrap();

            // Mint up to cap
            assert!(contract.mint(accounts.alice, token_id, 500).is_ok());
            assert_eq!(contract.total_supply(token_id), 1000);

            // Exceeding cap fails
            assert_eq!(
                contract.mint(accounts.alice, token_id, 1),
                Err(Error::SupplyCapExceeded)
            );
        }

        // ====================================================================
        // M02 — Two-Step Ownership Transfer
        // ====================================================================

        #[ink::test]
        fn two_step_ownership_transfer_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            // Alice proposes Bob
            contract.transfer_ownership(accounts.bob).unwrap();
            assert_eq!(contract.pending_owner(), Some(accounts.bob));
            assert_eq!(contract.owner(), accounts.alice); // Still Alice

            // Bob accepts
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            contract.accept_ownership().unwrap();
            assert_eq!(contract.owner(), accounts.bob);
            assert_eq!(contract.pending_owner(), None);
        }

        #[ink::test]
        fn ownership_transfer_rejects_zero_address() {
            let mut contract = Psp37MultiToken::new();
            assert_eq!(
                contract.transfer_ownership(AccountId::from([0u8; 32])),
                Err(Error::ZeroAddress)
            );
        }

        #[ink::test]
        fn cancel_ownership_transfer_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            contract.transfer_ownership(accounts.bob).unwrap();
            contract.cancel_ownership_transfer().unwrap();
            assert_eq!(contract.pending_owner(), None);
        }

        // ====================================================================
        // Original Tests (updated for new API)
        // ====================================================================

        #[ink::test]
        fn transfer_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_id = contract
                .create_token(TokenType::Fungible, 1000, None, None)
                .unwrap();

            assert!(contract.transfer(accounts.bob, token_id, 100).is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token_id), 900);
            assert_eq!(contract.balance_of(accounts.bob, token_id), 100);
        }

        #[ink::test]
        fn transfer_fails_insufficient_balance() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_id = contract
                .create_token(TokenType::Fungible, 100, None, None)
                .unwrap();

            assert_eq!(
                contract.transfer(accounts.bob, token_id, 1000),
                Err(Error::InsufficientBalance)
            );
        }

        #[ink::test]
        fn approval_for_all_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            assert!(contract.set_approval_for_all(accounts.bob, true).is_ok());
            assert!(contract.is_approved_for_all(accounts.alice, accounts.bob));

            assert!(contract.set_approval_for_all(accounts.bob, false).is_ok());
            assert!(!contract.is_approved_for_all(accounts.alice, accounts.bob));
        }

        #[ink::test]
        fn transfer_from_with_operator_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_id = contract
                .create_token(TokenType::Fungible, 1000, None, None)
                .unwrap();

            contract.set_approval_for_all(accounts.bob, true).unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert!(contract
                .transfer_from(accounts.alice, accounts.charlie, token_id, 100)
                .is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token_id), 900);
            assert_eq!(contract.balance_of(accounts.charlie, token_id), 100);
        }

        #[ink::test]
        fn batch_transfer_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token1 = contract
                .create_token(TokenType::Fungible, 1000, None, None)
                .unwrap();
            let token2 = contract
                .create_token(TokenType::Fungible, 2000, None, None)
                .unwrap();

            let token_ids = vec![token1, token2];
            let values = vec![100, 200];

            assert!(contract
                .batch_transfer(accounts.bob, token_ids, values)
                .is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token1), 900);
            assert_eq!(contract.balance_of(accounts.alice, token2), 1800);
            assert_eq!(contract.balance_of(accounts.bob, token1), 100);
            assert_eq!(contract.balance_of(accounts.bob, token2), 200);
        }

        #[ink::test]
        fn batch_mint_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token1 = contract
                .create_token(TokenType::Fungible, 0, None, None)
                .unwrap();
            let token2 = contract
                .create_token(TokenType::Fungible, 0, None, None)
                .unwrap();

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
            let accounts = default_accounts();

            let token_id = contract
                .create_token(TokenType::Fungible, 1000, None, None)
                .unwrap();

            assert!(contract.burn(token_id, 100).is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token_id), 900);
            assert_eq!(contract.total_supply(token_id), 900);
        }

        #[ink::test]
        fn balance_of_batch_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token1 = contract
                .create_token(TokenType::Fungible, 1000, None, None)
                .unwrap();
            let token2 = contract
                .create_token(TokenType::Fungible, 2000, None, None)
                .unwrap();

            let owners = vec![accounts.alice, accounts.alice];
            let token_ids = vec![token1, token2];

            let balances = contract.balance_of_batch(owners, token_ids).unwrap();
            assert_eq!(balances, vec![1000, 2000]);
        }

        // ====================================================================
        // L01 — Zero-value operations are no-ops
        // ====================================================================

        #[ink::test]
        fn zero_value_transfer_is_noop() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let token_id = contract
                .create_token(TokenType::Fungible, 1000, None, None)
                .unwrap();

            assert!(contract.transfer(accounts.bob, token_id, 0).is_ok());
            assert_eq!(contract.balance_of(accounts.alice, token_id), 1000);
        }

        #[ink::test]
        fn zero_value_burn_is_noop() {
            let mut contract = Psp37MultiToken::new();

            let token_id = contract
                .create_token(TokenType::Fungible, 1000, None, None)
                .unwrap();

            assert!(contract.burn(token_id, 0).is_ok());
            assert_eq!(contract.total_supply(token_id), 1000);
        }

        // ====================================================================
        // NFT Transfer
        // ====================================================================

        #[ink::test]
        fn nft_transfer_works() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let nft_id = contract
                .create_token(TokenType::NonFungible, 1, None, None)
                .unwrap();

            assert!(contract.transfer(accounts.bob, nft_id, 1).is_ok());
            assert_eq!(contract.balance_of(accounts.alice, nft_id), 0);
            assert_eq!(contract.balance_of(accounts.bob, nft_id), 1);
        }

        #[ink::test]
        fn nft_transfer_rejects_value_not_1() {
            let mut contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let nft_id = contract
                .create_token(TokenType::NonFungible, 1, None, None)
                .unwrap();

            assert_eq!(
                contract.transfer(accounts.bob, nft_id, 2),
                Err(Error::NonFungibleValueInvalid)
            );
        }

        // ====================================================================
        // Balance-of-batch size limit
        // ====================================================================

        #[ink::test]
        fn balance_of_batch_exceeds_limit_fails() {
            let contract = Psp37MultiToken::new();
            let accounts = default_accounts();

            let owners = vec![accounts.alice; 51];
            let token_ids: Vec<TokenId> = (0..51).collect();

            assert_eq!(
                contract.balance_of_batch(owners, token_ids),
                Err(Error::BatchTooLarge)
            );
        }
    }
}

#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// # BeliNFT - PSP34 Compliant NFT Contract
///
/// The official NFT contract for BelizeChain.
/// Implements the PSP34 standard (Polkadot's ERC721 equivalent).
///
/// ## Features
/// - PSP34 standard compliance (mint, transfer, burn, approve)
/// - Metadata support (token URI, attributes)
/// - Collection management
/// - Approval system (per-token and operator)
///
/// ## Use Cases
/// - Digital art collections
/// - Land title NFTs (BelizeChain Land Ledger)
/// - Identity documents
/// - Gaming assets
/// - Membership tokens
///
/// ## Design Notes
/// - Uses a simple single-owner model rather than the `access_control` library.
///   This is intentional: a single `owner` role is sufficient for mint/admin gating.
///   Multi-role governance can integrate via the companion `access_control` contract.

#[ink::contract]
mod beli_nft {
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    /// Token ID type
    pub type TokenId = u32;

    /// Maximum length for token URI strings (bytes)
    const MAX_URI_LENGTH: usize = 2048;

    /// The BeliNFT error types
    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        /// Token does not exist
        TokenNotFound,
        /// Caller is not owner or approved
        NotAuthorized,
        /// Cannot transfer to zero address
        InvalidRecipient,
        /// Token already exists (double mint)
        TokenExists,
        /// Minting only allowed by contract owner
        NotOwner,
        /// Approval to current owner
        SelfApproval,
        /// URI exceeds maximum length
        UriTooLong,
        /// Arithmetic overflow
        Overflow,
        /// Maximum supply cap reached
        MaxSupplyReached,
    }

    /// Result type for BeliNFT operations
    pub type Result<T> = core::result::Result<T, Error>;

    /// PSP34-standard token ID type
    #[derive(Debug, PartialEq, Eq, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[allow(clippy::cast_possible_truncation)]
    pub enum Id {
        U8(u8),
        U16(u16),
        U32(u32),
        U64(u64),
        U128(u128),
        Bytes(Vec<u8>),
    }

    /// PSP34-standard error type
    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[allow(clippy::cast_possible_truncation)]
    pub enum PSP34Error {
        /// Custom error with message
        Custom(String),
        /// Approval to self
        SelfApprove,
        /// Caller is not approved
        NotApproved,
        /// Token already exists
        TokenExists,
        /// Token does not exist
        TokenNotExists,
        /// Safe transfer check failed
        SafeTransferCheckFailed(String),
    }

    impl From<Error> for PSP34Error {
        fn from(err: Error) -> Self {
            match err {
                Error::TokenNotFound => PSP34Error::TokenNotExists,
                Error::NotAuthorized => PSP34Error::NotApproved,
                Error::InvalidRecipient => PSP34Error::Custom(String::from("InvalidRecipient")),
                Error::TokenExists => PSP34Error::TokenExists,
                Error::NotOwner => PSP34Error::Custom(String::from("NotOwner")),
                Error::SelfApproval => PSP34Error::SelfApprove,
                Error::UriTooLong => PSP34Error::Custom(String::from("UriTooLong")),
                Error::Overflow => PSP34Error::Custom(String::from("Overflow")),
                Error::MaxSupplyReached => PSP34Error::Custom(String::from("MaxSupplyReached")),
            }
        }
    }

    /// The BeliNFT storage
    #[ink(storage)]
    pub struct BeliNft {
        /// Mapping from token ID to owner
        token_owner: Mapping<TokenId, AccountId>,
        /// Mapping from owner to token count
        owned_tokens_count: Mapping<AccountId, u32>,
        /// Mapping from token ID to approved address
        token_approvals: Mapping<TokenId, AccountId>,
        /// Mapping from owner to operator approvals
        operator_approvals: Mapping<(AccountId, AccountId), ()>,
        /// Mapping from token ID to metadata URI
        token_uri: Mapping<TokenId, String>,
        /// Mapping of burned token IDs (prevents ID reuse)
        burned_ids: Mapping<TokenId, ()>,
        /// Total supply of tokens
        total_supply: u32,
        /// Next token ID to mint
        next_token_id: TokenId,
        /// Contract owner (can mint)
        owner: AccountId,
        /// Collection name
        name: String,
        /// Collection symbol
        symbol: String,
        /// Pending owner for two-step ownership transfer
        pending_owner: Option<AccountId>,
        /// Optional maximum supply cap
        max_supply: Option<u32>,
    }

    /// Event emitted when a token is transferred
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: TokenId,
    }

    /// Event emitted when a token is approved
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        approved: AccountId,
        #[ink(topic)]
        id: TokenId,
    }

    /// Event emitted when an operator is approved
    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        approved: bool,
    }

    /// Event emitted when token metadata is updated
    #[ink(event)]
    pub struct MetadataUpdate {
        #[ink(topic)]
        id: TokenId,
    }

    /// Event emitted when contract ownership is transferred
    #[ink(event)]
    pub struct OwnershipTransferred {
        #[ink(topic)]
        old_owner: AccountId,
        #[ink(topic)]
        new_owner: AccountId,
    }

    /// Event emitted when the contract code hash is updated
    #[ink(event)]
    pub struct CodeHashUpdated {
        #[ink(topic)]
        new_code_hash: Hash,
    }

    impl BeliNft {
        /// Creates a new BeliNFT collection
        #[ink(constructor)]
        pub fn new(name: String, symbol: String) -> Self {
            let caller = Self::env().caller();

            Self {
                token_owner: Mapping::default(),
                owned_tokens_count: Mapping::default(),
                token_approvals: Mapping::default(),
                operator_approvals: Mapping::default(),
                token_uri: Mapping::default(),
                burned_ids: Mapping::default(),
                total_supply: 0,
                next_token_id: 1,
                owner: caller,
                name,
                symbol,
                pending_owner: None,
                max_supply: None,
            }
        }

        /// Returns the collection name
        #[ink(message)]
        pub fn collection_name(&self) -> String {
            self.name.clone()
        }

        /// Returns the collection symbol
        #[ink(message)]
        pub fn collection_symbol(&self) -> String {
            self.symbol.clone()
        }

        /// Returns a unique identifier for this NFT collection (PSP34 standard)
        #[ink(message)]
        pub fn collection_id(&self) -> Id {
            Id::Bytes(self.name.as_bytes().to_vec())
        }

        /// Returns the total supply of tokens
        #[ink(message)]
        pub fn total_supply(&self) -> u32 {
            self.total_supply
        }

        /// Returns the owner of a token (PSP34 standard)
        #[ink(message)]
        pub fn owner_of(&self, id: Id) -> Option<AccountId> {
            let token_id = Self::extract_id(&id).ok()?;
            self.token_owner.get(token_id)
        }

        /// Returns the balance of an account
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u32 {
            self.owned_tokens_count.get(owner).unwrap_or(0)
        }

        /// Returns the approved address for a token
        #[ink(message)]
        pub fn get_approved(&self, id: TokenId) -> Option<AccountId> {
            self.token_approvals.get(id)
        }

        /// Returns whether an operator is approved for all tokens of an owner
        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.operator_approvals.contains((owner, operator))
        }

        /// Checks approval status (PSP34 standard)
        ///
        /// - `id: Some(Id)` — checks per-token approval
        /// - `id: None` — checks operator-level approval
        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, operator: AccountId, id: Option<Id>) -> bool {
            match id {
                Some(token_id) => {
                    let tid = match Self::extract_id(&token_id) {
                        Ok(v) => v,
                        Err(_) => return false,
                    };
                    self.token_approvals.get(tid) == Some(operator)
                }
                None => self.operator_approvals.contains((owner, operator)),
            }
        }

        /// Returns the metadata URI for a token
        #[ink(message)]
        pub fn token_uri(&self, id: TokenId) -> Option<String> {
            self.token_uri.get(id)
        }

        /// Returns a token attribute by key (PSP34Metadata standard)
        ///
        /// Supported keys: `b"uri"` (returns the token URI)
        #[ink(message)]
        pub fn get_attribute(&self, id: Id, key: Vec<u8>) -> Option<Vec<u8>> {
            let token_id = Self::extract_id(&id).ok()?;
            if !self.token_owner.contains(token_id) {
                return None;
            }
            if key == b"uri" {
                self.token_uri.get(token_id).map(|s| s.into_bytes())
            } else {
                None
            }
        }

        /// Mints a new token (owner only)
        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, uri: String) -> Result<TokenId> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }

            if let Some(cap) = self.max_supply {
                if self.total_supply >= cap {
                    return Err(Error::MaxSupplyReached);
                }
            }

            let token_id = self.next_token_id;
            self.mint_token(to, token_id, uri)?;
            self.next_token_id = self.next_token_id.checked_add(1).ok_or(Error::Overflow)?;

            Ok(token_id)
        }

        /// Transfers a token (PSP34 standard)
        #[ink(message)]
        pub fn transfer(
            &mut self,
            to: AccountId,
            id: Id,
            _data: Vec<u8>,
        ) -> core::result::Result<(), PSP34Error> {
            let token_id = Self::extract_id(&id)?;
            let caller = self.env().caller();
            let owner = self
                .token_owner
                .get(token_id)
                .ok_or(PSP34Error::TokenNotExists)?;

            if caller != owner && !self.is_approved_or_owner(caller, token_id) {
                return Err(PSP34Error::NotApproved);
            }

            self.transfer_token_from(owner, to, token_id)?;
            Ok(())
        }

        /// Transfers a token from one account to another
        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, id: TokenId) -> Result<()> {
            let caller = self.env().caller();
            let owner = self.token_owner.get(id).ok_or(Error::TokenNotFound)?;

            if owner != from {
                return Err(Error::NotAuthorized);
            }

            if !self.is_approved_or_owner(caller, id) {
                return Err(Error::NotAuthorized);
            }

            self.transfer_token_from(from, to, id)?;
            Ok(())
        }

        /// Approves an operator for a specific token or all tokens (PSP34 standard)
        ///
        /// - `id: Some(Id)` — per-token approval (grant or revoke)
        /// - `id: None` — operator-level approval for all tokens
        ///
        /// **Note:** Per-token approvals and operator-level approvals are independent
        /// systems per the PSP34 specification. Revoking an operator does not clear
        /// existing per-token approvals, and vice versa.
        #[ink(message)]
        pub fn approve(
            &mut self,
            operator: AccountId,
            id: Option<Id>,
            approved: bool,
        ) -> core::result::Result<(), PSP34Error> {
            let caller = self.env().caller();
            if caller == operator {
                return Err(PSP34Error::SelfApprove);
            }
            match id {
                Some(token_id) => {
                    let tid = Self::extract_id(&token_id)?;
                    let owner = self
                        .token_owner
                        .get(tid)
                        .ok_or(PSP34Error::TokenNotExists)?;
                    if caller != owner && !self.is_approved_for_all(owner, caller) {
                        return Err(PSP34Error::NotApproved);
                    }
                    if approved {
                        if operator == owner {
                            return Err(PSP34Error::SelfApprove);
                        }
                        self.token_approvals.insert(tid, &operator);
                    } else {
                        self.token_approvals.remove(tid);
                    }
                    self.env().emit_event(Approval {
                        owner,
                        approved: operator,
                        id: tid,
                    });
                }
                None => {
                    if approved {
                        self.operator_approvals.insert((caller, operator), &());
                    } else {
                        self.operator_approvals.remove((caller, operator));
                    }
                    self.env().emit_event(ApprovalForAll {
                        owner: caller,
                        operator,
                        approved,
                    });
                }
            }
            Ok(())
        }

        /// Sets or unsets the approval of an operator (convenience wrapper)
        #[ink(message)]
        pub fn set_approval_for_all(
            &mut self,
            operator: AccountId,
            approved: bool,
        ) -> core::result::Result<(), PSP34Error> {
            self.approve(operator, None, approved)
        }

        /// Burns a token (owner only)
        #[ink(message)]
        pub fn burn(&mut self, id: TokenId) -> Result<()> {
            let caller = self.env().caller();
            let owner = self.token_owner.get(id).ok_or(Error::TokenNotFound)?;

            if caller != owner {
                return Err(Error::NotAuthorized);
            }

            self.burn_token(id)?;
            Ok(())
        }

        /// Updates the URI of a token (owner only)
        #[ink(message)]
        pub fn set_token_uri(&mut self, id: TokenId, uri: String) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }

            if !self.token_owner.contains(id) {
                return Err(Error::TokenNotFound);
            }
            if uri.len() > MAX_URI_LENGTH {
                return Err(Error::UriTooLong);
            }

            self.token_uri.insert(id, &uri);
            self.env().emit_event(MetadataUpdate { id });
            Ok(())
        }

        /// Proposes a new owner for the contract (two-step transfer)
        ///
        /// The proposed owner must call `accept_ownership()` to complete the transfer.
        #[ink(message)]
        pub fn propose_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }
            if new_owner == AccountId::from([0u8; 32]) {
                return Err(Error::InvalidRecipient);
            }

            self.pending_owner = Some(new_owner);
            Ok(())
        }

        /// Accepts a pending ownership transfer (must be called by the proposed owner)
        #[ink(message)]
        pub fn accept_ownership(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let pending = self.pending_owner.ok_or(Error::NotAuthorized)?;
            if caller != pending {
                return Err(Error::NotAuthorized);
            }

            let old_owner = self.owner;
            self.owner = caller;
            self.pending_owner = None;

            self.env().emit_event(OwnershipTransferred {
                old_owner,
                new_owner: caller,
            });

            Ok(())
        }

        /// Returns the contract owner
        #[ink(message)]
        pub fn contract_owner(&self) -> AccountId {
            self.owner
        }

        /// Returns the pending owner (if any)
        #[ink(message)]
        pub fn pending_owner(&self) -> Option<AccountId> {
            self.pending_owner
        }

        /// Returns the maximum supply cap (None = unlimited)
        #[ink(message)]
        pub fn max_supply(&self) -> Option<u32> {
            self.max_supply
        }

        /// Sets the maximum supply cap (owner only, None = unlimited)
        #[ink(message)]
        pub fn set_max_supply(&mut self, max: Option<u32>) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }
            self.max_supply = max;
            Ok(())
        }

        /// Upgrades the contract code hash (owner only)
        ///
        /// Allows the contract to be upgraded to a new implementation
        /// while preserving storage state.
        #[ink(message)]
        pub fn set_code_hash(&mut self, new_code_hash: Hash) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }

            ink::env::set_code_hash::<Environment>(&new_code_hash).map_err(|_| Error::NotOwner)?;

            self.env().emit_event(CodeHashUpdated { new_code_hash });

            Ok(())
        }

        // ========== Internal Functions ==========

        /// Internal mint function
        fn mint_token(&mut self, to: AccountId, id: TokenId, uri: String) -> Result<()> {
            if to == AccountId::from([0u8; 32]) {
                return Err(Error::InvalidRecipient);
            }
            if self.token_owner.contains(id) {
                return Err(Error::TokenExists);
            }
            if self.burned_ids.contains(id) {
                return Err(Error::TokenExists);
            }
            if uri.len() > MAX_URI_LENGTH {
                return Err(Error::UriTooLong);
            }

            let count = self.balance_of(to);
            self.owned_tokens_count
                .insert(to, &(count.checked_add(1).ok_or(Error::Overflow)?));
            self.token_owner.insert(id, &to);
            self.token_uri.insert(id, &uri);
            self.total_supply = self.total_supply.checked_add(1).ok_or(Error::Overflow)?;

            self.env().emit_event(Transfer {
                from: None,
                to: Some(to),
                id,
            });

            Ok(())
        }

        /// Internal transfer function
        fn transfer_token_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: TokenId,
        ) -> Result<()> {
            if to == AccountId::from([0u8; 32]) {
                return Err(Error::InvalidRecipient);
            }

            // Clear approvals
            self.token_approvals.remove(id);

            // Update balances
            let from_count = self.balance_of(from);
            self.owned_tokens_count
                .insert(from, &(from_count.checked_sub(1).ok_or(Error::Overflow)?));

            let to_count = self.balance_of(to);
            self.owned_tokens_count
                .insert(to, &(to_count.checked_add(1).ok_or(Error::Overflow)?));

            // Update ownership
            self.token_owner.insert(id, &to);

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                id,
            });

            Ok(())
        }

        /// Internal burn function
        fn burn_token(&mut self, id: TokenId) -> Result<()> {
            let owner = self.token_owner.get(id).ok_or(Error::TokenNotFound)?;

            // Clear approvals
            self.token_approvals.remove(id);

            // Update balance
            let count = self.balance_of(owner);
            self.owned_tokens_count
                .insert(owner, &(count.checked_sub(1).ok_or(Error::Overflow)?));

            // Remove token
            self.token_owner.remove(id);
            self.token_uri.remove(id);
            self.burned_ids.insert(id, &());
            self.total_supply = self.total_supply.checked_sub(1).ok_or(Error::Overflow)?;

            self.env().emit_event(Transfer {
                from: Some(owner),
                to: None,
                id,
            });

            Ok(())
        }

        /// Checks if an address is approved or owner
        fn is_approved_or_owner(&self, spender: AccountId, id: TokenId) -> bool {
            let owner = match self.token_owner.get(id) {
                Some(o) => o,
                None => return false,
            };

            spender == owner
                || self.get_approved(id) == Some(spender)
                || self.is_approved_for_all(owner, spender)
        }

        /// Extracts u32 from a PSP34 Id (this collection uses U32 token IDs)
        fn extract_id(id: &Id) -> core::result::Result<TokenId, PSP34Error> {
            match id {
                Id::U32(v) => Ok(*v),
                _ => Err(PSP34Error::Custom(String::from(
                    "Only U32 token IDs supported",
                ))),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn default_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn set_caller(account: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(account);
        }

        #[ink::test]
        fn new_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let nft = BeliNft::new(String::from("Belize NFT Collection"), String::from("BNFT"));

            assert_eq!(nft.collection_name(), String::from("Belize NFT Collection"));
            assert_eq!(nft.collection_symbol(), String::from("BNFT"));
            assert_eq!(nft.total_supply(), 0);
            assert_eq!(nft.contract_owner(), accounts.alice);
        }

        #[ink::test]
        fn mint_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut nft = BeliNft::new(String::from("Belize NFT"), String::from("BNFT"));

            let uri = String::from("ipfs://QmTestHash");
            let result = nft.mint(accounts.bob, uri.clone());
            assert!(result.is_ok());

            let token_id = result.unwrap();
            assert_eq!(token_id, 1);
            assert_eq!(nft.owner_of(Id::U32(token_id)), Some(accounts.bob));
            assert_eq!(nft.balance_of(accounts.bob), 1);
            assert_eq!(nft.total_supply(), 1);
            assert_eq!(nft.token_uri(token_id), Some(uri));
        }

        #[ink::test]
        fn mint_fails_not_owner() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut nft = BeliNft::new(String::from("Belize NFT"), String::from("BNFT"));

            // Try to mint as non-owner
            set_caller(accounts.bob);
            let uri = String::from("ipfs://QmTestHash");
            let result = nft.mint(accounts.charlie, uri);
            assert_eq!(result, Err(Error::NotOwner));
        }

        #[ink::test]
        fn transfer_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut nft = BeliNft::new(String::from("Belize NFT"), String::from("BNFT"));

            // Mint token to Bob
            let uri = String::from("ipfs://QmTestHash");
            let token_id = nft.mint(accounts.bob, uri).unwrap();

            // Transfer from Bob to Charlie
            set_caller(accounts.bob);
            let result = nft.transfer(accounts.charlie, Id::U32(token_id), Vec::new());
            assert!(result.is_ok());

            assert_eq!(nft.owner_of(Id::U32(token_id)), Some(accounts.charlie));
            assert_eq!(nft.balance_of(accounts.bob), 0);
            assert_eq!(nft.balance_of(accounts.charlie), 1);
        }

        #[ink::test]
        fn transfer_fails_not_owner() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut nft = BeliNft::new(String::from("Belize NFT"), String::from("BNFT"));

            let uri = String::from("ipfs://QmTestHash");
            let token_id = nft.mint(accounts.bob, uri).unwrap();

            // Try to transfer as non-owner without approval
            set_caller(accounts.charlie);
            let result = nft.transfer(accounts.charlie, Id::U32(token_id), Vec::new());
            assert_eq!(result, Err(PSP34Error::NotApproved));
        }

        #[ink::test]
        fn approve_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut nft = BeliNft::new(String::from("Belize NFT"), String::from("BNFT"));

            let uri = String::from("ipfs://QmTestHash");
            let token_id = nft.mint(accounts.bob, uri).unwrap();

            // Approve Charlie to transfer Bob's token
            set_caller(accounts.bob);
            let result = nft.approve(accounts.charlie, Some(Id::U32(token_id)), true);
            assert!(result.is_ok());

            assert_eq!(nft.get_approved(token_id), Some(accounts.charlie));

            // Charlie can now transfer
            set_caller(accounts.charlie);
            let result = nft.transfer(accounts.charlie, Id::U32(token_id), Vec::new());
            assert!(result.is_ok());
            assert_eq!(nft.owner_of(Id::U32(token_id)), Some(accounts.charlie));
        }

        #[ink::test]
        fn approval_for_all_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut nft = BeliNft::new(String::from("Belize NFT"), String::from("BNFT"));

            // Mint two tokens to Bob
            let uri1 = String::from("ipfs://QmHash1");
            let uri2 = String::from("ipfs://QmHash2");
            let token_id_1 = nft.mint(accounts.bob, uri1).unwrap();
            let token_id_2 = nft.mint(accounts.bob, uri2).unwrap();

            // Bob approves Charlie as operator
            set_caller(accounts.bob);
            let result = nft.set_approval_for_all(accounts.charlie, true);
            assert!(result.is_ok());
            assert!(nft.is_approved_for_all(accounts.bob, accounts.charlie));

            // Charlie can transfer both tokens
            set_caller(accounts.charlie);
            assert!(nft
                .transfer(accounts.charlie, Id::U32(token_id_1), Vec::new())
                .is_ok());
            assert!(nft
                .transfer(accounts.charlie, Id::U32(token_id_2), Vec::new())
                .is_ok());
            assert_eq!(nft.balance_of(accounts.charlie), 2);
        }

        #[ink::test]
        fn burn_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut nft = BeliNft::new(String::from("Belize NFT"), String::from("BNFT"));

            let uri = String::from("ipfs://QmTestHash");
            let token_id = nft.mint(accounts.bob, uri).unwrap();
            assert_eq!(nft.total_supply(), 1);

            // Bob burns his token
            set_caller(accounts.bob);
            let result = nft.burn(token_id);
            assert!(result.is_ok());

            assert_eq!(nft.owner_of(Id::U32(token_id)), None);
            assert_eq!(nft.balance_of(accounts.bob), 0);
            assert_eq!(nft.total_supply(), 0);
        }

        #[ink::test]
        fn transfer_ownership_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut nft = BeliNft::new(String::from("Belize NFT"), String::from("BNFT"));

            assert_eq!(nft.contract_owner(), accounts.alice);

            // Step 1: Propose new owner
            let result = nft.propose_ownership(accounts.bob);
            assert!(result.is_ok());
            assert_eq!(nft.contract_owner(), accounts.alice); // Still Alice
            assert_eq!(nft.pending_owner(), Some(accounts.bob));

            // Step 2: Bob accepts ownership
            set_caller(accounts.bob);
            let result = nft.accept_ownership();
            assert!(result.is_ok());
            assert_eq!(nft.contract_owner(), accounts.bob);
            assert_eq!(nft.pending_owner(), None);

            // Alice can no longer mint
            let uri = String::from("ipfs://QmTestHash");
            set_caller(accounts.alice);
            let result = nft.mint(accounts.charlie, uri.clone());
            assert_eq!(result, Err(Error::NotOwner));

            // Bob can mint
            set_caller(accounts.bob);
            let result = nft.mint(accounts.charlie, uri);
            assert!(result.is_ok());
        }

        #[ink::test]
        fn max_supply_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut nft = BeliNft::new(String::from("Belize NFT"), String::from("BNFT"));

            // Set max supply to 2
            assert!(nft.set_max_supply(Some(2)).is_ok());
            assert_eq!(nft.max_supply(), Some(2));

            // Mint up to cap
            assert!(nft.mint(accounts.bob, String::from("ipfs://1")).is_ok());
            assert!(nft.mint(accounts.bob, String::from("ipfs://2")).is_ok());

            // Third mint should fail
            let result = nft.mint(accounts.bob, String::from("ipfs://3"));
            assert_eq!(result, Err(Error::MaxSupplyReached));

            // Remove cap
            assert!(nft.set_max_supply(None).is_ok());
            assert!(nft.mint(accounts.bob, String::from("ipfs://3")).is_ok());
        }

        #[ink::test]
        fn multiple_mints_increment_ids() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut nft = BeliNft::new(String::from("Belize NFT"), String::from("BNFT"));

            let uri1 = String::from("ipfs://QmHash1");
            let uri2 = String::from("ipfs://QmHash2");
            let uri3 = String::from("ipfs://QmHash3");

            let token_1 = nft.mint(accounts.bob, uri1).unwrap();
            let token_2 = nft.mint(accounts.bob, uri2).unwrap();
            let token_3 = nft.mint(accounts.bob, uri3).unwrap();

            assert_eq!(token_1, 1);
            assert_eq!(token_2, 2);
            assert_eq!(token_3, 3);
            assert_eq!(nft.balance_of(accounts.bob), 3);
            assert_eq!(nft.total_supply(), 3);
        }
    }
}

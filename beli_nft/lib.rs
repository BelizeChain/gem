#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// # BeliNFT - PSP34 Compliant NFT Contract
/// 
/// The official NFT contract for BelizeChain.
/// Implements the PSP34 standard (Polkadot's ERC721 equivalent).
/// 
/// ## Features
/// - PSP34 standard compliance (mint, transfer, burn)
/// - Metadata support (token URI)
/// - Collection management
/// - Enumeration support
/// - Approval system
/// 
/// ## Use Cases
/// - Digital art collections
/// - Land title NFTs (BelizeChain Land Ledger)
/// - Identity documents
/// - Gaming assets
/// - Membership tokens

#[ink::contract]
mod beli_nft {
    use ink::prelude::string::String;
    use ink::storage::Mapping;

    /// Token ID type
    pub type TokenId = u32;

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
    }

    /// Result type for BeliNFT operations
    pub type Result<T> = core::result::Result<T, Error>;

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
                total_supply: 0,
                next_token_id: 1,
                owner: caller,
                name,
                symbol,
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

        /// Returns the total supply of tokens
        #[ink(message)]
        pub fn total_supply(&self) -> u32 {
            self.total_supply
        }

        /// Returns the owner of a token
        #[ink(message)]
        pub fn owner_of(&self, id: TokenId) -> Option<AccountId> {
            self.token_owner.get(id)
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

        /// Returns the metadata URI for a token
        #[ink(message)]
        pub fn token_uri(&self, id: TokenId) -> Option<String> {
            self.token_uri.get(id)
        }

        /// Mints a new token (owner only)
        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, uri: String) -> Result<TokenId> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }

            let token_id = self.next_token_id;
            self.mint_token(to, token_id, uri)?;
            self.next_token_id = self.next_token_id.saturating_add(1);

            Ok(token_id)
        }

        /// Transfers a token
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, id: TokenId) -> Result<()> {
            let caller = self.env().caller();
            let owner = self.owner_of(id).ok_or(Error::TokenNotFound)?;

            if caller != owner && !self.is_approved_or_owner(caller, id) {
                return Err(Error::NotAuthorized);
            }

            self.transfer_token_from(owner, to, id)?;
            Ok(())
        }

        /// Transfers a token from one account to another
        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, id: TokenId) -> Result<()> {
            let caller = self.env().caller();
            let owner = self.owner_of(id).ok_or(Error::TokenNotFound)?;

            if owner != from {
                return Err(Error::NotAuthorized);
            }

            if caller != owner && !self.is_approved_or_owner(caller, id) {
                return Err(Error::NotAuthorized);
            }

            self.transfer_token_from(from, to, id)?;
            Ok(())
        }

        /// Approves an address to transfer a specific token
        #[ink(message)]
        pub fn approve(&mut self, to: AccountId, id: TokenId) -> Result<()> {
            let caller = self.env().caller();
            let owner = self.owner_of(id).ok_or(Error::TokenNotFound)?;

            if caller != owner && !self.is_approved_for_all(owner, caller) {
                return Err(Error::NotAuthorized);
            }

            if to == owner {
                return Err(Error::SelfApproval);
            }

            self.token_approvals.insert(id, &to);

            self.env().emit_event(Approval {
                owner,
                approved: to,
                id,
            });

            Ok(())
        }

        /// Sets or unsets the approval of an operator
        #[ink(message)]
        pub fn set_approval_for_all(&mut self, operator: AccountId, approved: bool) -> Result<()> {
            let caller = self.env().caller();

            if caller == operator {
                return Err(Error::SelfApproval);
            }

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

            Ok(())
        }

        /// Burns a token
        #[ink(message)]
        pub fn burn(&mut self, id: TokenId) -> Result<()> {
            let caller = self.env().caller();
            let owner = self.owner_of(id).ok_or(Error::TokenNotFound)?;

            if caller != owner && !self.is_approved_or_owner(caller, id) {
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

            if self.owner_of(id).is_none() {
                return Err(Error::TokenNotFound);
            }

            self.token_uri.insert(id, &uri);
            Ok(())
        }

        /// Transfers ownership of the contract
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::NotOwner);
            }

            self.owner = new_owner;
            Ok(())
        }

        /// Returns the contract owner
        #[ink(message)]
        pub fn contract_owner(&self) -> AccountId {
            self.owner
        }

        // ========== Internal Functions ==========

        /// Internal mint function
        fn mint_token(&mut self, to: AccountId, id: TokenId, uri: String) -> Result<()> {
            if self.token_owner.contains(id) {
                return Err(Error::TokenExists);
            }

            let count = self.balance_of(to);
            self.owned_tokens_count.insert(to, &(count.saturating_add(1)));
            self.token_owner.insert(id, &to);
            self.token_uri.insert(id, &uri);
            self.total_supply = self.total_supply.saturating_add(1);

            self.env().emit_event(Transfer {
                from: None,
                to: Some(to),
                id,
            });

            Ok(())
        }

        /// Internal transfer function
        fn transfer_token_from(&mut self, from: AccountId, to: AccountId, id: TokenId) -> Result<()> {
            // Clear approvals
            self.token_approvals.remove(id);

            // Update balances
            let from_count = self.balance_of(from);
            self.owned_tokens_count.insert(from, &(from_count.saturating_sub(1)));

            let to_count = self.balance_of(to);
            self.owned_tokens_count.insert(to, &(to_count.saturating_add(1)));

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
            let owner = self.owner_of(id).ok_or(Error::TokenNotFound)?;

            // Clear approvals
            self.token_approvals.remove(id);

            // Update balance
            let count = self.balance_of(owner);
            self.owned_tokens_count.insert(owner, &(count.saturating_sub(1)));

            // Remove token
            self.token_owner.remove(id);
            self.token_uri.remove(id);
            self.total_supply = self.total_supply.saturating_sub(1);

            self.env().emit_event(Transfer {
                from: Some(owner),
                to: None,
                id,
            });

            Ok(())
        }

        /// Checks if an address is approved or owner
        fn is_approved_or_owner(&self, spender: AccountId, id: TokenId) -> bool {
            let owner = match self.owner_of(id) {
                Some(o) => o,
                None => return false,
            };

            spender == owner
                || self.get_approved(id) == Some(spender)
                || self.is_approved_for_all(owner, spender)
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
            
            let nft = BeliNft::new(
                String::from("Belize NFT Collection"),
                String::from("BNFT"),
            );

            assert_eq!(nft.collection_name(), String::from("Belize NFT Collection"));
            assert_eq!(nft.collection_symbol(), String::from("BNFT"));
            assert_eq!(nft.total_supply(), 0);
            assert_eq!(nft.contract_owner(), accounts.alice);
        }

        #[ink::test]
        fn mint_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut nft = BeliNft::new(
                String::from("Belize NFT"),
                String::from("BNFT"),
            );

            let uri = String::from("ipfs://QmTestHash");
            let result = nft.mint(accounts.bob, uri.clone());
            assert!(result.is_ok());

            let token_id = result.unwrap();
            assert_eq!(token_id, 1);
            assert_eq!(nft.owner_of(token_id), Some(accounts.bob));
            assert_eq!(nft.balance_of(accounts.bob), 1);
            assert_eq!(nft.total_supply(), 1);
            assert_eq!(nft.token_uri(token_id), Some(uri));
        }

        #[ink::test]
        fn mint_fails_not_owner() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut nft = BeliNft::new(
                String::from("Belize NFT"),
                String::from("BNFT"),
            );

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
            
            let mut nft = BeliNft::new(
                String::from("Belize NFT"),
                String::from("BNFT"),
            );

            // Mint token to Bob
            let uri = String::from("ipfs://QmTestHash");
            let token_id = nft.mint(accounts.bob, uri).unwrap();

            // Transfer from Bob to Charlie
            set_caller(accounts.bob);
            let result = nft.transfer(accounts.charlie, token_id);
            assert!(result.is_ok());

            assert_eq!(nft.owner_of(token_id), Some(accounts.charlie));
            assert_eq!(nft.balance_of(accounts.bob), 0);
            assert_eq!(nft.balance_of(accounts.charlie), 1);
        }

        #[ink::test]
        fn transfer_fails_not_owner() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut nft = BeliNft::new(
                String::from("Belize NFT"),
                String::from("BNFT"),
            );

            let uri = String::from("ipfs://QmTestHash");
            let token_id = nft.mint(accounts.bob, uri).unwrap();

            // Try to transfer as non-owner without approval
            set_caller(accounts.charlie);
            let result = nft.transfer(accounts.charlie, token_id);
            assert_eq!(result, Err(Error::NotAuthorized));
        }

        #[ink::test]
        fn approve_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut nft = BeliNft::new(
                String::from("Belize NFT"),
                String::from("BNFT"),
            );

            let uri = String::from("ipfs://QmTestHash");
            let token_id = nft.mint(accounts.bob, uri).unwrap();

            // Approve Charlie to transfer Bob's token
            set_caller(accounts.bob);
            let result = nft.approve(accounts.charlie, token_id);
            assert!(result.is_ok());

            assert_eq!(nft.get_approved(token_id), Some(accounts.charlie));

            // Charlie can now transfer
            set_caller(accounts.charlie);
            let result = nft.transfer(accounts.charlie, token_id);
            assert!(result.is_ok());
            assert_eq!(nft.owner_of(token_id), Some(accounts.charlie));
        }

        #[ink::test]
        fn approval_for_all_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut nft = BeliNft::new(
                String::from("Belize NFT"),
                String::from("BNFT"),
            );

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
            assert!(nft.transfer(accounts.charlie, token_id_1).is_ok());
            assert!(nft.transfer(accounts.charlie, token_id_2).is_ok());
            assert_eq!(nft.balance_of(accounts.charlie), 2);
        }

        #[ink::test]
        fn burn_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut nft = BeliNft::new(
                String::from("Belize NFT"),
                String::from("BNFT"),
            );

            let uri = String::from("ipfs://QmTestHash");
            let token_id = nft.mint(accounts.bob, uri).unwrap();
            assert_eq!(nft.total_supply(), 1);

            // Bob burns his token
            set_caller(accounts.bob);
            let result = nft.burn(token_id);
            assert!(result.is_ok());

            assert_eq!(nft.owner_of(token_id), None);
            assert_eq!(nft.balance_of(accounts.bob), 0);
            assert_eq!(nft.total_supply(), 0);
        }

        #[ink::test]
        fn transfer_ownership_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut nft = BeliNft::new(
                String::from("Belize NFT"),
                String::from("BNFT"),
            );

            assert_eq!(nft.contract_owner(), accounts.alice);

            let result = nft.transfer_ownership(accounts.bob);
            assert!(result.is_ok());
            assert_eq!(nft.contract_owner(), accounts.bob);

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
        fn multiple_mints_increment_ids() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            
            let mut nft = BeliNft::new(
                String::from("Belize NFT"),
                String::from("BNFT"),
            );

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

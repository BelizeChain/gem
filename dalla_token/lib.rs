#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// # DALLA Token - PSP22 Compliant
///
/// The official wrapped token for BelizeChain's native DALLA currency.
/// Implements the PSP22 standard (Polkadot's ERC20 equivalent).
///
/// ## Features
/// - PSP22 standard compliance (transfer, approve, transferFrom)
/// - Role-based access control (admin, minter roles)
/// - Two-step ownership transfer
/// - Minting (controlled by minter role) and self-burning
/// - Total supply tracking with max supply cap
/// - Event emission for all state changes
/// - Allowance management with increase/decrease helpers
///
/// ## Economics
/// - Symbol: DALLA
/// - Decimals: 12 (same as native DALLA)
/// - Max Supply: 100 million DALLA (100_000_000 * 10^12)
/// - Initial Supply: 21 million DALLA (like Bitcoin's 21M)

#[ink::contract]
mod dalla_token {
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;

    /// Maximum supply cap: 100 million DALLA (with 12 decimals)
    const MAX_SUPPLY: u128 = 100_000_000_000_000_000_000;

    /// Role constants for access control
    const ADMIN_ROLE: u32 = 0;
    const MINTER_ROLE: u32 = 1;

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
        /// Operation only allowed by authorized role
        UnauthorizedAccess,
        /// Minting would exceed max supply
        ExceedsMaxSupply,
        /// Arithmetic overflow
        Overflow,
        /// Target address is zero
        ZeroAddress,
        /// Caller does not have the required role
        MissingRole,
        /// Caller is not the pending owner
        NotPendingOwner,
        /// Transfer blocked — account has active voting lock
        TransferWhileLocked,
        /// No pending code hash upgrade to execute
        NoPendingUpgrade,
        /// Timelock period has not elapsed
        TimelockNotReached,
        /// Code hash upgrade failed
        UpgradeFailed,
        /// Contract is paused
        ContractPaused,
    }

    /// Result type for DALLA operations
    pub type Result<T> = core::result::Result<T, Error>;

    /// The DALLA token storage
    #[ink(storage)]
    pub struct DallaToken {
        /// Total supply of DALLA tokens
        total_supply: u128,
        /// Maximum supply cap (kept for storage layout compatibility)
        max_supply: u128,
        /// Mapping from account to balance
        balances: Mapping<AccountId, u128>,
        /// Mapping from (owner, spender) to allowance
        allowances: Mapping<(AccountId, AccountId), u128>,
        /// Contract owner
        owner: AccountId,
        /// Role-based access control: (account, role) => granted
        roles: Mapping<(AccountId, u32), ()>,
        /// Pending owner for two-step ownership transfer
        pending_owner: Option<AccountId>,
        /// Voting lock: account → block number until which transfers are blocked
        locked_until: Mapping<AccountId, u32>,
        /// Authorized DAO contract that can set voting locks
        authorized_dao: Option<AccountId>,
        /// Proposed code hash for timelocked upgrade
        proposed_code_hash: Option<Hash>,
        /// Block number at which the code hash was proposed
        code_hash_proposal_block: u32,
        /// Emergency pause flag
        paused: bool,
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

    /// Event emitted when ownership is transferred
    #[ink(event)]
    pub struct OwnershipTransferred {
        #[ink(topic)]
        previous_owner: AccountId,
        #[ink(topic)]
        new_owner: AccountId,
    }

    /// Event emitted when a new owner is proposed
    #[ink(event)]
    pub struct OwnershipProposed {
        #[ink(topic)]
        current_owner: AccountId,
        #[ink(topic)]
        proposed_owner: AccountId,
    }

    /// Event emitted when the code hash is updated
    #[ink(event)]
    pub struct CodeHashUpdated {
        #[ink(topic)]
        new_code_hash: Hash,
    }

    /// Event emitted when a role is granted
    #[ink(event)]
    pub struct RoleGranted {
        #[ink(topic)]
        account: AccountId,
        role: u32,
    }

    /// Event emitted when a role is revoked
    #[ink(event)]
    pub struct RoleRevoked {
        #[ink(topic)]
        account: AccountId,
        role: u32,
    }

    /// Event emitted when voting lock is set
    #[ink(event)]
    pub struct VotingLockSet {
        #[ink(topic)]
        account: AccountId,
        until_block: u32,
    }

    /// Event emitted when authorized DAO is updated
    #[ink(event)]
    pub struct AuthorizedDaoUpdated {
        #[ink(topic)]
        dao: AccountId,
    }

    impl DallaToken {
        /// Creates a new DALLA token contract with initial supply
        #[ink(constructor)]
        pub fn new(initial_supply: u128) -> Self {
            assert!(initial_supply <= MAX_SUPPLY, "initial supply exceeds max");

            let caller = Self::env().caller();

            let mut balances = Mapping::default();
            balances.insert(caller, &initial_supply);

            let mut roles = Mapping::default();
            roles.insert((caller, ADMIN_ROLE), &());
            roles.insert((caller, MINTER_ROLE), &());

            Self::env().emit_event(Transfer {
                from: None,
                to: Some(caller),
                value: initial_supply,
            });

            Self {
                total_supply: initial_supply,
                max_supply: MAX_SUPPLY,
                balances,
                allowances: Mapping::default(),
                owner: caller,
                roles,
                pending_owner: None,
                locked_until: Mapping::default(),
                authorized_dao: None,
                proposed_code_hash: None,
                code_hash_proposal_block: 0,
                paused: false,
            }
        }

        // ====================================================================
        // PSP22 Metadata
        // ====================================================================

        /// Returns the token name
        #[ink(message, selector = 0x3d261bd4)]
        pub fn token_name(&self) -> Option<String> {
            Some(String::from("DALLA Token"))
        }

        /// Returns the token symbol
        #[ink(message, selector = 0x34205be5)]
        pub fn token_symbol(&self) -> Option<String> {
            Some(String::from("DALLA"))
        }

        /// Returns the token decimals
        #[ink(message, selector = 0x7271b782)]
        pub fn token_decimals(&self) -> u8 {
            12
        }

        // ====================================================================
        // PSP22 Core
        // ====================================================================

        /// Returns the total supply
        #[ink(message, selector = 0x162df8c2)]
        pub fn total_supply(&self) -> u128 {
            self.total_supply
        }

        /// Returns the maximum supply cap
        #[ink(message)]
        pub fn max_supply(&self) -> u128 {
            MAX_SUPPLY
        }

        /// Returns the balance of an account
        #[ink(message, selector = 0x6568_2523)]
        pub fn balance_of(&self, owner: AccountId) -> u128 {
            self.balances.get(owner).unwrap_or(0)
        }

        /// Returns the allowance granted by owner to spender
        #[ink(message, selector = 0x4d47d921)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> u128 {
            self.allowances.get((owner, spender)).unwrap_or(0)
        }

        /// Transfers tokens from caller to recipient (PSP22)
        #[ink(message, selector = 0xdb20f9f5)]
        pub fn transfer(&mut self, to: AccountId, value: u128, _data: Vec<u8>) -> Result<()> {
            let from = self.env().caller();
            self.transfer_from_to(from, to, value)
        }

        /// Approves spender to spend tokens on behalf of caller
        #[ink(message, selector = 0xb20f1bbd)]
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

        /// Transfers tokens from one account to another using allowance (PSP22)
        #[ink(message, selector = 0x54b3c76e)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: u128,
            _data: Vec<u8>,
        ) -> Result<()> {
            let caller = self.env().caller();
            let allowance = self.allowance(from, caller);

            if allowance < value {
                return Err(Error::InsufficientAllowance);
            }

            // Deduct allowance BEFORE balance transfer (checks-effects-interactions)
            let new_allowance = allowance.saturating_sub(value);
            self.allowances.insert((from, caller), &new_allowance);

            self.transfer_from_to(from, to, value)?;

            Ok(())
        }

        // ====================================================================
        // PSP22 Allowance Extensions
        // ====================================================================

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

        // ====================================================================
        // Mint / Burn
        // ====================================================================

        /// Mints new tokens (requires MINTER_ROLE)
        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, value: u128) -> Result<()> {
            self.ensure_not_paused()?;
            let caller = self.env().caller();
            self.ensure_role(caller, MINTER_ROLE)?;

            if to == AccountId::from([0u8; 32]) {
                return Err(Error::InvalidRecipient);
            }

            let new_supply = self
                .total_supply
                .checked_add(value)
                .ok_or(Error::Overflow)?;
            if new_supply > MAX_SUPPLY {
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
            self.total_supply = self
                .total_supply
                .checked_sub(value)
                .ok_or(Error::Overflow)?;

            self.env().emit_event(Transfer {
                from: Some(caller),
                to: None,
                value,
            });

            Ok(())
        }

        // ====================================================================
        // Ownership (two-step transfer)
        // ====================================================================

        /// Proposes a new owner (step 1 of two-step transfer)
        #[ink(message)]
        pub fn propose_ownership(&mut self, proposed: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err(Error::UnauthorizedAccess);
            }
            if proposed == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            self.pending_owner = Some(proposed);

            self.env().emit_event(OwnershipProposed {
                current_owner: caller,
                proposed_owner: proposed,
            });

            Ok(())
        }

        /// Accepts pending ownership (step 2 of two-step transfer)
        #[ink(message)]
        pub fn accept_ownership(&mut self) -> Result<()> {
            let caller = self.env().caller();
            match self.pending_owner {
                Some(pending) if pending == caller => {
                    let previous_owner = self.owner;
                    self.owner = caller;
                    self.pending_owner = None;

                    // Transfer all roles from previous owner to new owner
                    if self.roles.contains((previous_owner, ADMIN_ROLE)) {
                        self.roles.remove((previous_owner, ADMIN_ROLE));
                        self.roles.insert((caller, ADMIN_ROLE), &());
                    }
                    if self.roles.contains((previous_owner, MINTER_ROLE)) {
                        self.roles.remove((previous_owner, MINTER_ROLE));
                        self.roles.insert((caller, MINTER_ROLE), &());
                    }

                    self.env().emit_event(OwnershipTransferred {
                        previous_owner,
                        new_owner: caller,
                    });

                    Ok(())
                }
                _ => Err(Error::NotPendingOwner),
            }
        }

        /// Returns the contract owner
        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner
        }

        /// Returns the pending owner (if any)
        #[ink(message)]
        pub fn pending_owner(&self) -> Option<AccountId> {
            self.pending_owner
        }

        // ====================================================================
        // Role-Based Access Control
        // ====================================================================

        /// Grants a role to an account (admin only)
        #[ink(message)]
        pub fn grant_role(&mut self, account: AccountId, role: u32) -> Result<()> {
            let caller = self.env().caller();
            self.ensure_role(caller, ADMIN_ROLE)?;

            if account == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            self.roles.insert((account, role), &());

            self.env().emit_event(RoleGranted { account, role });

            Ok(())
        }

        /// Revokes a role from an account (admin only)
        #[ink(message)]
        pub fn revoke_role(&mut self, account: AccountId, role: u32) -> Result<()> {
            let caller = self.env().caller();
            self.ensure_role(caller, ADMIN_ROLE)?;

            self.roles.remove((account, role));

            self.env().emit_event(RoleRevoked { account, role });

            Ok(())
        }

        /// Checks if an account has a specific role
        #[ink(message)]
        pub fn has_role(&self, account: AccountId, role: u32) -> bool {
            self.roles.contains((account, role))
        }

        // ====================================================================
        // Upgrade (timelocked)
        // ====================================================================

        /// Minimum delay in blocks before a proposed upgrade can be executed
        /// (~24 hours at 12s block time = 7200 blocks)
        const UPGRADE_TIMELOCK_BLOCKS: u32 = 7200;

        /// Proposes a new code hash for upgrade (admin only, step 1)
        #[ink(message)]
        pub fn propose_code_hash_upgrade(&mut self, new_code_hash: Hash) -> Result<()> {
            let caller = self.env().caller();
            self.ensure_role(caller, ADMIN_ROLE)?;

            self.proposed_code_hash = Some(new_code_hash);
            self.code_hash_proposal_block = self.env().block_number();

            Ok(())
        }

        /// Executes a previously proposed upgrade after timelock (admin only, step 2)
        #[ink(message)]
        pub fn execute_code_hash_upgrade(&mut self) -> Result<()> {
            let caller = self.env().caller();
            self.ensure_role(caller, ADMIN_ROLE)?;

            let new_code_hash = self.proposed_code_hash.ok_or(Error::NoPendingUpgrade)?;
            let current_block = self.env().block_number();
            let elapsed = current_block.saturating_sub(self.code_hash_proposal_block);
            if elapsed < Self::UPGRADE_TIMELOCK_BLOCKS {
                return Err(Error::TimelockNotReached);
            }

            self.proposed_code_hash = None;
            self.code_hash_proposal_block = 0;

            ink::env::set_code_hash::<Environment>(&new_code_hash)
                .map_err(|_| Error::UpgradeFailed)?;

            self.env().emit_event(CodeHashUpdated { new_code_hash });

            Ok(())
        }

        /// Cancel a pending code hash upgrade proposal (admin only)
        #[ink(message)]
        pub fn cancel_code_hash_upgrade(&mut self) -> Result<()> {
            let caller = self.env().caller();
            self.ensure_role(caller, ADMIN_ROLE)?;

            self.proposed_code_hash = None;
            self.code_hash_proposal_block = 0;

            Ok(())
        }

        /// Returns the pending code hash upgrade (if any)
        #[ink(message)]
        pub fn pending_code_hash_upgrade(&self) -> Option<Hash> {
            self.proposed_code_hash
        }

        // ====================================================================
        // Governance Integration (Voting Locks)
        // ====================================================================

        /// Sets the authorized DAO contract that can lock tokens for voting (admin only)
        #[ink(message)]
        pub fn set_authorized_dao(&mut self, dao: AccountId) -> Result<()> {
            let caller = self.env().caller();
            self.ensure_role(caller, ADMIN_ROLE)?;

            if dao == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            self.authorized_dao = Some(dao);

            self.env().emit_event(AuthorizedDaoUpdated { dao });

            Ok(())
        }

        /// Returns the authorized DAO contract address
        #[ink(message)]
        pub fn authorized_dao(&self) -> Option<AccountId> {
            self.authorized_dao
        }

        /// Locks an account's tokens until a given block (callable only by authorized DAO)
        ///
        /// The lock is only extended, never reduced — if the account already has
        /// a longer lock, the existing lock is preserved. Returns `true` on success,
        /// `false` if the caller is not the authorized DAO.
        #[ink(message, selector = 0x6C6F636B)]
        pub fn lock_for_voting(&mut self, account: AccountId, until_block: u32) -> bool {
            let caller = self.env().caller();
            match self.authorized_dao {
                Some(dao) if dao == caller => {
                    let current_lock = self.locked_until.get(account).unwrap_or(0);
                    if until_block > current_lock {
                        self.locked_until.insert(account, &until_block);
                        self.env().emit_event(VotingLockSet {
                            account,
                            until_block,
                        });
                    }
                    true
                }
                _ => false,
            }
        }

        /// Returns the block number until which an account's tokens are locked
        #[ink(message)]
        pub fn get_voting_lock(&self, account: AccountId) -> u32 {
            self.locked_until.get(account).unwrap_or(0)
        }

        // ====================================================================
        // Internal
        // ====================================================================

        /// Internal transfer function
        fn transfer_from_to(&mut self, from: AccountId, to: AccountId, value: u128) -> Result<()> {
            self.ensure_not_paused()?;

            if to == AccountId::from([0u8; 32]) {
                return Err(Error::InvalidRecipient);
            }

            // Check voting lock — prevent transfers while tokens are locked
            let lock_until = self.locked_until.get(from).unwrap_or(0);
            if lock_until > 0 && self.env().block_number() <= lock_until {
                return Err(Error::TransferWhileLocked);
            }

            let from_balance = self.balance_of(from);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            // Self-transfer: skip double storage write, just emit event
            if from == to {
                self.env().emit_event(Transfer {
                    from: Some(from),
                    to: Some(to),
                    value,
                });
                return Ok(());
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

        /// Ensures the caller has the required role
        fn ensure_role(&self, account: AccountId, role: u32) -> Result<()> {
            if self.roles.contains((account, role)) {
                Ok(())
            } else {
                Err(Error::MissingRole)
            }
        }

        /// Ensures the contract is not paused
        fn ensure_not_paused(&self) -> Result<()> {
            if self.paused {
                Err(Error::ContractPaused)
            } else {
                Ok(())
            }
        }

        // ====================================================================
        // Emergency Pause
        // ====================================================================

        /// Pauses the contract (admin only)
        #[ink(message)]
        pub fn pause(&mut self) -> Result<()> {
            let caller = self.env().caller();
            self.ensure_role(caller, ADMIN_ROLE)?;
            self.paused = true;
            Ok(())
        }

        /// Unpauses the contract (admin only)
        #[ink(message)]
        pub fn unpause(&mut self) -> Result<()> {
            let caller = self.env().caller();
            self.ensure_role(caller, ADMIN_ROLE)?;
            self.paused = false;
            Ok(())
        }

        /// Returns whether the contract is paused
        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.paused
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn set_caller(account: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(account);
        }

        #[ink::test]
        fn new_works() {
            let initial_supply = 21_000_000_000_000_000_000_u128;
            let token = DallaToken::new(initial_supply);

            assert_eq!(token.total_supply(), initial_supply);
            assert_eq!(token.token_symbol(), Some(String::from("DALLA")));
            assert_eq!(token.token_decimals(), 12);
            assert_eq!(token.max_supply(), MAX_SUPPLY);
        }

        #[ink::test]
        #[should_panic(expected = "initial supply exceeds max")]
        fn new_rejects_excessive_supply() {
            let _ = DallaToken::new(MAX_SUPPLY + 1);
        }

        #[ink::test]
        fn balance_of_works() {
            let accs = accounts();
            let initial_supply = 1_000_000_000_000_u128;
            let token = DallaToken::new(initial_supply);

            assert_eq!(token.balance_of(accs.alice), initial_supply);
            assert_eq!(token.balance_of(accs.bob), 0);
        }

        #[ink::test]
        fn transfer_works() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.transfer(accs.bob, 100_000_000_000, Vec::new()).is_ok());
            assert_eq!(token.balance_of(accs.alice), 900_000_000_000);
            assert_eq!(token.balance_of(accs.bob), 100_000_000_000);
        }

        #[ink::test]
        fn transfer_fails_insufficient_balance() {
            let accs = accounts();
            let mut token = DallaToken::new(100_000_000_000_u128);

            let result = token.transfer(accs.bob, 200_000_000_000, Vec::new());
            assert_eq!(result, Err(Error::InsufficientBalance));
        }

        #[ink::test]
        fn transfer_rejects_zero_address() {
            let mut token = DallaToken::new(1_000_000_000_000_u128);
            let zero = AccountId::from([0u8; 32]);

            let result = token.transfer(zero, 100, Vec::new());
            assert_eq!(result, Err(Error::InvalidRecipient));
        }

        #[ink::test]
        fn self_transfer_preserves_balance() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.transfer(accs.alice, 100_000_000_000, Vec::new()).is_ok());
            assert_eq!(token.balance_of(accs.alice), 1_000_000_000_000);
        }

        #[ink::test]
        fn approve_works() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.approve(accs.bob, 100_000_000_000).is_ok());
            assert_eq!(token.allowance(accs.alice, accs.bob), 100_000_000_000);
        }

        #[ink::test]
        fn transfer_from_works() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.approve(accs.bob, 100_000_000_000).is_ok());
            set_caller(accs.bob);

            assert!(token
                .transfer_from(accs.alice, accs.charlie, 50_000_000_000, Vec::new())
                .is_ok());

            assert_eq!(token.balance_of(accs.alice), 950_000_000_000);
            assert_eq!(token.balance_of(accs.charlie), 50_000_000_000);
            assert_eq!(token.allowance(accs.alice, accs.bob), 50_000_000_000);
        }

        #[ink::test]
        fn transfer_from_deducts_allowance_before_transfer() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.approve(accs.bob, 100_000_000_000).is_ok());
            set_caller(accs.bob);

            assert!(token
                .transfer_from(accs.alice, accs.charlie, 100_000_000_000, Vec::new())
                .is_ok());
            assert_eq!(token.allowance(accs.alice, accs.bob), 0);
        }

        #[ink::test]
        fn mint_works() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.mint(accs.bob, 500_000_000_000).is_ok());
            assert_eq!(token.total_supply(), 1_500_000_000_000);
            assert_eq!(token.balance_of(accs.bob), 500_000_000_000);
        }

        #[ink::test]
        fn mint_fails_exceeds_max_supply() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            let result = token.mint(accs.bob, MAX_SUPPLY);
            assert_eq!(result, Err(Error::ExceedsMaxSupply));
        }

        #[ink::test]
        fn mint_rejects_zero_address() {
            let mut token = DallaToken::new(1_000_000_000_000_u128);
            let zero = AccountId::from([0u8; 32]);

            let result = token.mint(zero, 100);
            assert_eq!(result, Err(Error::InvalidRecipient));
        }

        #[ink::test]
        fn mint_requires_minter_role() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            set_caller(accs.bob);
            let result = token.mint(accs.charlie, 100);
            assert_eq!(result, Err(Error::MissingRole));
        }

        #[ink::test]
        fn burn_works() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.burn(200_000_000_000).is_ok());
            assert_eq!(token.total_supply(), 800_000_000_000);
            assert_eq!(token.balance_of(accs.alice), 800_000_000_000);
        }

        #[ink::test]
        fn increase_allowance_works() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.approve(accs.bob, 100_000_000_000).is_ok());
            assert!(token.increase_allowance(accs.bob, 50_000_000_000).is_ok());
            assert_eq!(token.allowance(accs.alice, accs.bob), 150_000_000_000);
        }

        #[ink::test]
        fn decrease_allowance_works() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.approve(accs.bob, 100_000_000_000).is_ok());
            assert!(token.decrease_allowance(accs.bob, 30_000_000_000).is_ok());
            assert_eq!(token.allowance(accs.alice, accs.bob), 70_000_000_000);
        }

        #[ink::test]
        fn two_step_ownership_transfer() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            // Step 1: Propose
            assert!(token.propose_ownership(accs.bob).is_ok());
            assert_eq!(token.pending_owner(), Some(accs.bob));
            assert_eq!(token.owner(), accs.alice);

            // Non-pending owner cannot accept
            set_caller(accs.charlie);
            assert_eq!(token.accept_ownership(), Err(Error::NotPendingOwner));

            // Step 2: Accept as pending owner
            set_caller(accs.bob);
            assert!(token.accept_ownership().is_ok());
            assert_eq!(token.owner(), accs.bob);
            assert_eq!(token.pending_owner(), None);
        }

        #[ink::test]
        fn propose_ownership_rejects_zero_address() {
            let mut token = DallaToken::new(1_000_000_000_000_u128);
            let zero = AccountId::from([0u8; 32]);

            assert_eq!(token.propose_ownership(zero), Err(Error::ZeroAddress));
        }

        #[ink::test]
        fn role_management_works() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            // Owner (alice) has admin and minter roles
            assert!(token.has_role(accs.alice, ADMIN_ROLE));
            assert!(token.has_role(accs.alice, MINTER_ROLE));

            // Grant minter role to bob
            assert!(token.grant_role(accs.bob, MINTER_ROLE).is_ok());
            assert!(token.has_role(accs.bob, MINTER_ROLE));

            // Bob can now mint
            set_caller(accs.bob);
            assert!(token.mint(accs.charlie, 100).is_ok());

            // Revoke bob's minter role (must be admin)
            set_caller(accs.alice);
            assert!(token.revoke_role(accs.bob, MINTER_ROLE).is_ok());

            // Bob can no longer mint
            set_caller(accs.bob);
            assert_eq!(token.mint(accs.charlie, 100), Err(Error::MissingRole));
        }

        #[ink::test]
        fn grant_role_requires_admin() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            set_caller(accs.bob);
            assert_eq!(
                token.grant_role(accs.charlie, MINTER_ROLE),
                Err(Error::MissingRole)
            );
        }

        #[ink::test]
        fn grant_role_rejects_zero_address() {
            let mut token = DallaToken::new(1_000_000_000_000_u128);
            let zero = AccountId::from([0u8; 32]);

            assert_eq!(token.grant_role(zero, MINTER_ROLE), Err(Error::ZeroAddress));
        }

        // ── Voting lock tests ───────────────────────────────────────────

        #[ink::test]
        fn set_authorized_dao_works() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            assert!(token.set_authorized_dao(accs.bob).is_ok());
            assert_eq!(token.authorized_dao(), Some(accs.bob));
        }

        #[ink::test]
        fn set_authorized_dao_requires_admin() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            set_caller(accs.bob);
            assert_eq!(
                token.set_authorized_dao(accs.charlie),
                Err(Error::MissingRole)
            );
        }

        #[ink::test]
        fn set_authorized_dao_rejects_zero_address() {
            let mut token = DallaToken::new(1_000_000_000_000_u128);
            let zero = AccountId::from([0u8; 32]);

            assert_eq!(token.set_authorized_dao(zero), Err(Error::ZeroAddress));
        }

        #[ink::test]
        fn lock_for_voting_works() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            token.set_authorized_dao(accs.bob).unwrap();
            set_caller(accs.bob);
            assert!(token.lock_for_voting(accs.alice, 100));
            assert_eq!(token.get_voting_lock(accs.alice), 100);
        }

        #[ink::test]
        fn lock_for_voting_unauthorized_returns_false() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            set_caller(accs.charlie);
            assert!(!token.lock_for_voting(accs.alice, 100));
            assert_eq!(token.get_voting_lock(accs.alice), 0);
        }

        #[ink::test]
        fn lock_for_voting_extends_to_max() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            token.set_authorized_dao(accs.bob).unwrap();
            set_caller(accs.bob);

            assert!(token.lock_for_voting(accs.alice, 100));
            assert_eq!(token.get_voting_lock(accs.alice), 100);

            // Extend to 200
            assert!(token.lock_for_voting(accs.alice, 200));
            assert_eq!(token.get_voting_lock(accs.alice), 200);

            // Attempt to reduce to 50 — lock stays at 200
            assert!(token.lock_for_voting(accs.alice, 50));
            assert_eq!(token.get_voting_lock(accs.alice), 200);
        }

        #[ink::test]
        fn transfer_while_locked_fails() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            token.set_authorized_dao(accs.bob).unwrap();
            set_caller(accs.bob);
            token.lock_for_voting(accs.alice, 100);

            set_caller(accs.alice);
            let result = token.transfer(accs.charlie, 100, Vec::new());
            assert_eq!(result, Err(Error::TransferWhileLocked));
        }

        #[ink::test]
        fn transfer_from_while_locked_fails() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            // Approve bob
            token.approve(accs.bob, 500_000_000_000).unwrap();

            // Lock alice's tokens
            token.set_authorized_dao(accs.charlie).unwrap();
            set_caller(accs.charlie);
            token.lock_for_voting(accs.alice, 100);

            // Bob tries transfer_from — should fail because alice is locked
            set_caller(accs.bob);
            let result = token.transfer_from(accs.alice, accs.charlie, 100, Vec::new());
            assert_eq!(result, Err(Error::TransferWhileLocked));
        }

        #[ink::test]
        fn transfer_after_lock_expires_works() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            token.set_authorized_dao(accs.bob).unwrap();
            set_caller(accs.bob);
            token.lock_for_voting(accs.alice, 5);

            // Advance past the lock
            ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(6);

            set_caller(accs.alice);
            assert!(token.transfer(accs.charlie, 100, Vec::new()).is_ok());
        }

        #[ink::test]
        fn unlocked_account_can_transfer() {
            let accs = accounts();
            let mut token = DallaToken::new(1_000_000_000_000_u128);

            // No lock set — transfers work normally
            assert!(token.transfer(accs.bob, 100, Vec::new()).is_ok());
        }

        #[ink::test]
        fn get_voting_lock_default_zero() {
            let accs = accounts();
            let token = DallaToken::new(1_000_000_000_000_u128);

            assert_eq!(token.get_voting_lock(accs.alice), 0);
            assert_eq!(token.get_voting_lock(accs.bob), 0);
        }
    }
}

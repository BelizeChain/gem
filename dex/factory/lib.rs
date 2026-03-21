#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod factory {
    use ink::storage::Mapping;
    use scale::{Decode, Encode};

    // ============================================================================
    // Storage
    // ============================================================================

    #[ink(storage)]
    pub struct Factory {
        /// Contract admin (can upgrade contract code and pair code hash)
        admin: AccountId,

        /// Pending admin for two-step transfer
        pending_admin: Option<AccountId>,

        /// Fee recipient address (receives 1/6 of trading fees)
        fee_to: Option<AccountId>,

        /// Fee setter (can change fee_to)
        fee_to_setter: AccountId,

        /// Pending fee setter for two-step transfer
        pending_fee_to_setter: Option<AccountId>,

        /// All created pairs: index => pair_address
        all_pairs: Mapping<u32, AccountId>,

        /// Pair addresses by tokens: (token0, token1) => pair_address
        /// Note: token0 < token1 (lexicographically sorted)
        get_pair: Mapping<(AccountId, AccountId), AccountId>,

        /// Total number of pairs created
        all_pairs_length: u32,

        /// Pair contract code hash (for instantiation)
        pair_code_hash: Hash,

        /// Reentrancy lock
        locked: bool,

        /// Proposed factory code hash for timelocked upgrade
        proposed_code_hash: Option<Hash>,

        /// Block at which factory code hash was proposed
        code_hash_proposal_block: u32,

        /// Proposed pair code hash for timelocked update
        proposed_pair_code_hash: Option<Hash>,

        /// Block at which pair code hash was proposed
        pair_code_hash_proposal_block: u32,
    }

    // ============================================================================
    // Events
    // ============================================================================

    #[ink(event)]
    pub struct PairCreated {
        #[ink(topic)]
        token0: AccountId,
        #[ink(topic)]
        token1: AccountId,
        pair: AccountId,
        pair_number: u32,
    }

    #[ink(event)]
    pub struct FeeToSet {
        #[ink(topic)]
        old_fee_to: Option<AccountId>,
        #[ink(topic)]
        new_fee_to: Option<AccountId>,
    }

    #[ink(event)]
    pub struct FeeToSetterSet {
        #[ink(topic)]
        old_setter: AccountId,
        #[ink(topic)]
        new_setter: AccountId,
    }

    #[ink(event)]
    pub struct AdminTransferred {
        #[ink(topic)]
        old_admin: AccountId,
        #[ink(topic)]
        new_admin: AccountId,
    }

    #[ink(event)]
    pub struct PairCodeHashUpdated {
        old_code_hash: Hash,
        new_code_hash: Hash,
    }

    #[ink(event)]
    pub struct CodeHashUpdated {
        #[ink(topic)]
        new_code_hash: Hash,
    }

    // ============================================================================
    // Errors
    // ============================================================================

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Identical token addresses
        IdenticalAddresses,
        /// Zero address provided
        ZeroAddress,
        /// Pair already exists
        PairExists,
        /// Not authorized
        NotAuthorized,
        /// Pair instantiation failed
        PairInstantiationFailed,
        /// Arithmetic overflow
        Overflow,
        /// Reentrancy detected
        Locked,
        /// No pending transfer to accept
        NoPendingTransfer,
        /// Contract upgrade failed
        UpgradeFailed,
        /// No pending code hash upgrade
        NoPendingUpgrade,
        /// Timelock period has not elapsed
        TimelockNotReached,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    // ============================================================================
    // Implementation
    // ============================================================================

    impl Factory {
        // ========================================================================
        // Constructor
        // ========================================================================

        /// Create a new DEX factory
        ///
        /// # Parameters
        /// * `admin` - Contract admin (can upgrade code, set pair code hash)
        /// * `fee_to_setter` - Address that can change fee recipient
        /// * `pair_code_hash` - Code hash of Pair contract (for instantiation)
        #[ink(constructor)]
        pub fn new(admin: AccountId, fee_to_setter: AccountId, pair_code_hash: Hash) -> Self {
            let zero = AccountId::from([0u8; 32]);
            assert!(admin != zero, "admin cannot be zero address");
            assert!(
                fee_to_setter != zero,
                "fee_to_setter cannot be zero address"
            );
            Self {
                admin,
                pending_admin: None,
                fee_to: None,
                fee_to_setter,
                pending_fee_to_setter: None,
                all_pairs: Mapping::default(),
                get_pair: Mapping::default(),
                all_pairs_length: 0,
                pair_code_hash,
                locked: false,
                proposed_code_hash: None,
                code_hash_proposal_block: 0,
                proposed_pair_code_hash: None,
                pair_code_hash_proposal_block: 0,
            }
        }

        // ========================================================================
        // View Functions
        // ========================================================================

        /// Get pair address for two tokens
        #[ink(message)]
        pub fn get_pair_address(
            &self,
            token_a: AccountId,
            token_b: AccountId,
        ) -> Option<AccountId> {
            let (token0, token1) = Self::sort_tokens(token_a, token_b).ok()?;
            self.get_pair.get((token0, token1))
        }

        /// Get total number of pairs
        #[ink(message)]
        pub fn all_pairs_length(&self) -> u32 {
            self.all_pairs_length
        }

        /// Get pair address by index
        #[ink(message)]
        pub fn get_pair_by_index(&self, index: u32) -> Option<AccountId> {
            if index >= self.all_pairs_length {
                None
            } else {
                self.all_pairs.get(index)
            }
        }

        /// Get fee recipient address
        #[ink(message)]
        pub fn fee_to(&self) -> Option<AccountId> {
            self.fee_to
        }

        /// Get fee setter address
        #[ink(message)]
        pub fn fee_to_setter(&self) -> AccountId {
            self.fee_to_setter
        }

        /// Get pending fee setter address
        #[ink(message)]
        pub fn pending_fee_to_setter(&self) -> Option<AccountId> {
            self.pending_fee_to_setter
        }

        /// Get contract admin address
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Get pending admin address
        #[ink(message)]
        pub fn pending_admin(&self) -> Option<AccountId> {
            self.pending_admin
        }

        /// Get pair contract code hash
        #[ink(message)]
        pub fn pair_code_hash(&self) -> Hash {
            self.pair_code_hash
        }

        // ========================================================================
        // State-Changing Functions
        // ========================================================================

        /// Create a new trading pair
        ///
        /// # Parameters
        /// * `token_a` - First token address
        /// * `token_b` - Second token address
        ///
        /// # Returns
        /// Address of the newly created pair
        ///
        /// # Requirements
        /// - Tokens must be different
        /// - Neither token can be zero address
        /// - Pair must not already exist
        #[ink(message)]
        pub fn create_pair(&mut self, token_a: AccountId, token_b: AccountId) -> Result<AccountId> {
            // Reentrancy guard
            if self.locked {
                return Err(Error::Locked);
            }
            self.locked = true;

            let result = self._create_pair_inner(token_a, token_b);

            self.locked = false;
            result
        }

        /// Set fee recipient address (fee_to_setter only)
        ///
        /// # Parameters
        /// * `fee_to` - New fee recipient address (or None to disable fees)
        #[ink(message)]
        pub fn set_fee_to(&mut self, fee_to: Option<AccountId>) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.fee_to_setter {
                return Err(Error::NotAuthorized);
            }

            // Reject zero address wrapped in Some — use None to disable fees
            if let Some(addr) = fee_to {
                if addr == AccountId::from([0u8; 32]) {
                    return Err(Error::ZeroAddress);
                }
            }

            let old_fee_to = self.fee_to;
            self.fee_to = fee_to;

            self.env().emit_event(FeeToSet {
                old_fee_to,
                new_fee_to: fee_to,
            });

            Ok(())
        }

        /// Propose a new fee setter address (fee_to_setter only, two-step)
        ///
        /// The new setter must call `accept_fee_to_setter()` to complete the transfer.
        #[ink(message)]
        pub fn set_fee_to_setter(&mut self, new_setter: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.fee_to_setter {
                return Err(Error::NotAuthorized);
            }

            if new_setter == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            self.pending_fee_to_setter = Some(new_setter);
            Ok(())
        }

        /// Accept the fee setter role (pending setter only)
        #[ink(message)]
        pub fn accept_fee_to_setter(&mut self) -> Result<()> {
            let pending = self.pending_fee_to_setter.ok_or(Error::NoPendingTransfer)?;
            let caller = self.env().caller();
            if caller != pending {
                return Err(Error::NotAuthorized);
            }

            let old_setter = self.fee_to_setter;
            self.fee_to_setter = pending;
            self.pending_fee_to_setter = None;

            self.env().emit_event(FeeToSetterSet {
                old_setter,
                new_setter: pending,
            });

            Ok(())
        }

        /// Propose a new admin (admin only, two-step)
        ///
        /// The new admin must call `accept_admin()` to complete the transfer.
        #[ink(message)]
        pub fn set_admin(&mut self, new_admin: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotAuthorized);
            }

            if new_admin == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            self.pending_admin = Some(new_admin);
            Ok(())
        }

        /// Accept the admin role (pending admin only)
        #[ink(message)]
        pub fn accept_admin(&mut self) -> Result<()> {
            let pending = self.pending_admin.ok_or(Error::NoPendingTransfer)?;
            let caller = self.env().caller();
            if caller != pending {
                return Err(Error::NotAuthorized);
            }

            let old_admin = self.admin;
            self.admin = pending;
            self.pending_admin = None;

            self.env().emit_event(AdminTransferred {
                old_admin,
                new_admin: pending,
            });

            Ok(())
        }

        /// Minimum delay in blocks before a proposed upgrade can be executed
        /// (~24 hours at 12s block time = 7200 blocks)
        const UPGRADE_TIMELOCK_BLOCKS: u32 = 7200;

        /// Propose a new pair contract code hash (admin only, step 1)
        ///
        /// Must wait UPGRADE_TIMELOCK_BLOCKS before executing.
        #[ink(message)]
        pub fn propose_pair_code_hash(&mut self, new_code_hash: Hash) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotAuthorized);
            }

            self.proposed_pair_code_hash = Some(new_code_hash);
            self.pair_code_hash_proposal_block = self.env().block_number();

            Ok(())
        }

        /// Execute a previously proposed pair code hash update (admin only, step 2)
        #[ink(message)]
        pub fn execute_pair_code_hash_update(&mut self) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotAuthorized);
            }

            let new_code_hash = self
                .proposed_pair_code_hash
                .ok_or(Error::NoPendingUpgrade)?;
            let current_block = self.env().block_number();
            let elapsed = current_block.saturating_sub(self.pair_code_hash_proposal_block);
            if elapsed < Self::UPGRADE_TIMELOCK_BLOCKS {
                return Err(Error::TimelockNotReached);
            }

            let old_code_hash = self.pair_code_hash;
            self.pair_code_hash = new_code_hash;
            self.proposed_pair_code_hash = None;
            self.pair_code_hash_proposal_block = 0;

            self.env().emit_event(PairCodeHashUpdated {
                old_code_hash,
                new_code_hash,
            });

            Ok(())
        }

        /// Propose a factory contract code upgrade (admin only, step 1)
        ///
        /// Must wait UPGRADE_TIMELOCK_BLOCKS before executing.
        #[ink(message)]
        pub fn propose_code_hash_upgrade(&mut self, new_code_hash: Hash) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotAuthorized);
            }

            self.proposed_code_hash = Some(new_code_hash);
            self.code_hash_proposal_block = self.env().block_number();

            Ok(())
        }

        /// Execute a previously proposed factory upgrade after timelock (admin only, step 2)
        #[ink(message)]
        pub fn execute_code_hash_upgrade(&mut self) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotAuthorized);
            }

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

        /// Cancel a pending upgrade proposal (admin only)
        #[ink(message)]
        pub fn cancel_pending_upgrades(&mut self) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotAuthorized);
            }

            self.proposed_code_hash = None;
            self.code_hash_proposal_block = 0;
            self.proposed_pair_code_hash = None;
            self.pair_code_hash_proposal_block = 0;

            Ok(())
        }

        // ========================================================================
        // Internal Functions
        // ========================================================================

        /// Inner create pair logic (called within reentrancy guard)
        fn _create_pair_inner(
            &mut self,
            token_a: AccountId,
            token_b: AccountId,
        ) -> Result<AccountId> {
            if token_a == token_b {
                return Err(Error::IdenticalAddresses);
            }

            let zero_address = AccountId::from([0u8; 32]);
            if token_a == zero_address || token_b == zero_address {
                return Err(Error::ZeroAddress);
            }

            // Sort tokens (token0 < token1)
            let (token0, token1) = Self::sort_tokens(token_a, token_b)?;

            // Check if pair already exists
            if self.get_pair.get((token0, token1)).is_some() {
                return Err(Error::PairExists);
            }

            // Instantiate new Pair contract
            let pair_address = self._create_pair_contract(token0, token1)?;

            // Store pair (sorted order only — get_pair_address() normalizes via sort_tokens)
            self.get_pair.insert((token0, token1), &pair_address);
            self.all_pairs.insert(self.all_pairs_length, &pair_address);

            let pair_number = self.all_pairs_length;
            self.all_pairs_length = self
                .all_pairs_length
                .checked_add(1)
                .ok_or(Error::Overflow)?;

            // Emit event
            self.env().emit_event(PairCreated {
                token0,
                token1,
                pair: pair_address,
                pair_number,
            });

            Ok(pair_address)
        }

        /// Sort token addresses (token0 < token1)
        fn sort_tokens(token_a: AccountId, token_b: AccountId) -> Result<(AccountId, AccountId)> {
            if token_a == token_b {
                return Err(Error::IdenticalAddresses);
            }

            // Lexicographic comparison
            if token_a < token_b {
                Ok((token_a, token_b))
            } else {
                Ok((token_b, token_a))
            }
        }

        /// Create pair contract instance (production: deploys via PairRef)
        ///
        /// Uses the stored `pair_code_hash` to instantiate the audited Pair contract.
        /// Salt is derived from both sorted token addresses for deterministic addressing.
        #[cfg(not(test))]
        fn _create_pair_contract(&self, token0: AccountId, token1: AccountId) -> Result<AccountId> {
            use belizex_pair::pair::PairRef;
            use ink::prelude::vec::Vec;
            use ink::ToAccountId;

            // Deterministic salt from sorted token addresses
            let mut salt = Vec::with_capacity(64);
            salt.extend_from_slice(token0.as_ref());
            salt.extend_from_slice(token1.as_ref());

            let pair = PairRef::new(token0, token1)
                .code_hash(self.pair_code_hash)
                .endowment(0)
                .salt_bytes(salt)
                .try_instantiate()
                .map_err(|_| Error::PairInstantiationFailed)?
                .map_err(|_| Error::PairInstantiationFailed)?;

            Ok(pair.to_account_id())
        }

        /// Create pair contract instance (test mock: deterministic address)
        ///
        /// Generates a unique address from both token addresses.
        #[cfg(test)]
        fn _create_pair_contract(&self, token0: AccountId, token1: AccountId) -> Result<AccountId> {
            let t0: &[u8] = token0.as_ref();
            let t1: &[u8] = token1.as_ref();
            let mut pair_bytes = [0u8; 32];
            for i in 0..16 {
                pair_bytes[i] = t0[i].wrapping_add(t1[i]);
            }
            for i in 0..16 {
                pair_bytes[16 + i] = t0[16 + i].wrapping_add(t1[16 + i]).wrapping_add(1);
            }
            Ok(AccountId::from(pair_bytes))
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        fn get_test_accounts() -> (AccountId, AccountId, AccountId, AccountId, AccountId) {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            (
                accounts.alice,
                accounts.bob,
                accounts.charlie,
                accounts.django,
                accounts.eve,
            )
        }

        fn default_code_hash() -> Hash {
            Hash::from([0x42; 32])
        }

        /// Helper: create factory with alice as admin, bob as fee_to_setter
        fn create_factory() -> Factory {
            let (admin, fee_setter, _, _, _) = get_test_accounts();
            Factory::new(admin, fee_setter, default_code_hash())
        }

        // ====================================================================
        // Constructor tests
        // ====================================================================

        #[ink::test]
        fn new_works() {
            let (admin, fee_setter, _, _, _) = get_test_accounts();
            let code_hash = default_code_hash();
            let factory = Factory::new(admin, fee_setter, code_hash);

            assert_eq!(factory.admin(), admin);
            assert_eq!(factory.fee_to_setter(), fee_setter);
            assert_eq!(factory.fee_to(), None);
            assert_eq!(factory.all_pairs_length(), 0);
            assert_eq!(factory.pair_code_hash(), code_hash);
            assert_eq!(factory.pending_admin(), None);
            assert_eq!(factory.pending_fee_to_setter(), None);
        }

        #[ink::test]
        #[should_panic(expected = "admin cannot be zero address")]
        fn new_rejects_zero_admin() {
            let (_, fee_setter, _, _, _) = get_test_accounts();
            let zero = AccountId::from([0u8; 32]);
            Factory::new(zero, fee_setter, default_code_hash());
        }

        #[ink::test]
        #[should_panic(expected = "fee_to_setter cannot be zero address")]
        fn new_rejects_zero_fee_setter() {
            let (admin, _, _, _, _) = get_test_accounts();
            let zero = AccountId::from([0u8; 32]);
            Factory::new(admin, zero, default_code_hash());
        }

        // ====================================================================
        // Pair creation tests
        // ====================================================================

        #[ink::test]
        fn create_pair_works() {
            let (_, _, token_a, token_b, _) = get_test_accounts();
            let mut factory = create_factory();

            let pair = factory.create_pair(token_a, token_b).unwrap();

            assert_eq!(factory.all_pairs_length(), 1);
            assert_eq!(factory.get_pair_address(token_a, token_b), Some(pair));
            assert_eq!(factory.get_pair_address(token_b, token_a), Some(pair));
            assert_eq!(factory.get_pair_by_index(0), Some(pair));
        }

        #[ink::test]
        fn create_pair_fails_identical_addresses() {
            let (_, _, token_a, _, _) = get_test_accounts();
            let mut factory = create_factory();

            assert_eq!(
                factory.create_pair(token_a, token_a),
                Err(Error::IdenticalAddresses)
            );
        }

        #[ink::test]
        fn create_pair_fails_zero_address() {
            let (_, _, token_a, _, _) = get_test_accounts();
            let zero = AccountId::from([0u8; 32]);
            let mut factory = create_factory();

            assert_eq!(factory.create_pair(token_a, zero), Err(Error::ZeroAddress));
            assert_eq!(factory.create_pair(zero, token_a), Err(Error::ZeroAddress));
        }

        #[ink::test]
        fn create_pair_fails_if_exists() {
            let (_, _, token_a, token_b, _) = get_test_accounts();
            let mut factory = create_factory();

            factory.create_pair(token_a, token_b).unwrap();

            assert_eq!(
                factory.create_pair(token_a, token_b),
                Err(Error::PairExists)
            );
            assert_eq!(
                factory.create_pair(token_b, token_a),
                Err(Error::PairExists)
            );
        }

        #[ink::test]
        fn create_multiple_pairs_unique_addresses() {
            let (_, _, token_a, token_b, token_c) = get_test_accounts();
            let mut factory = create_factory();

            let pair1 = factory.create_pair(token_a, token_b).unwrap();
            let pair2 = factory.create_pair(token_a, token_c).unwrap();
            let pair3 = factory.create_pair(token_b, token_c).unwrap();

            assert_eq!(factory.all_pairs_length(), 3);
            // All pairs must have distinct addresses (C02 fix)
            assert_ne!(pair1, pair2);
            assert_ne!(pair1, pair3);
            assert_ne!(pair2, pair3);
            assert_eq!(factory.get_pair_by_index(0), Some(pair1));
            assert_eq!(factory.get_pair_by_index(1), Some(pair2));
            assert_eq!(factory.get_pair_by_index(2), Some(pair3));
        }

        #[ink::test]
        fn get_pair_by_index_out_of_bounds() {
            let factory = create_factory();
            assert_eq!(factory.get_pair_by_index(0), None);
            assert_eq!(factory.get_pair_by_index(u32::MAX), None);
        }

        // ====================================================================
        // Fee management tests
        // ====================================================================

        #[ink::test]
        fn set_fee_to_works() {
            let (_, fee_setter, recipient, _, _) = get_test_accounts();
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(fee_setter);

            factory.set_fee_to(Some(recipient)).unwrap();
            assert_eq!(factory.fee_to(), Some(recipient));

            // Disable fees with None
            factory.set_fee_to(None).unwrap();
            assert_eq!(factory.fee_to(), None);
        }

        #[ink::test]
        fn set_fee_to_rejects_zero_address() {
            let (_, fee_setter, _, _, _) = get_test_accounts();
            let zero = AccountId::from([0u8; 32]);
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(fee_setter);
            assert_eq!(factory.set_fee_to(Some(zero)), Err(Error::ZeroAddress));
        }

        #[ink::test]
        fn set_fee_to_fails_not_authorized() {
            let (_, _, other, _, _) = get_test_accounts();
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(other);
            assert_eq!(factory.set_fee_to(Some(other)), Err(Error::NotAuthorized));
        }

        // ====================================================================
        // Two-step fee setter transfer tests
        // ====================================================================

        #[ink::test]
        fn set_fee_to_setter_two_step_works() {
            let (_, fee_setter, new_setter, _, _) = get_test_accounts();
            let mut factory = create_factory();

            // Step 1: Current setter proposes
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(fee_setter);
            factory.set_fee_to_setter(new_setter).unwrap();
            assert_eq!(factory.pending_fee_to_setter(), Some(new_setter));
            assert_eq!(factory.fee_to_setter(), fee_setter); // Not changed yet

            // Step 2: New setter accepts
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(new_setter);
            factory.accept_fee_to_setter().unwrap();
            assert_eq!(factory.fee_to_setter(), new_setter);
            assert_eq!(factory.pending_fee_to_setter(), None);
        }

        #[ink::test]
        fn set_fee_to_setter_rejects_zero() {
            let (_, fee_setter, _, _, _) = get_test_accounts();
            let zero = AccountId::from([0u8; 32]);
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(fee_setter);
            assert_eq!(factory.set_fee_to_setter(zero), Err(Error::ZeroAddress));
        }

        #[ink::test]
        fn set_fee_to_setter_fails_not_authorized() {
            let (_, _, other, _, _) = get_test_accounts();
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(other);
            assert_eq!(factory.set_fee_to_setter(other), Err(Error::NotAuthorized));
        }

        #[ink::test]
        fn accept_fee_to_setter_fails_no_pending() {
            let (_, fee_setter, _, _, _) = get_test_accounts();
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(fee_setter);
            assert_eq!(
                factory.accept_fee_to_setter(),
                Err(Error::NoPendingTransfer)
            );
        }

        #[ink::test]
        fn accept_fee_to_setter_fails_wrong_caller() {
            let (_, fee_setter, new_setter, wrong, _) = get_test_accounts();
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(fee_setter);
            factory.set_fee_to_setter(new_setter).unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(wrong);
            assert_eq!(factory.accept_fee_to_setter(), Err(Error::NotAuthorized));
        }

        // ====================================================================
        // Two-step admin transfer tests
        // ====================================================================

        #[ink::test]
        fn set_admin_two_step_works() {
            let (admin, _, new_admin, _, _) = get_test_accounts();
            let mut factory = create_factory();

            // Step 1: Admin proposes
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(admin);
            factory.set_admin(new_admin).unwrap();
            assert_eq!(factory.pending_admin(), Some(new_admin));
            assert_eq!(factory.admin(), admin); // Not changed yet

            // Step 2: New admin accepts
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(new_admin);
            factory.accept_admin().unwrap();
            assert_eq!(factory.admin(), new_admin);
            assert_eq!(factory.pending_admin(), None);
        }

        #[ink::test]
        fn set_admin_rejects_zero() {
            let (admin, _, _, _, _) = get_test_accounts();
            let zero = AccountId::from([0u8; 32]);
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(admin);
            assert_eq!(factory.set_admin(zero), Err(Error::ZeroAddress));
        }

        #[ink::test]
        fn set_admin_fails_not_authorized() {
            let (_, _, other, _, _) = get_test_accounts();
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(other);
            assert_eq!(factory.set_admin(other), Err(Error::NotAuthorized));
        }

        #[ink::test]
        fn accept_admin_fails_no_pending() {
            let (admin, _, _, _, _) = get_test_accounts();
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(admin);
            assert_eq!(factory.accept_admin(), Err(Error::NoPendingTransfer));
        }

        #[ink::test]
        fn accept_admin_fails_wrong_caller() {
            let (admin, _, new_admin, wrong, _) = get_test_accounts();
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(admin);
            factory.set_admin(new_admin).unwrap();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(wrong);
            assert_eq!(factory.accept_admin(), Err(Error::NotAuthorized));
        }

        // ====================================================================
        // Code hash & pair code hash tests
        // ====================================================================

        #[ink::test]
        fn propose_pair_code_hash_works() {
            let (admin, _, _, _, _) = get_test_accounts();
            let mut factory = create_factory();
            let new_hash = Hash::from([0xAA; 32]);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(admin);
            factory.propose_pair_code_hash(new_hash).unwrap();
            // pair_code_hash unchanged until timelock elapses and execute called
            assert_ne!(factory.pair_code_hash(), new_hash);
        }

        #[ink::test]
        fn propose_pair_code_hash_fails_not_admin() {
            let (_, fee_setter, _, _, _) = get_test_accounts();
            let mut factory = create_factory();
            let new_hash = Hash::from([0xAA; 32]);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(fee_setter);
            assert_eq!(
                factory.propose_pair_code_hash(new_hash),
                Err(Error::NotAuthorized)
            );
        }

        #[ink::test]
        fn propose_code_hash_upgrade_fails_not_admin() {
            let (_, fee_setter, _, _, _) = get_test_accounts();
            let mut factory = create_factory();
            let new_hash = Hash::from([0xBB; 32]);

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(fee_setter);
            assert_eq!(
                factory.propose_code_hash_upgrade(new_hash),
                Err(Error::NotAuthorized)
            );
        }

        // ====================================================================
        // Role separation verification
        // ====================================================================

        #[ink::test]
        fn fee_setter_cannot_upgrade_contract() {
            let (_, fee_setter, _, _, _) = get_test_accounts();
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(fee_setter);
            assert_eq!(
                factory.propose_code_hash_upgrade(Hash::from([0xCC; 32])),
                Err(Error::NotAuthorized)
            );
            assert_eq!(
                factory.propose_pair_code_hash(Hash::from([0xCC; 32])),
                Err(Error::NotAuthorized)
            );
            assert_eq!(factory.set_admin(fee_setter), Err(Error::NotAuthorized));
        }

        #[ink::test]
        fn admin_cannot_manage_fees() {
            let (admin, _, recipient, _, _) = get_test_accounts();
            let mut factory = create_factory();

            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(admin);
            assert_eq!(
                factory.set_fee_to(Some(recipient)),
                Err(Error::NotAuthorized)
            );
            assert_eq!(
                factory.set_fee_to_setter(recipient),
                Err(Error::NotAuthorized)
            );
        }
    }
}

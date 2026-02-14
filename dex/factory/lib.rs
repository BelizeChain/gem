#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod factory {
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use scale::{Decode, Encode};

    // ============================================================================
    // Storage
    // ============================================================================

    #[ink(storage)]
    pub struct Factory {
        // Fee recipient address (receives 1/6 of trading fees)
        fee_to: Option<AccountId>,

        // Fee setter (can change fee_to)
        fee_to_setter: AccountId,

        // All created pairs: index => pair_address
        all_pairs: Mapping<u32, AccountId>,

        // Pair addresses by tokens: (token0, token1) => pair_address
        // Note: token0 < token1 (lexicographically sorted)
        get_pair: Mapping<(AccountId, AccountId), AccountId>,

        // Total number of pairs created
        all_pairs_length: u32,

        // Pair contract code hash (for instantiation)
        pair_code_hash: Hash,
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
        /// Not authorized (requires fee_to_setter)
        NotAuthorized,
        /// Pair instantiation failed
        PairInstantiationFailed,
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
        /// * `fee_to_setter` - Address that can change fee recipient
        /// * `pair_code_hash` - Code hash of Pair contract (for instantiation)
        #[ink(constructor)]
        pub fn new(fee_to_setter: AccountId, pair_code_hash: Hash) -> Self {
            Self {
                fee_to: None,
                fee_to_setter,
                all_pairs: Mapping::default(),
                get_pair: Mapping::default(),
                all_pairs_length: 0,
                pair_code_hash,
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
            // Validate inputs
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
            // NOTE: In production, this would use ink::env::call::create_contract
            // For now, we'll simulate the pair address
            // TODO: Implement actual contract instantiation
            let pair_address = self._create_pair_contract(token0, token1)?;

            // Store pair
            self.get_pair.insert((token0, token1), &pair_address);
            self.get_pair.insert((token1, token0), &pair_address); // Both directions
            self.all_pairs.insert(self.all_pairs_length, &pair_address);

            // Emit event
            self.env().emit_event(PairCreated {
                token0,
                token1,
                pair: pair_address,
                pair_number: self.all_pairs_length,
            });

            self.all_pairs_length = self.all_pairs_length.saturating_add(1);

            Ok(pair_address)
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

            let old_fee_to = self.fee_to;
            self.fee_to = fee_to;

            self.env().emit_event(FeeToSet {
                old_fee_to,
                new_fee_to: fee_to,
            });

            Ok(())
        }

        /// Set fee setter address (fee_to_setter only)
        ///
        /// # Parameters
        /// * `new_setter` - New fee setter address
        #[ink(message)]
        pub fn set_fee_to_setter(&mut self, new_setter: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.fee_to_setter {
                return Err(Error::NotAuthorized);
            }

            if new_setter == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            let old_setter = self.fee_to_setter;
            self.fee_to_setter = new_setter;

            self.env().emit_event(FeeToSetterSet {
                old_setter,
                new_setter,
            });

            Ok(())
        }

        // ========================================================================
        // Internal Functions
        // ========================================================================

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

        /// Create pair contract instance
        ///
        /// NOTE: This is a simplified version. Production implementation would use:
        /// ```ignore
        /// use ink::env::call::{build_create, ExecutionInput, Selector};
        ///
        /// let pair = build_create::<Pair>()
        ///     .code_hash(self.pair_code_hash)
        ///     .gas_limit(0)
        ///     .endowment(0)
        ///     .exec_input(
        ///         ExecutionInput::new(Selector::new(ink::selector_bytes!("new")))
        ///             .push_arg(token0)
        ///             .push_arg(token1)
        ///     )
        ///     .returns::<AccountId>()
        ///     .instantiate();
        /// ```
        fn _create_pair_contract(&self, token0: AccountId, token1: AccountId) -> Result<AccountId> {
            // TODO: Implement actual contract instantiation
            // For now, generate a deterministic address from token addresses
            let mut data = Vec::new();
            data.extend_from_slice(token0.as_ref());
            data.extend_from_slice(token1.as_ref());

            // Generate deterministic address (simplified)
            let mut pair_bytes = [0u8; 32];
            pair_bytes[0..16].copy_from_slice(&data[0..16]);
            pair_bytes[16..32].copy_from_slice(&data[16..32]);

            Ok(AccountId::from(pair_bytes))
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        fn get_test_accounts() -> (AccountId, AccountId, AccountId) {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            (accounts.alice, accounts.bob, accounts.charlie)
        }

        #[ink::test]
        fn new_works() {
            let (setter, _, _) = get_test_accounts();
            let code_hash = Hash::from([0x42; 32]);

            let factory = Factory::new(setter, code_hash);

            assert_eq!(factory.fee_to_setter(), setter);
            assert_eq!(factory.fee_to(), None);
            assert_eq!(factory.all_pairs_length(), 0);
        }

        #[ink::test]
        fn create_pair_works() {
            let (setter, token_a, token_b) = get_test_accounts();
            let code_hash = Hash::from([0x42; 32]);

            let mut factory = Factory::new(setter, code_hash);

            // Create pair
            let pair = factory.create_pair(token_a, token_b).unwrap();

            // Verify pair created
            assert_eq!(factory.all_pairs_length(), 1);
            assert_eq!(factory.get_pair_address(token_a, token_b), Some(pair));
            assert_eq!(factory.get_pair_address(token_b, token_a), Some(pair)); // Both directions
            assert_eq!(factory.get_pair_by_index(0), Some(pair));
        }

        #[ink::test]
        fn create_pair_fails_identical_addresses() {
            let (setter, token_a, _) = get_test_accounts();
            let code_hash = Hash::from([0x42; 32]);

            let mut factory = Factory::new(setter, code_hash);

            assert_eq!(
                factory.create_pair(token_a, token_a),
                Err(Error::IdenticalAddresses)
            );
        }

        #[ink::test]
        fn create_pair_fails_zero_address() {
            let (setter, token_a, _) = get_test_accounts();
            let code_hash = Hash::from([0x42; 32]);
            let zero_address = AccountId::from([0u8; 32]);

            let mut factory = Factory::new(setter, code_hash);

            assert_eq!(
                factory.create_pair(token_a, zero_address),
                Err(Error::ZeroAddress)
            );
        }

        #[ink::test]
        fn create_pair_fails_if_exists() {
            let (setter, token_a, token_b) = get_test_accounts();
            let code_hash = Hash::from([0x42; 32]);

            let mut factory = Factory::new(setter, code_hash);

            // Create first pair
            factory.create_pair(token_a, token_b).unwrap();

            // Try to create again
            assert_eq!(
                factory.create_pair(token_a, token_b),
                Err(Error::PairExists)
            );

            // Try reversed order
            assert_eq!(
                factory.create_pair(token_b, token_a),
                Err(Error::PairExists)
            );
        }

        #[ink::test]
        fn set_fee_to_works() {
            let (setter, new_fee_to, _) = get_test_accounts();
            let code_hash = Hash::from([0x42; 32]);

            let mut factory = Factory::new(setter, code_hash);

            // Set fee_to
            factory.set_fee_to(Some(new_fee_to)).unwrap();
            assert_eq!(factory.fee_to(), Some(new_fee_to));

            // Clear fee_to
            factory.set_fee_to(None).unwrap();
            assert_eq!(factory.fee_to(), None);
        }

        #[ink::test]
        fn set_fee_to_fails_not_authorized() {
            let (setter, _, other) = get_test_accounts();
            let code_hash = Hash::from([0x42; 32]);

            let mut factory = Factory::new(setter, code_hash);

            // Change caller to non-setter
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(other);

            assert_eq!(factory.set_fee_to(Some(other)), Err(Error::NotAuthorized));
        }

        #[ink::test]
        fn set_fee_to_setter_works() {
            let (setter, new_setter, _) = get_test_accounts();
            let code_hash = Hash::from([0x42; 32]);

            let mut factory = Factory::new(setter, code_hash);

            factory.set_fee_to_setter(new_setter).unwrap();
            assert_eq!(factory.fee_to_setter(), new_setter);
        }
    }
}

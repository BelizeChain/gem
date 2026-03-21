#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
#[allow(clippy::too_many_arguments)]
pub mod router {
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::prelude::{vec, vec::Vec};
    use scale::{Decode, Encode};

    // ============================================================================
    // Storage
    // ============================================================================

    #[ink(storage)]
    pub struct Router {
        // Factory contract address
        factory: AccountId,

        // Wrapped native token address (WBZC)
        wbzc: AccountId,
    }

    // ============================================================================
    // Events
    // ============================================================================

    #[ink(event)]
    pub struct SwapExecuted {
        #[ink(topic)]
        sender: AccountId,
        path: Vec<AccountId>,
        amounts: Vec<Balance>,
    }

    #[ink(event)]
    pub struct LiquidityAdded {
        #[ink(topic)]
        provider: AccountId,
        token_a: AccountId,
        token_b: AccountId,
        amount_a: Balance,
        amount_b: Balance,
        liquidity: Balance,
    }

    #[ink(event)]
    pub struct LiquidityRemoved {
        #[ink(topic)]
        provider: AccountId,
        token_a: AccountId,
        token_b: AccountId,
        amount_a: Balance,
        amount_b: Balance,
    }

    // ============================================================================
    // Errors
    // ============================================================================

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Deadline has passed
        Expired,
        /// Insufficient output amount (slippage too high)
        InsufficientOutputAmount,
        /// Insufficient A amount for liquidity
        InsufficientAAmount,
        /// Insufficient B amount for liquidity
        InsufficientBAmount,
        /// Excessive input amount (slippage too high)
        ExcessiveInputAmount,
        /// Invalid path (must have at least 2 tokens)
        InvalidPath,
        /// Identical token addresses
        IdenticalAddresses,
        /// Zero address
        ZeroAddress,
        /// Zero amount
        ZeroAmount,
        /// Insufficient liquidity
        InsufficientLiquidity,
        /// Pair doesn't exist
        PairNotFound,
        /// Swap failed
        SwapFailed,
        /// Cross-contract call failed
        CallFailed,
        /// Arithmetic operation failed
        ArithmeticError,
        /// Caller is not authorized
        NotAuthorized,
        /// Path exceeds maximum allowed length
        PathTooLong,
        /// Path contains circular route (duplicate tokens)
        CircularPath,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    /// Maximum number of tokens in a swap path
    const MAX_PATH_LENGTH: usize = 4;

    // ============================================================================
    // Implementation
    // ============================================================================

    impl Router {
        // ========================================================================
        // Constructor
        // ========================================================================

        /// Create a new router
        ///
        /// # Parameters
        /// * `factory` - Factory contract address
        /// * `wbzc` - Wrapped BZC token address
        #[ink(constructor)]
        pub fn new(factory: AccountId, wbzc: AccountId) -> Self {
            Self { factory, wbzc }
        }

        // ========================================================================
        // View Functions
        // ========================================================================

        /// Get factory address
        #[ink(message)]
        pub fn factory(&self) -> AccountId {
            self.factory
        }

        /// Get WBZC address
        #[ink(message)]
        pub fn wbzc(&self) -> AccountId {
            self.wbzc
        }

        /// Quote: How much token B needed for exact token A?
        ///
        /// Maintains current price ratio.
        #[ink(message)]
        pub fn quote(
            &self,
            amount_a: Balance,
            reserve_a: Balance,
            reserve_b: Balance,
        ) -> Result<Balance> {
            if amount_a == 0 {
                return Err(Error::ZeroAmount);
            }
            if reserve_a == 0 || reserve_b == 0 {
                return Err(Error::InsufficientLiquidity);
            }

            let amount_b = amount_a
                .checked_mul(reserve_b)
                .ok_or(Error::ArithmeticError)?
                .checked_div(reserve_a)
                .ok_or(Error::ArithmeticError)?;

            Ok(amount_b)
        }

        /// Calculate output amount for exact input
        #[ink(message)]
        pub fn get_amount_out(
            &self,
            amount_in: Balance,
            reserve_in: Balance,
            reserve_out: Balance,
        ) -> Result<Balance> {
            if amount_in == 0 {
                return Err(Error::ZeroAmount);
            }
            if reserve_in == 0 || reserve_out == 0 {
                return Err(Error::InsufficientLiquidity);
            }

            // 0.3% fee: amount_in * 997 / 1000
            let amount_in_with_fee = amount_in.checked_mul(997).ok_or(Error::ArithmeticError)?;

            let denominator = reserve_in
                .checked_mul(1000)
                .ok_or(Error::ArithmeticError)?
                .checked_add(amount_in_with_fee)
                .ok_or(Error::ArithmeticError)?;

            Self::_checked_mul_div(amount_in_with_fee, reserve_out, denominator)
                .ok_or(Error::ArithmeticError)
        }

        /// Calculate input amount for exact output
        #[ink(message)]
        pub fn get_amount_in(
            &self,
            amount_out: Balance,
            reserve_in: Balance,
            reserve_out: Balance,
        ) -> Result<Balance> {
            if amount_out == 0 {
                return Err(Error::ZeroAmount);
            }
            if reserve_in == 0 || reserve_out == 0 {
                return Err(Error::InsufficientLiquidity);
            }
            if amount_out >= reserve_out {
                return Err(Error::InsufficientLiquidity);
            }

            let numerator_factor = reserve_in.checked_mul(1000).ok_or(Error::ArithmeticError)?;

            let denominator = reserve_out
                .checked_sub(amount_out)
                .ok_or(Error::ArithmeticError)?
                .checked_mul(997)
                .ok_or(Error::ArithmeticError)?;

            let result = Self::_checked_mul_div(numerator_factor, amount_out, denominator)
                .ok_or(Error::ArithmeticError)?;
            Ok(result + 1)
        }

        /// Calculate output amounts for multi-hop swap
        ///
        /// Example: path = [DALLA, BZC, USDT]
        /// Returns: [100 DALLA in, 200 BZC mid, 50 USDT out]
        #[ink(message)]
        pub fn get_amounts_out(
            &self,
            amount_in: Balance,
            path: Vec<AccountId>,
        ) -> Result<Vec<Balance>> {
            Self::_validate_path(&path)?;

            let mut amounts = Vec::new();
            amounts.push(amount_in);

            for i in 0..path.len() - 1 {
                let (reserve_in, reserve_out) = self._get_reserves(path[i], path[i + 1])?;
                let amount_out = self.get_amount_out(amounts[i], reserve_in, reserve_out)?;
                amounts.push(amount_out);
            }

            Ok(amounts)
        }

        /// Calculate input amounts for multi-hop swap
        #[ink(message)]
        pub fn get_amounts_in(
            &self,
            amount_out: Balance,
            path: Vec<AccountId>,
        ) -> Result<Vec<Balance>> {
            Self::_validate_path(&path)?;

            let mut amounts = vec![0; path.len()];
            amounts[path.len() - 1] = amount_out;

            for i in (1..path.len()).rev() {
                let (reserve_in, reserve_out) = self._get_reserves(path[i - 1], path[i])?;
                let amount_in = self.get_amount_in(amounts[i], reserve_in, reserve_out)?;
                amounts[i - 1] = amount_in;
            }

            Ok(amounts)
        }

        // ========================================================================
        // Liquidity Functions
        // ========================================================================

        /// Add liquidity to a pool
        ///
        /// # Parameters
        /// * `token_a` - First token address
        /// * `token_b` - Second token address
        /// * `amount_a_desired` - Desired amount of token A
        /// * `amount_b_desired` - Desired amount of token B
        /// * `amount_a_min` - Minimum amount of token A (slippage protection)
        /// * `amount_b_min` - Minimum amount of token B (slippage protection)
        /// * `to` - LP token recipient
        /// * `deadline` - Transaction must complete before this timestamp
        ///
        /// # Returns
        /// (amount_a, amount_b, liquidity)
        #[ink(message)]
        pub fn add_liquidity(
            &mut self,
            token_a: AccountId,
            token_b: AccountId,
            amount_a_desired: Balance,
            amount_b_desired: Balance,
            amount_a_min: Balance,
            amount_b_min: Balance,
            to: AccountId,
            deadline: u64,
        ) -> Result<(Balance, Balance, Balance)> {
            // Check deadline
            self._ensure_not_expired(deadline)?;

            // Calculate optimal amounts
            let (amount_a, amount_b) = self._calculate_liquidity_amounts(
                token_a,
                token_b,
                amount_a_desired,
                amount_b_desired,
                amount_a_min,
                amount_b_min,
            )?;

            // Get pair address
            let pair = self._get_pair(token_a, token_b)?;

            // Transfer tokens from caller to pair
            self._token_transfer_from(token_a, self.env().caller(), pair, amount_a)?;
            self._token_transfer_from(token_b, self.env().caller(), pair, amount_b)?;

            // Call pair.mint(to) to receive LP tokens
            let liquidity = self._pair_mint(pair, to)?;

            // Emit event
            self.env().emit_event(LiquidityAdded {
                provider: self.env().caller(),
                token_a,
                token_b,
                amount_a,
                amount_b,
                liquidity,
            });

            Ok((amount_a, amount_b, liquidity))
        }

        /// Remove liquidity from a pool
        ///
        /// # Parameters
        /// * `token_a` - First token address
        /// * `token_b` - Second token address
        /// * `liquidity` - Amount of LP tokens to burn
        /// * `amount_a_min` - Minimum amount of token A to receive
        /// * `amount_b_min` - Minimum amount of token B to receive
        /// * `to` - Token recipient
        /// * `deadline` - Transaction must complete before this timestamp
        ///
        /// # Returns
        /// (amount_a, amount_b)
        #[ink(message)]
        pub fn remove_liquidity(
            &mut self,
            token_a: AccountId,
            token_b: AccountId,
            liquidity: Balance,
            amount_a_min: Balance,
            amount_b_min: Balance,
            to: AccountId,
            deadline: u64,
        ) -> Result<(Balance, Balance)> {
            // Check deadline
            self._ensure_not_expired(deadline)?;

            // Get pair address
            let pair = self._get_pair(token_a, token_b)?;

            // Transfer LP tokens from caller to pair
            self._token_transfer_from(pair, self.env().caller(), pair, liquidity)?;

            // Call pair.burn(to) to receive underlying tokens
            let (amount0, amount1) = self._pair_burn(pair, to)?;

            // Sort amounts based on token order
            let (token0, _) = Self::_sort_tokens(token_a, token_b)?;
            let (amount_a, amount_b) = if token_a == token0 {
                (amount0, amount1)
            } else {
                (amount1, amount0)
            };

            // Check slippage
            if amount_a < amount_a_min {
                return Err(Error::InsufficientAAmount);
            }
            if amount_b < amount_b_min {
                return Err(Error::InsufficientBAmount);
            }

            // Emit event
            self.env().emit_event(LiquidityRemoved {
                provider: self.env().caller(),
                token_a,
                token_b,
                amount_a,
                amount_b,
            });

            Ok((amount_a, amount_b))
        }

        // ========================================================================
        // Swap Functions
        // ========================================================================

        /// Swap exact tokens for tokens
        ///
        /// # Example
        /// Swap exactly 100 DALLA for at least 190 BZC
        ///
        /// # Parameters
        /// * `amount_in` - Exact amount of input tokens
        /// * `amount_out_min` - Minimum amount of output tokens (slippage protection)
        /// * `path` - Token swap path [DALLA, BZC] or [DALLA, BZC, USDT]
        /// * `to` - Output token recipient
        /// * `deadline` - Transaction must complete before this timestamp
        #[ink(message)]
        pub fn swap_exact_tokens_for_tokens(
            &mut self,
            amount_in: Balance,
            amount_out_min: Balance,
            path: Vec<AccountId>,
            to: AccountId,
            deadline: u64,
        ) -> Result<Vec<Balance>> {
            // Check deadline
            self._ensure_not_expired(deadline)?;

            // Calculate amounts for each hop
            let amounts = self.get_amounts_out(amount_in, path.clone())?;

            // Check slippage
            if amounts[amounts.len() - 1] < amount_out_min {
                return Err(Error::InsufficientOutputAmount);
            }

            // Transfer input tokens from caller to first pair
            let first_pair = self._get_pair(path[0], path[1])?;
            self._token_transfer_from(path[0], self.env().caller(), first_pair, amounts[0])?;

            // Execute swaps
            self._swap(&amounts, &path, to)?;

            // Emit event
            self.env().emit_event(SwapExecuted {
                sender: self.env().caller(),
                path,
                amounts: amounts.clone(),
            });

            Ok(amounts)
        }

        /// Swap tokens for exact tokens
        ///
        /// # Example
        /// Swap at most 105 DALLA for exactly 200 BZC
        ///
        /// # Parameters
        /// * `amount_out` - Exact amount of output tokens desired
        /// * `amount_in_max` - Maximum amount of input tokens (slippage protection)
        /// * `path` - Token swap path
        /// * `to` - Output token recipient
        /// * `deadline` - Transaction must complete before this timestamp
        #[ink(message)]
        pub fn swap_tokens_for_exact_tokens(
            &mut self,
            amount_out: Balance,
            amount_in_max: Balance,
            path: Vec<AccountId>,
            to: AccountId,
            deadline: u64,
        ) -> Result<Vec<Balance>> {
            // Check deadline
            self._ensure_not_expired(deadline)?;

            // Calculate amounts for each hop
            let amounts = self.get_amounts_in(amount_out, path.clone())?;

            // Check slippage
            if amounts[0] > amount_in_max {
                return Err(Error::ExcessiveInputAmount);
            }

            // Transfer input tokens from caller to first pair
            let first_pair = self._get_pair(path[0], path[1])?;
            self._token_transfer_from(path[0], self.env().caller(), first_pair, amounts[0])?;

            // Execute swaps
            self._swap(&amounts, &path, to)?;

            // Emit event
            self.env().emit_event(SwapExecuted {
                sender: self.env().caller(),
                path,
                amounts: amounts.clone(),
            });

            Ok(amounts)
        }

        /// Upgrades the contract code hash (factory deployer only)
        ///
        /// Allows the router contract to be upgraded to a new implementation
        /// while preserving storage state.
        #[ink(message)]
        pub fn set_code_hash(&mut self, new_code_hash: Hash) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.factory {
                return Err(Error::NotAuthorized);
            }

            ink::env::set_code_hash::<Environment>(&new_code_hash)
                .map_err(|_| Error::NotAuthorized)?;
            Ok(())
        }

        // ========================================================================
        // Internal Functions
        // ========================================================================

        /// Transfer tokens via PSP22 cross-contract call (transfer_from)
        ///
        /// Calls the `transfer_from` method on a PSP22 token contract
        fn _token_transfer_from(
            &self,
            token: AccountId,
            from: AccountId,
            to: AccountId,
            amount: Balance,
        ) -> Result<()> {
            // PSP22::transfer_from selector is 0x54b3c76e
            let selector = [0x54, 0xb3, 0xc7, 0x6e];

            let data: ink::prelude::vec::Vec<u8> = ink::prelude::vec::Vec::new();
            let result = build_call::<Environment>()
                .call(token)
                .exec_input(
                    ExecutionInput::new(Selector::new(selector))
                        .push_arg(from)
                        .push_arg(to)
                        .push_arg(amount)
                        .push_arg(data),
                )
                .returns::<core::result::Result<(), ink::prelude::vec::Vec<u8>>>()
                .try_invoke();

            match result {
                Ok(Ok(_)) => Ok(()),
                _ => Err(Error::CallFailed),
            }
        }

        /// Ensure transaction hasn't expired
        fn _ensure_not_expired(&self, deadline: u64) -> Result<()> {
            let now = self.env().block_timestamp();
            if now > deadline {
                return Err(Error::Expired);
            }
            Ok(())
        }

        /// Sort token addresses
        fn _sort_tokens(token_a: AccountId, token_b: AccountId) -> Result<(AccountId, AccountId)> {
            if token_a == token_b {
                return Err(Error::IdenticalAddresses);
            }

            let zero_address = AccountId::from([0u8; 32]);
            if token_a == zero_address || token_b == zero_address {
                return Err(Error::ZeroAddress);
            }

            if token_a < token_b {
                Ok((token_a, token_b))
            } else {
                Ok((token_b, token_a))
            }
        }

        /// Get pair address for two tokens
        ///
        /// Calls factory.get_pair(tokenA, tokenB) to retrieve the pair address.
        fn _get_pair(&self, token_a: AccountId, token_b: AccountId) -> Result<AccountId> {
            let selector = [0xe7, 0xac, 0xcb, 0x3e]; // get_pair_address method
            let result = build_call::<Environment>()
                .call(self.factory)
                .exec_input(
                    ExecutionInput::new(Selector::new(selector))
                        .push_arg(token_a)
                        .push_arg(token_b),
                )
                .returns::<Option<AccountId>>()
                .try_invoke();

            match result {
                Ok(Ok(Some(pair))) => Ok(pair),
                _ => Err(Error::PairNotFound),
            }
        }

        /// Get reserves for two tokens via cross-contract call to pair
        fn _get_reserves(
            &self,
            token_a: AccountId,
            token_b: AccountId,
        ) -> Result<(Balance, Balance)> {
            let (token0, _) = Self::_sort_tokens(token_a, token_b)?;
            let pair = self._get_pair(token_a, token_b)?;

            // Call pair.get_reserves() — returns (reserve0, reserve1, block_timestamp_last)
            let selector = [0x8a, 0x0d, 0x11, 0x6f]; // get_reserves
            let result = build_call::<Environment>()
                .call(pair)
                .exec_input(ExecutionInput::new(Selector::new(selector)))
                .returns::<(Balance, Balance, u64)>()
                .try_invoke();

            let (reserve0, reserve1) = match result {
                Ok(Ok((r0, r1, _timestamp))) => (r0, r1),
                _ => return Err(Error::CallFailed),
            };

            if token_a == token0 {
                Ok((reserve0, reserve1))
            } else {
                Ok((reserve1, reserve0))
            }
        }

        /// Calculate optimal liquidity amounts
        fn _calculate_liquidity_amounts(
            &self,
            token_a: AccountId,
            token_b: AccountId,
            amount_a_desired: Balance,
            amount_b_desired: Balance,
            amount_a_min: Balance,
            amount_b_min: Balance,
        ) -> Result<(Balance, Balance)> {
            // Get reserves
            let (reserve_a, reserve_b) = match self._get_reserves(token_a, token_b) {
                Ok(reserves) => reserves,
                Err(_) => {
                    // Pair doesn't exist, use desired amounts
                    return Ok((amount_a_desired, amount_b_desired));
                }
            };

            if reserve_a == 0 && reserve_b == 0 {
                // First liquidity provision
                Ok((amount_a_desired, amount_b_desired))
            } else {
                // Calculate optimal amount B
                let amount_b_optimal = self.quote(amount_a_desired, reserve_a, reserve_b)?;

                if amount_b_optimal <= amount_b_desired {
                    // Use optimal B amount
                    if amount_b_optimal < amount_b_min {
                        return Err(Error::InsufficientBAmount);
                    }
                    Ok((amount_a_desired, amount_b_optimal))
                } else {
                    // Calculate optimal amount A
                    let amount_a_optimal = self.quote(amount_b_desired, reserve_b, reserve_a)?;
                    if amount_a_optimal > amount_a_desired {
                        return Err(Error::InsufficientAAmount);
                    }
                    if amount_a_optimal < amount_a_min {
                        return Err(Error::InsufficientAAmount);
                    }
                    Ok((amount_a_optimal, amount_b_desired))
                }
            }
        }

        /// Execute multi-hop swap
        ///
        /// Swaps tokens through multiple pairs in sequence.
        /// Tokens are transferred directly between pairs for efficiency.
        fn _swap(&self, amounts: &[Balance], path: &[AccountId], to: AccountId) -> Result<()> {
            for i in 0..path.len() - 1 {
                let (input, output) = (path[i], path[i + 1]);

                // Get pair address
                let pair = self._get_pair(input, output)?;

                // Determine token order in pair (pairs use sorted addresses)
                let (token0, _token1) = if input < output {
                    (input, output)
                } else {
                    (output, input)
                };

                // Calculate output amounts based on token order
                let amount_out = amounts[i + 1];
                let (amount0_out, amount1_out) = if input == token0 {
                    (0, amount_out)
                } else {
                    (amount_out, 0)
                };

                // Determine recipient: next pair or final destination
                let recipient = if i < path.len() - 2 {
                    self._get_pair(output, path[i + 2])?
                } else {
                    to
                };

                // Call pair.swap(amount0Out, amount1Out, to)
                let selector = [0x11, 0x00, 0x4f, 0xa6]; // swap method
                let result = build_call::<Environment>()
                    .call(pair)
                    .exec_input(
                        ExecutionInput::new(Selector::new(selector))
                            .push_arg(amount0_out)
                            .push_arg(amount1_out)
                            .push_arg(recipient),
                    )
                    .returns::<core::result::Result<(), Vec<u8>>>()
                    .try_invoke();

                match result {
                    Ok(Ok(_)) => {}
                    _ => return Err(Error::SwapFailed),
                }
            }

            Ok(())
        }

        /// Validate swap path: length bounds and no circular routes
        fn _validate_path(path: &[AccountId]) -> Result<()> {
            if path.len() < 2 {
                return Err(Error::InvalidPath);
            }
            if path.len() > MAX_PATH_LENGTH {
                return Err(Error::PathTooLong);
            }
            for i in 0..path.len() {
                for j in (i + 1)..path.len() {
                    if path[i] == path[j] {
                        return Err(Error::CircularPath);
                    }
                }
            }
            Ok(())
        }

        /// Call pair.mint(to) to mint LP tokens
        fn _pair_mint(&self, pair: AccountId, to: AccountId) -> Result<Balance> {
            let selector = [0xcf, 0xdd, 0x9a, 0xa2]; // mint
            let result = build_call::<Environment>()
                .call(pair)
                .exec_input(ExecutionInput::new(Selector::new(selector)).push_arg(to))
                .returns::<core::result::Result<Balance, Vec<u8>>>()
                .try_invoke();

            match result {
                Ok(Ok(Ok(liquidity))) => Ok(liquidity),
                _ => Err(Error::CallFailed),
            }
        }

        /// Call pair.burn(to) to burn LP tokens and receive underlying tokens
        fn _pair_burn(&self, pair: AccountId, to: AccountId) -> Result<(Balance, Balance)> {
            let selector = [0xb1, 0xef, 0xc1, 0x7b]; // burn
            let result = build_call::<Environment>()
                .call(pair)
                .exec_input(ExecutionInput::new(Selector::new(selector)).push_arg(to))
                .returns::<core::result::Result<(Balance, Balance), Vec<u8>>>()
                .try_invoke();

            match result {
                Ok(Ok(Ok(amounts))) => Ok(amounts),
                _ => Err(Error::CallFailed),
            }
        }

        /// Compute (a * b) / c safely using GCD reduction to avoid u128 overflow.
        fn _checked_mul_div(a: u128, b: u128, c: u128) -> Option<u128> {
            if c == 0 {
                return None;
            }
            if let Some(product) = a.checked_mul(b) {
                return Some(product / c);
            }
            // Reduce by GCD to minimize values before multiplication
            let g1 = Self::_gcd(a, c);
            let a_reduced = a / g1;
            let c_reduced = c / g1;
            let g2 = Self::_gcd(b, c_reduced);
            let b_reduced = b / g2;
            let c_final = c_reduced / g2;
            a_reduced.checked_mul(b_reduced).map(|p| p / c_final)
        }

        /// Greatest common divisor (Euclidean algorithm)
        fn _gcd(mut a: u128, mut b: u128) -> u128 {
            while b != 0 {
                let t = b;
                b = a % b;
                a = t;
            }
            a
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        fn get_test_accounts() -> (AccountId, AccountId) {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            (accounts.alice, accounts.bob)
        }

        #[ink::test]
        fn new_works() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);

            assert_eq!(router.factory(), factory);
            assert_eq!(router.wbzc(), wbzc);
        }

        #[ink::test]
        fn quote_works() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);

            // Reserve: 1000 A, 2000 B
            // Quote 500 A → ? B
            let amount_b = router.quote(500, 1000, 2000).unwrap();
            assert_eq!(amount_b, 1000); // 500 * 2000 / 1000 = 1000
        }

        #[ink::test]
        fn get_amount_out_works() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);

            // Reserve: 1000 A, 2000 B
            // Swap 100 A → ? B
            let amount_out = router.get_amount_out(100, 1000, 2000).unwrap();

            // Formula: (100 * 997 * 2000) / (1000 * 1000 + 100 * 997)
            //        = 199400000 / 1099700 = 181.35...
            assert!(amount_out > 180 && amount_out < 182);
        }

        #[ink::test]
        fn get_amount_in_works() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);

            // Reserve: 1000 A, 2000 B
            // Want exactly 181 B → ? A
            let amount_in = router.get_amount_in(181, 1000, 2000).unwrap();

            // Should be around 100
            assert!(amount_in > 99 && amount_in < 101);
        }

        // ====================================================================
        // Error condition tests
        // ====================================================================

        #[ink::test]
        fn quote_zero_amount_fails() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);
            assert_eq!(router.quote(0, 1000, 2000), Err(Error::ZeroAmount));
        }

        #[ink::test]
        fn quote_zero_reserves_fails() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);
            assert_eq!(
                router.quote(100, 0, 2000),
                Err(Error::InsufficientLiquidity)
            );
            assert_eq!(
                router.quote(100, 1000, 0),
                Err(Error::InsufficientLiquidity)
            );
        }

        #[ink::test]
        fn get_amount_out_zero_amount_fails() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);
            assert_eq!(router.get_amount_out(0, 1000, 2000), Err(Error::ZeroAmount));
        }

        #[ink::test]
        fn get_amount_out_zero_reserves_fails() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);
            assert_eq!(
                router.get_amount_out(100, 0, 2000),
                Err(Error::InsufficientLiquidity)
            );
            assert_eq!(
                router.get_amount_out(100, 1000, 0),
                Err(Error::InsufficientLiquidity)
            );
        }

        #[ink::test]
        fn get_amount_in_zero_amount_fails() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);
            assert_eq!(router.get_amount_in(0, 1000, 2000), Err(Error::ZeroAmount));
        }

        #[ink::test]
        fn get_amount_in_zero_reserves_fails() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);
            assert_eq!(
                router.get_amount_in(100, 0, 2000),
                Err(Error::InsufficientLiquidity)
            );
            assert_eq!(
                router.get_amount_in(100, 1000, 0),
                Err(Error::InsufficientLiquidity)
            );
        }

        #[ink::test]
        fn get_amount_in_amount_exceeds_reserve_fails() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);
            assert_eq!(
                router.get_amount_in(2000, 1000, 2000),
                Err(Error::InsufficientLiquidity)
            );
            assert_eq!(
                router.get_amount_in(2001, 1000, 2000),
                Err(Error::InsufficientLiquidity)
            );
        }

        // ====================================================================
        // Sort tokens tests
        // ====================================================================

        #[ink::test]
        fn sort_tokens_identical_fails() {
            let (factory, _) = get_test_accounts();
            assert_eq!(
                Router::_sort_tokens(factory, factory),
                Err(Error::IdenticalAddresses)
            );
        }

        #[ink::test]
        fn sort_tokens_zero_address_fails() {
            let (factory, _) = get_test_accounts();
            let zero = AccountId::from([0u8; 32]);
            assert_eq!(Router::_sort_tokens(zero, factory), Err(Error::ZeroAddress));
            assert_eq!(Router::_sort_tokens(factory, zero), Err(Error::ZeroAddress));
        }

        #[ink::test]
        fn sort_tokens_returns_ordered() {
            let a = AccountId::from([1u8; 32]);
            let b = AccountId::from([2u8; 32]);
            let (t0, t1) = Router::_sort_tokens(a, b).unwrap();
            assert!(t0 <= t1);
            let (t0_r, t1_r) = Router::_sort_tokens(b, a).unwrap();
            assert_eq!(t0, t0_r);
            assert_eq!(t1, t1_r);
        }

        // ====================================================================
        // Path validation tests
        // ====================================================================

        #[ink::test]
        fn validate_path_too_short_fails() {
            let a = AccountId::from([1u8; 32]);
            assert_eq!(Router::_validate_path(&[a]), Err(Error::InvalidPath));
            assert_eq!(Router::_validate_path(&[]), Err(Error::InvalidPath));
        }

        #[ink::test]
        fn validate_path_too_long_fails() {
            let tokens: Vec<AccountId> = (1..=5).map(|i| AccountId::from([i; 32])).collect();
            assert_eq!(Router::_validate_path(&tokens), Err(Error::PathTooLong));
        }

        #[ink::test]
        fn validate_path_circular_fails() {
            let a = AccountId::from([1u8; 32]);
            let b = AccountId::from([2u8; 32]);
            assert_eq!(Router::_validate_path(&[a, b, a]), Err(Error::CircularPath));
        }

        #[ink::test]
        fn validate_path_valid() {
            let a = AccountId::from([1u8; 32]);
            let b = AccountId::from([2u8; 32]);
            let c = AccountId::from([3u8; 32]);
            assert!(Router::_validate_path(&[a, b]).is_ok());
            assert!(Router::_validate_path(&[a, b, c]).is_ok());
        }

        // ====================================================================
        // Math helper tests
        // ====================================================================

        #[ink::test]
        fn checked_mul_div_basic() {
            // Simple case: (10 * 20) / 5 = 40
            assert_eq!(Router::_checked_mul_div(10, 20, 5), Some(40));
        }

        #[ink::test]
        fn checked_mul_div_zero_denominator() {
            assert_eq!(Router::_checked_mul_div(10, 20, 0), None);
        }

        #[ink::test]
        fn checked_mul_div_large_values() {
            // Values that would overflow u128 if multiplied directly
            let a: u128 = 1_000_000_000_000_000_000; // 10^18
            let b: u128 = 2_000_000_000_000_000_000; // 2 * 10^18
            let c: u128 = 500_000_000_000_000_000; // 5 * 10^17
                                                   // (10^18 * 2*10^18) / (5*10^17) = 4*10^18
            let result = Router::_checked_mul_div(a, b, c);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), 4_000_000_000_000_000_000);
        }

        #[ink::test]
        fn gcd_basic() {
            assert_eq!(Router::_gcd(12, 8), 4);
            assert_eq!(Router::_gcd(100, 75), 25);
            assert_eq!(Router::_gcd(17, 13), 1);
            assert_eq!(Router::_gcd(0, 5), 5);
            assert_eq!(Router::_gcd(5, 0), 5);
        }

        // ====================================================================
        // Deadline test
        // ====================================================================

        #[ink::test]
        fn ensure_not_expired_works() {
            let (factory, wbzc) = get_test_accounts();
            let router = Router::new(factory, wbzc);
            // Current block timestamp is 0 in test env, so deadline=0 means not expired (0 > 0 is false)
            assert!(router._ensure_not_expired(0).is_ok());
            assert!(router._ensure_not_expired(1).is_ok());
        }
    }
}

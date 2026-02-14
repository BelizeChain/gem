#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod router {
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::prelude::vec::Vec;
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
    }

    pub type Result<T> = core::result::Result<T, Error>;

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

            let numerator = amount_in_with_fee
                .checked_mul(reserve_out)
                .ok_or(Error::ArithmeticError)?;

            let denominator = reserve_in
                .checked_mul(1000)
                .ok_or(Error::ArithmeticError)?
                .checked_add(amount_in_with_fee)
                .ok_or(Error::ArithmeticError)?;

            Ok(numerator / denominator)
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

            let numerator = reserve_in
                .checked_mul(amount_out)
                .ok_or(Error::ArithmeticError)?
                .checked_mul(1000)
                .ok_or(Error::ArithmeticError)?;

            let denominator = reserve_out
                .checked_sub(amount_out)
                .ok_or(Error::ArithmeticError)?
                .checked_mul(997)
                .ok_or(Error::ArithmeticError)?;

            Ok(numerator / denominator + 1)
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
            if path.len() < 2 {
                return Err(Error::InvalidPath);
            }

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
            if path.len() < 2 {
                return Err(Error::InvalidPath);
            }

            let mut amounts = Vec::new();
            amounts.resize(path.len(), 0);
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

            // Call pair.mint(to) - placeholder for cross-contract call
            let liquidity = 0; // TODO: Implement pair.mint() cross-contract call

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
            // Note: LP tokens are managed by the pair contract itself
            // In production,  would transfer pair LP tokens here

            // Call pair.burn(to) - placeholder for cross-contract call
            let (amount0, amount1) = (0, 0); // TODO: Implement pair.burn() cross-contract call

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

            let result = build_call::<Environment>()
                .call(token)
                .exec_input(
                    ExecutionInput::new(Selector::new(selector))
                        .push_arg(from)
                        .push_arg(to)
                        .push_arg(amount),
                )
                .returns::<core::result::Result<(), ink::prelude::vec::Vec<u8>>>()
                .try_invoke();

            match result {
                Ok(Ok(_)) => Ok(()),
                _ => Err(Error::CallFailed),
            }
        }

        /// Get token balance via PSP22 cross-contract call
        fn _token_balance_of(&self, token: AccountId, account: AccountId) -> Balance {
            // PSP22::balance_of selector is 0x65682523
            let selector = [0x65, 0x68, 0x25, 0x23];

            let result = build_call::<Environment>()
                .call(token)
                .exec_input(ExecutionInput::new(Selector::new(selector)).push_arg(account))
                .returns::<Balance>()
                .try_invoke();

            match result {
                Ok(Ok(balance)) => balance,
                _ => 0,
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
            let selector = [0x6a, 0x3d, 0x0f, 0x5f]; // get_pair method
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

        /// Get reserves for two tokens
        fn _get_reserves(
            &self,
            token_a: AccountId,
            token_b: AccountId,
        ) -> Result<(Balance, Balance)> {
            let (token0, _) = Self::_sort_tokens(token_a, token_b)?;

            // TODO: Call pair.get_reserves()
            // For now, return placeholder (1000 DALLA, 2000 BZC)
            let (reserve0, reserve1) = (1000, 2000);

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
                let selector = [0x1e, 0x6a, 0xf2, 0x6f]; // swap method
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
    }
}

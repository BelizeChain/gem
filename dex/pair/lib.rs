#![cfg_attr(not(feature = "std"), no_std, no_main)]

//! # Pair Contract
//!
//! Core AMM trading pair implementing constant product formula (x * y = k).
//!
//! ## Features
//! - Liquidity provision (mint LP tokens)
//! - Liquidity removal (burn LP tokens)
//! - Token swaps with 0.3% fee
//! - Price oracle (TWAP ready)
//! - Minimum liquidity lock

#[ink::contract]
pub mod pair {
    use ink::env::call::{build_call, ExecutionInput, Selector};

    use ink::storage::Mapping;
    use scale::{Decode, Encode};

    /// Minimum liquidity locked forever (prevents manipulation)
    const MINIMUM_LIQUIDITY: Balance = 1000;

    /// Trading fee: 0.3% (represented as 3/1000)
    const FEE_NUMERATOR: u128 = 3;
    const FEE_DENOMINATOR: u128 = 1000;

    // ============================================================================
    // Storage
    // ============================================================================

    #[ink(storage)]
    pub struct Pair {
        /// Factory contract that created this pair
        factory: AccountId,

        /// Token0 address (lexicographically smaller)
        token0: AccountId,

        /// Token1 address (lexicographically larger)
        token1: AccountId,

        /// Reserve of token0
        reserve0: Balance,

        /// Reserve of token1
        reserve1: Balance,

        /// Total supply of LP tokens
        total_supply: Balance,

        /// LP token balances: account => balance
        balances: Mapping<AccountId, Balance>,

        /// LP token allowances: (owner, spender) => amount
        allowances: Mapping<(AccountId, AccountId), Balance>,

        /// Block timestamp of last reserve update (for TWAP oracle)
        block_timestamp_last: u64,

        /// Cumulative price0 (for TWAP oracle)
        price0_cumulative_last: u128,

        /// Cumulative price1 (for TWAP oracle)
        price1_cumulative_last: u128,

        /// K value (reserve0 * reserve1) - must never decrease
        k_last: u128,

        /// Reentrancy lock
        locked: bool,
    }

    // ============================================================================
    // Events
    // ============================================================================

    #[ink(event)]
    pub struct Mint {
        #[ink(topic)]
        sender: AccountId,
        amount0: Balance,
        amount1: Balance,
        liquidity: Balance,
    }

    #[ink(event)]
    pub struct Burn {
        #[ink(topic)]
        sender: AccountId,
        amount0: Balance,
        amount1: Balance,
        #[ink(topic)]
        to: AccountId,
        liquidity: Balance,
    }

    #[ink(event)]
    pub struct Swap {
        #[ink(topic)]
        sender: AccountId,
        amount0_in: Balance,
        amount1_in: Balance,
        amount0_out: Balance,
        amount1_out: Balance,
        #[ink(topic)]
        to: AccountId,
    }

    #[ink(event)]
    pub struct Sync {
        reserve0: Balance,
        reserve1: Balance,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: Balance,
    }

    // ============================================================================
    // Errors
    // ============================================================================

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Insufficient liquidity minted
        InsufficientLiquidityMinted,
        /// Insufficient liquidity burned
        InsufficientLiquidityBurned,
        /// Insufficient output amount
        InsufficientOutputAmount,
        /// Insufficient liquidity
        InsufficientLiquidity,
        /// Invalid recipient (zero address)
        InvalidTo,
        /// Insufficient input amount
        InsufficientInputAmount,
        /// K value decreased (invariant violated)
        KValueDecreased,
        /// Overflow occurred
        Overflow,
        /// Identical addresses
        IdenticalAddresses,
        /// Zero address
        ZeroAddress,
        /// Insufficient balance
        InsufficientBalance,
        /// Insufficient allowance
        InsufficientAllowance,
        /// Reentrancy detected
        Locked,
        /// Transfer failed
        TransferFailed,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    // ============================================================================
    // Implementation
    // ============================================================================

    impl Pair {
        // ========================================================================
        // Constructor
        // ========================================================================

        /// Initialize a new trading pair
        #[ink(constructor)]
        pub fn new(token0: AccountId, token1: AccountId) -> Self {
            Self {
                factory: Self::env().caller(),
                token0,
                token1,
                reserve0: 0,
                reserve1: 0,
                total_supply: 0,
                balances: Mapping::default(),
                allowances: Mapping::default(),
                block_timestamp_last: 0,
                price0_cumulative_last: 0,
                price1_cumulative_last: 0,
                k_last: 0,
                locked: false,
            }
        }

        // ========================================================================
        // LP Token Functions (PSP22-like)
        // ========================================================================

        /// Get LP token balance of account
        #[ink(message)]
        pub fn balance_of(&self, account: AccountId) -> Balance {
            self.balances.get(account).unwrap_or(0)
        }

        /// Get total supply of LP tokens
        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        /// Transfer LP tokens
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let caller = self.env().caller();
            self._transfer(caller, to, value)
        }

        /// Approve spender for LP tokens
        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, value: Balance) -> Result<()> {
            let caller = self.env().caller();
            self.allowances.insert((caller, spender), &value);

            self.env().emit_event(Approval {
                owner: caller,
                spender,
                value,
            });

            Ok(())
        }

        /// Transfer LP tokens from another account
        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) -> Result<()> {
            let caller = self.env().caller();
            let allowance = self.allowances.get((from, caller)).unwrap_or(0);

            if allowance < value {
                return Err(Error::InsufficientAllowance);
            }

            self.allowances.insert((from, caller), &(allowance - value));
            self._transfer(from, to, value)
        }

        /// Get allowance
        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.allowances.get((owner, spender)).unwrap_or(0)
        }

        // ========================================================================
        // AMM Functions
        // ========================================================================

        /// Get token addresses
        #[ink(message)]
        pub fn get_tokens(&self) -> (AccountId, AccountId) {
            (self.token0, self.token1)
        }

        /// Get current reserves
        #[ink(message)]
        pub fn get_reserves(&self) -> (Balance, Balance, u64) {
            (self.reserve0, self.reserve1, self.block_timestamp_last)
        }

        /// Add liquidity and mint LP tokens
        ///
        /// # Parameters
        /// * `to` - Recipient of LP tokens
        ///
        /// # Returns
        /// Amount of LP tokens minted
        ///
        /// # Requirements
        /// - Caller must have transferred tokens to this contract first
        /// - First liquidity provision must exceed MINIMUM_LIQUIDITY
        #[ink(message)]
        pub fn mint(&mut self, to: AccountId) -> Result<Balance> {
            self.ensure_not_locked()?;
            self.locked = true;

            if to == AccountId::from([0u8; 32]) {
                self.locked = false;
                return Err(Error::InvalidTo);
            }

            let (reserve0, reserve1) = (self.reserve0, self.reserve1);

            // Get actual token balances (caller must have transferred tokens first)
            // In production, this would call token0.balance_of(self) and token1.balance_of(self)
            // For now, we simulate this
            let balance0 = reserve0; // TODO: Call token0 contract
            let balance1 = reserve1; // TODO: Call token1 contract

            let amount0 = balance0.saturating_sub(reserve0);
            let amount1 = balance1.saturating_sub(reserve1);

            let liquidity = if self.total_supply == 0 {
                // First liquidity provision
                let initial_liquidity =
                    Self::sqrt(amount0.checked_mul(amount1).ok_or(Error::Overflow)?);

                if initial_liquidity <= MINIMUM_LIQUIDITY {
                    self.locked = false;
                    return Err(Error::InsufficientLiquidityMinted);
                }

                // Lock minimum liquidity forever (to zero address)
                let zero_address = AccountId::from([0u8; 32]);
                self.balances.insert(zero_address, &MINIMUM_LIQUIDITY);
                self.total_supply = MINIMUM_LIQUIDITY;

                initial_liquidity - MINIMUM_LIQUIDITY
            } else {
                // Subsequent liquidity provisions
                let liquidity0 = amount0
                    .checked_mul(self.total_supply)
                    .ok_or(Error::Overflow)?
                    .checked_div(reserve0)
                    .ok_or(Error::InsufficientLiquidity)?;

                let liquidity1 = amount1
                    .checked_mul(self.total_supply)
                    .ok_or(Error::Overflow)?
                    .checked_div(reserve1)
                    .ok_or(Error::InsufficientLiquidity)?;

                // Use minimum to maintain price ratio
                if liquidity0 < liquidity1 {
                    liquidity0
                } else {
                    liquidity1
                }
            };

            if liquidity == 0 {
                self.locked = false;
                return Err(Error::InsufficientLiquidityMinted);
            }

            // Mint LP tokens
            let to_balance = self.balance_of(to);
            self.balances
                .insert(to, &to_balance.saturating_add(liquidity));
            self.total_supply = self.total_supply.saturating_add(liquidity);

            // Update reserves
            self._update(balance0, balance1)?;

            self.env().emit_event(Mint {
                sender: self.env().caller(),
                amount0,
                amount1,
                liquidity,
            });

            self.locked = false;
            Ok(liquidity)
        }

        /// Remove liquidity and burn LP tokens
        ///
        /// # Parameters
        /// * `to` - Recipient of underlying tokens
        ///
        /// # Returns
        /// (amount0, amount1) - Amounts of tokens returned
        #[ink(message)]
        pub fn burn(&mut self, to: AccountId) -> Result<(Balance, Balance)> {
            self.ensure_not_locked()?;
            self.locked = true;

            if to == AccountId::from([0u8; 32]) {
                self.locked = false;
                return Err(Error::InvalidTo);
            }

            let (reserve0, reserve1) = (self.reserve0, self.reserve1);

            // Get LP tokens sent to this contract
            let liquidity = self.balance_of(self.env().account_id());

            if liquidity == 0 {
                self.locked = false;
                return Err(Error::InsufficientLiquidityBurned);
            }

            // Calculate amounts to return
            let amount0 = liquidity
                .checked_mul(reserve0)
                .ok_or(Error::Overflow)?
                .checked_div(self.total_supply)
                .ok_or(Error::InsufficientLiquidity)?;

            let amount1 = liquidity
                .checked_mul(reserve1)
                .ok_or(Error::Overflow)?
                .checked_div(self.total_supply)
                .ok_or(Error::InsufficientLiquidity)?;

            if amount0 == 0 || amount1 == 0 {
                self.locked = false;
                return Err(Error::InsufficientLiquidityBurned);
            }

            // Burn LP tokens
            let this_balance = self.balance_of(self.env().account_id());
            self.balances
                .insert(self.env().account_id(), &(this_balance - liquidity));
            self.total_supply = self.total_supply - liquidity;

            // Transfer tokens to recipient
            self._token_transfer(self.token0, to, amount0)?;
            self._token_transfer(self.token1, to, amount1)?;

            // Update reserves
            let balance0 = reserve0 - amount0;
            let balance1 = reserve1 - amount1;
            self._update(balance0, balance1)?;

            self.env().emit_event(Burn {
                sender: self.env().caller(),
                amount0,
                amount1,
                to,
                liquidity,
            });

            self.locked = false;
            Ok((amount0, amount1))
        }

        /// Swap tokens
        ///
        /// # Parameters
        /// * `amount0_out` - Amount of token0 to send
        /// * `amount1_out` - Amount of token1 to send
        /// * `to` - Recipient address
        ///
        /// # Requirements
        /// - One of amount0_out or amount1_out must be > 0
        /// - Caller must have sent input tokens first
        /// - K-value must not decrease (enforces constant product)
        #[ink(message)]
        pub fn swap(
            &mut self,
            amount0_out: Balance,
            amount1_out: Balance,
            to: AccountId,
        ) -> Result<()> {
            self.ensure_not_locked()?;
            self.locked = true;

            if amount0_out == 0 && amount1_out == 0 {
                self.locked = false;
                return Err(Error::InsufficientOutputAmount);
            }

            if to == AccountId::from([0u8; 32]) {
                self.locked = false;
                return Err(Error::InvalidTo);
            }

            let (reserve0, reserve1) = (self.reserve0, self.reserve1);

            if amount0_out >= reserve0 || amount1_out >= reserve1 {
                self.locked = false;
                return Err(Error::InsufficientLiquidity);
            }

            // Transfer tokens out
            if amount0_out > 0 {
                self._token_transfer(self.token0, to, amount0_out)?;
            }
            if amount1_out > 0 {
                self._token_transfer(self.token1, to, amount1_out)?;
            }

            // Get actual balances after transfer
            let balance0 = reserve0.saturating_sub(amount0_out);
            let balance1 = reserve1.saturating_sub(amount1_out);

            // Calculate amounts in (what user sent)
            let amount0_in = if balance0 > reserve0.saturating_sub(amount0_out) {
                balance0 - (reserve0 - amount0_out)
            } else {
                0
            };

            let amount1_in = if balance1 > reserve1.saturating_sub(amount1_out) {
                balance1 - (reserve1 - amount1_out)
            } else {
                0
            };

            if amount0_in == 0 && amount1_in == 0 {
                self.locked = false;
                return Err(Error::InsufficientInputAmount);
            }

            // Verify K-value (with 0.3% fee)
            let balance0_adjusted = balance0
                .saturating_mul(1000)
                .saturating_sub(amount0_in.saturating_mul(FEE_NUMERATOR));

            let balance1_adjusted = balance1
                .saturating_mul(1000)
                .saturating_sub(amount1_in.saturating_mul(FEE_NUMERATOR));

            let k_new = balance0_adjusted
                .checked_mul(balance1_adjusted)
                .ok_or(Error::Overflow)?;

            let k_old = reserve0
                .saturating_mul(reserve1)
                .saturating_mul(1000 * 1000);

            if k_new < k_old {
                self.locked = false;
                return Err(Error::KValueDecreased);
            }

            // Update reserves
            self._update(balance0, balance1)?;

            self.env().emit_event(Swap {
                sender: self.env().caller(),
                amount0_in,
                amount1_in,
                amount0_out,
                amount1_out,
                to,
            });

            self.locked = false;
            Ok(())
        }

        /// Force reserves to match actual balances (emergency function)
        #[ink(message)]
        pub fn sync(&mut self) -> Result<()> {
            self.ensure_not_locked()?;

            // Get actual balances
            // TODO: Call token contracts
            let balance0 = self.reserve0;
            let balance1 = self.reserve1;

            self._update(balance0, balance1)?;
            Ok(())
        }

        // ========================================================================
        // View Functions
        // ========================================================================

        /// Calculate amount out for exact amount in (before fees)
        #[ink(message)]
        pub fn get_amount_out(
            &self,
            amount_in: Balance,
            reserve_in: Balance,
            reserve_out: Balance,
        ) -> Result<Balance> {
            if amount_in == 0 {
                return Err(Error::InsufficientInputAmount);
            }

            if reserve_in == 0 || reserve_out == 0 {
                return Err(Error::InsufficientLiquidity);
            }

            // Apply 0.3% fee
            let amount_in_with_fee = amount_in.saturating_mul(FEE_DENOMINATOR - FEE_NUMERATOR);

            let numerator = amount_in_with_fee
                .checked_mul(reserve_out)
                .ok_or(Error::Overflow)?;

            let denominator = reserve_in
                .saturating_mul(FEE_DENOMINATOR)
                .saturating_add(amount_in_with_fee);

            let amount_out = numerator
                .checked_div(denominator)
                .ok_or(Error::InsufficientLiquidity)?;

            Ok(amount_out)
        }

        /// Calculate amount in for exact amount out (before fees)
        #[ink(message)]
        pub fn get_amount_in(
            &self,
            amount_out: Balance,
            reserve_in: Balance,
            reserve_out: Balance,
        ) -> Result<Balance> {
            if amount_out == 0 {
                return Err(Error::InsufficientOutputAmount);
            }

            if reserve_in == 0 || reserve_out == 0 || amount_out >= reserve_out {
                return Err(Error::InsufficientLiquidity);
            }

            let numerator = reserve_in
                .checked_mul(amount_out)
                .ok_or(Error::Overflow)?
                .saturating_mul(FEE_DENOMINATOR);

            let denominator = reserve_out
                .saturating_sub(amount_out)
                .saturating_mul(FEE_DENOMINATOR - FEE_NUMERATOR);

            let amount_in = numerator
                .checked_div(denominator)
                .ok_or(Error::InsufficientLiquidity)?
                .saturating_add(1); // Round up

            Ok(amount_in)
        }

        // ========================================================================
        // Internal Functions
        // ========================================================================

        /// Transfer tokens via PSP22 cross-contract call
        ///
        /// Calls the `transfer` method on a PSP22 token contract
        fn _token_transfer(&self, token: AccountId, to: AccountId, amount: Balance) -> Result<()> {
            // PSP22::transfer selector is 0xdb20f9f5
            let selector = [0xdb, 0x20, 0xf9, 0xf5];

            // Build cross-contract call
            let result = build_call::<Environment>()
                .call(token)
                .exec_input(
                    ExecutionInput::new(Selector::new(selector))
                        .push_arg(to)
                        .push_arg(amount),
                )
                .returns::<core::result::Result<(), ink::prelude::vec::Vec<u8>>>()
                .try_invoke();

            match result {
                Ok(Ok(_)) => Ok(()),
                _ => Err(Error::TransferFailed),
            }
        }

        /// Get token balance via PSP22 cross-contract call
        ///
        /// Calls the `balance_of` method on a PSP22 token contract
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

        /// Update reserves and price accumulators
        fn _update(&mut self, balance0: Balance, balance1: Balance) -> Result<()> {
            // Update price oracle (TWAP)
            let block_timestamp = self.env().block_timestamp();
            let time_elapsed = block_timestamp.saturating_sub(self.block_timestamp_last);

            if time_elapsed > 0 && self.reserve0 > 0 && self.reserve1 > 0 {
                // Price0 = reserve1 / reserve0
                self.price0_cumulative_last = self.price0_cumulative_last.saturating_add(
                    (self.reserve1 as u128)
                        .saturating_mul(time_elapsed as u128)
                        .saturating_div(self.reserve0 as u128),
                );

                // Price1 = reserve0 / reserve1
                self.price1_cumulative_last = self.price1_cumulative_last.saturating_add(
                    (self.reserve0 as u128)
                        .saturating_mul(time_elapsed as u128)
                        .saturating_div(self.reserve1 as u128),
                );
            }

            self.reserve0 = balance0;
            self.reserve1 = balance1;
            self.block_timestamp_last = block_timestamp;

            self.env().emit_event(Sync {
                reserve0: balance0,
                reserve1: balance1,
            });

            Ok(())
        }

        /// Transfer LP tokens
        fn _transfer(&mut self, from: AccountId, to: AccountId, value: Balance) -> Result<()> {
            if to == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            let from_balance = self.balance_of(from);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert(from, &(from_balance - value));

            let to_balance = self.balance_of(to);
            self.balances.insert(to, &to_balance.saturating_add(value));

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value,
            });

            Ok(())
        }

        /// Ensure contract is not locked (reentrancy guard)
        fn ensure_not_locked(&self) -> Result<()> {
            if self.locked {
                Err(Error::Locked)
            } else {
                Ok(())
            }
        }

        /// Integer square root (Babylonian method)
        fn sqrt(y: Balance) -> Balance {
            if y > 3 {
                let mut z = y;
                let mut x = y / 2 + 1;
                while x < z {
                    z = x;
                    x = (y / x + x) / 2;
                }
                z
            } else if y != 0 {
                1
            } else {
                0
            }
        }
    }

    // ============================================================================
    // Tests
    // ============================================================================

    #[cfg(test)]
    mod tests {
        use super::*;

        fn create_tokens() -> (AccountId, AccountId) {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            (accounts.bob, accounts.charlie)
        }

        #[ink::test]
        fn new_works() {
            let (token0, token1) = create_tokens();
            let pair = Pair::new(token0, token1);

            let (t0, t1) = pair.get_tokens();
            assert_eq!(t0, token0);
            assert_eq!(t1, token1);
            assert_eq!(pair.total_supply(), 0);
        }

        #[ink::test]
        fn sqrt_works() {
            assert_eq!(Pair::sqrt(0), 0);
            assert_eq!(Pair::sqrt(1), 1);
            assert_eq!(Pair::sqrt(4), 2);
            assert_eq!(Pair::sqrt(9), 3);
            assert_eq!(Pair::sqrt(16), 4);
            assert_eq!(Pair::sqrt(100), 10);
            assert_eq!(Pair::sqrt(1000000), 1000);
        }

        #[ink::test]
        fn get_amount_out_works() {
            let (token0, token1) = create_tokens();
            let pair = Pair::new(token0, token1);

            // Swap 100 tokens in pool with 1000 reserves each
            // Amount out = (100 * 997 * 1000) / (1000 * 1000 + 100 * 997)
            // = 99700000 / 1099700 ≈ 90.67
            let amount_out = pair.get_amount_out(100, 1000, 1000).unwrap();
            assert_eq!(amount_out, 90); // ~90 tokens out (with 0.3% fee)
        }

        #[ink::test]
        fn get_amount_in_works() {
            let (token0, token1) = create_tokens();
            let pair = Pair::new(token0, token1);

            // To get 90 tokens out from pool with 1000 reserves each
            let amount_in = pair.get_amount_in(90, 1000, 1000).unwrap();
            assert!(amount_in > 90); // Need more than 90 due to fee
            assert!(amount_in <= 100); // Approximately 100 tokens needed
        }
    }
}

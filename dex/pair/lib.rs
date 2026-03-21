#![cfg_attr(not(feature = "std"), no_std, no_main)]

//! # Pair Contract
//!
//! Core AMM trading pair implementing constant product formula (x * y = k).
//!
//! ## Features
//! - Liquidity provision (mint LP tokens)
//! - Liquidity removal (burn LP tokens)
//! - Token swaps with 0.3% fee
//! - Price oracle (TWAP with UQ64.64 fixed-point)
//! - Minimum liquidity lock
//!
//! ## Design Notes
//! - Flash swaps are intentionally not supported (no callback mechanism).
//!   This reduces the attack surface vs Uniswap V2 at the cost of flash loan utility.

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

    /// Fixed-point shift for TWAP price encoding (UQ64.64)
    const FIXED_POINT_SHIFT: u32 = 64;

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

        /// Cumulative price0 (UQ64.64 fixed-point, wrapping overflow by design)
        price0_cumulative_last: u128,

        /// Cumulative price1 (UQ64.64 fixed-point, wrapping overflow by design)
        price1_cumulative_last: u128,

        /// Reentrancy lock
        locked: bool,

        /// Emergency pause flag
        paused: bool,
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
        /// Caller is not authorized
        NotAuthorized,
        /// Cross-contract balance query failed
        BalanceQueryFailed,
        /// Contract is paused
        ContractPaused,
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
                locked: false,
                paused: false,
            }
        }

        // ========================================================================
        // LP Token Functions (PSP22-compliant)
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

        /// Transfer LP tokens (PSP22 compliant)
        #[ink(message)]
        pub fn transfer(
            &mut self,
            to: AccountId,
            value: Balance,
            _data: ink::prelude::vec::Vec<u8>,
        ) -> Result<()> {
            let caller = self.env().caller();
            self._transfer(caller, to, value)
        }

        /// Approve spender for LP tokens
        ///
        /// NOTE: To mitigate the approval race condition, callers should first
        /// set the allowance to 0 before setting it to a new value.
        /// Alternatively, use `increase_allowance` / `decrease_allowance`.
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

        /// Increase allowance atomically (race-condition safe)
        #[ink(message)]
        pub fn increase_allowance(&mut self, spender: AccountId, delta: Balance) -> Result<()> {
            let caller = self.env().caller();
            let current = self.allowances.get((caller, spender)).unwrap_or(0);
            let new_allowance = current.checked_add(delta).ok_or(Error::Overflow)?;
            self.allowances.insert((caller, spender), &new_allowance);

            self.env().emit_event(Approval {
                owner: caller,
                spender,
                value: new_allowance,
            });

            Ok(())
        }

        /// Decrease allowance atomically (race-condition safe)
        #[ink(message)]
        pub fn decrease_allowance(&mut self, spender: AccountId, delta: Balance) -> Result<()> {
            let caller = self.env().caller();
            let current = self.allowances.get((caller, spender)).unwrap_or(0);
            let new_allowance = current
                .checked_sub(delta)
                .ok_or(Error::InsufficientAllowance)?;
            self.allowances.insert((caller, spender), &new_allowance);

            self.env().emit_event(Approval {
                owner: caller,
                spender,
                value: new_allowance,
            });

            Ok(())
        }

        /// Transfer LP tokens from another account (PSP22 compliant)
        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
            _data: ink::prelude::vec::Vec<u8>,
        ) -> Result<()> {
            let caller = self.env().caller();
            let allowance = self.allowances.get((from, caller)).unwrap_or(0);

            if allowance < value {
                return Err(Error::InsufficientAllowance);
            }

            self.allowances.insert(
                (from, caller),
                &(allowance.checked_sub(value).ok_or(Error::Overflow)?),
            );
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
            self.ensure_not_paused()?;
            self.ensure_not_locked()?;
            self.locked = true;
            let result = self._mint_inner(to);
            self.locked = false;
            result
        }

        /// Internal mint logic (lock managed by caller)
        fn _mint_inner(&mut self, to: AccountId) -> Result<Balance> {
            if to == AccountId::from([0u8; 32]) {
                return Err(Error::InvalidTo);
            }

            let (reserve0, reserve1) = (self.reserve0, self.reserve1);

            // Get actual token balances via cross-contract calls
            let this = self.env().account_id();
            let balance0 = self._token_balance_of(self.token0, this)?;
            let balance1 = self._token_balance_of(self.token1, this)?;

            let amount0 = balance0.checked_sub(reserve0).ok_or(Error::Overflow)?;
            let amount1 = balance1.checked_sub(reserve1).ok_or(Error::Overflow)?;

            let liquidity = if self.total_supply == 0 {
                // First liquidity provision
                let initial_liquidity =
                    Self::sqrt(amount0.checked_mul(amount1).ok_or(Error::Overflow)?);

                if initial_liquidity <= MINIMUM_LIQUIDITY {
                    return Err(Error::InsufficientLiquidityMinted);
                }

                // Lock minimum liquidity forever (to zero address)
                let zero_address = AccountId::from([0u8; 32]);
                self.balances.insert(zero_address, &MINIMUM_LIQUIDITY);
                self.total_supply = MINIMUM_LIQUIDITY;

                // Emit Transfer event for MINIMUM_LIQUIDITY mint
                self.env().emit_event(Transfer {
                    from: None,
                    to: Some(zero_address),
                    value: MINIMUM_LIQUIDITY,
                });

                initial_liquidity
                    .checked_sub(MINIMUM_LIQUIDITY)
                    .ok_or(Error::Overflow)?
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
                return Err(Error::InsufficientLiquidityMinted);
            }

            // Mint LP tokens
            let to_balance = self.balance_of(to);
            self.balances.insert(
                to,
                &to_balance.checked_add(liquidity).ok_or(Error::Overflow)?,
            );
            self.total_supply = self
                .total_supply
                .checked_add(liquidity)
                .ok_or(Error::Overflow)?;

            // Emit Transfer event for LP token mint
            self.env().emit_event(Transfer {
                from: None,
                to: Some(to),
                value: liquidity,
            });

            // Update reserves
            self._update(balance0, balance1)?;

            self.env().emit_event(Mint {
                sender: self.env().caller(),
                amount0,
                amount1,
                liquidity,
            });

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
            self.ensure_not_paused()?;
            self.ensure_not_locked()?;
            self.locked = true;
            let result = self._burn_inner(to);
            self.locked = false;
            result
        }

        /// Internal burn logic (lock managed by caller)
        fn _burn_inner(&mut self, to: AccountId) -> Result<(Balance, Balance)> {
            if to == AccountId::from([0u8; 32]) {
                return Err(Error::InvalidTo);
            }

            let this = self.env().account_id();

            // Read actual token balances
            let balance0 = self._token_balance_of(self.token0, this)?;
            let balance1 = self._token_balance_of(self.token1, this)?;

            // Get LP tokens sent to this contract
            let liquidity = self.balance_of(this);

            if liquidity == 0 {
                return Err(Error::InsufficientLiquidityBurned);
            }

            // Calculate amounts to return (pro-rata based on actual balances)
            let amount0 = liquidity
                .checked_mul(balance0)
                .ok_or(Error::Overflow)?
                .checked_div(self.total_supply)
                .ok_or(Error::InsufficientLiquidity)?;

            let amount1 = liquidity
                .checked_mul(balance1)
                .ok_or(Error::Overflow)?
                .checked_div(self.total_supply)
                .ok_or(Error::InsufficientLiquidity)?;

            if amount0 == 0 || amount1 == 0 {
                return Err(Error::InsufficientLiquidityBurned);
            }

            // Burn LP tokens (checked arithmetic)
            let this_balance = self.balance_of(this);
            self.balances.insert(
                this,
                &this_balance.checked_sub(liquidity).ok_or(Error::Overflow)?,
            );
            self.total_supply = self
                .total_supply
                .checked_sub(liquidity)
                .ok_or(Error::Overflow)?;

            // Emit Transfer event for LP token burn
            self.env().emit_event(Transfer {
                from: Some(this),
                to: None,
                value: liquidity,
            });

            // Transfer tokens to recipient
            self._token_transfer(self.token0, to, amount0)?;
            self._token_transfer(self.token1, to, amount1)?;

            // Read actual balances after transfers
            let new_balance0 = self._token_balance_of(self.token0, this)?;
            let new_balance1 = self._token_balance_of(self.token1, this)?;
            self._update(new_balance0, new_balance1)?;

            self.env().emit_event(Burn {
                sender: self.env().caller(),
                amount0,
                amount1,
                to,
                liquidity,
            });

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
            self.ensure_not_paused()?;
            self.ensure_not_locked()?;
            self.locked = true;
            let result = self._swap_inner(amount0_out, amount1_out, to);
            self.locked = false;
            result
        }

        /// Internal swap logic (lock managed by caller)
        fn _swap_inner(
            &mut self,
            amount0_out: Balance,
            amount1_out: Balance,
            to: AccountId,
        ) -> Result<()> {
            if amount0_out == 0 && amount1_out == 0 {
                return Err(Error::InsufficientOutputAmount);
            }

            if to == AccountId::from([0u8; 32]) {
                return Err(Error::InvalidTo);
            }

            let (reserve0, reserve1) = (self.reserve0, self.reserve1);

            if amount0_out >= reserve0 || amount1_out >= reserve1 {
                return Err(Error::InsufficientLiquidity);
            }

            // Transfer tokens out
            if amount0_out > 0 {
                self._token_transfer(self.token0, to, amount0_out)?;
            }
            if amount1_out > 0 {
                self._token_transfer(self.token1, to, amount1_out)?;
            }

            // Get actual balances after transfer via cross-contract calls
            let this = self.env().account_id();
            let balance0 = self._token_balance_of(self.token0, this)?;
            let balance1 = self._token_balance_of(self.token1, this)?;

            // Calculate amounts in (what user sent)
            let amount0_in =
                if balance0 > reserve0.checked_sub(amount0_out).ok_or(Error::Overflow)? {
                    balance0
                        .checked_sub(reserve0.checked_sub(amount0_out).ok_or(Error::Overflow)?)
                        .ok_or(Error::Overflow)?
                } else {
                    0
                };

            let amount1_in =
                if balance1 > reserve1.checked_sub(amount1_out).ok_or(Error::Overflow)? {
                    balance1
                        .checked_sub(reserve1.checked_sub(amount1_out).ok_or(Error::Overflow)?)
                        .ok_or(Error::Overflow)?
                } else {
                    0
                };

            if amount0_in == 0 && amount1_in == 0 {
                return Err(Error::InsufficientInputAmount);
            }

            // Verify K-value (with 0.3% fee) using U256 to prevent overflow
            let balance0_adjusted = balance0
                .checked_mul(1000)
                .ok_or(Error::Overflow)?
                .checked_sub(
                    amount0_in
                        .checked_mul(FEE_NUMERATOR)
                        .ok_or(Error::Overflow)?,
                )
                .ok_or(Error::Overflow)?;

            let balance1_adjusted = balance1
                .checked_mul(1000)
                .ok_or(Error::Overflow)?
                .checked_sub(
                    amount1_in
                        .checked_mul(FEE_NUMERATOR)
                        .ok_or(Error::Overflow)?,
                )
                .ok_or(Error::Overflow)?;

            // Use U256 multiplication to prevent overflow in invariant check
            let (k_new_hi, k_new_lo) = Self::mul_u256(balance0_adjusted, balance1_adjusted);

            let reserve0_scaled = reserve0.checked_mul(1000).ok_or(Error::Overflow)?;
            let reserve1_scaled = reserve1.checked_mul(1000).ok_or(Error::Overflow)?;

            let (k_old_hi, k_old_lo) = Self::mul_u256(reserve0_scaled, reserve1_scaled);

            // Compare k_new >= k_old using 256-bit comparison
            if k_new_hi < k_old_hi || (k_new_hi == k_old_hi && k_new_lo < k_old_lo) {
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

            Ok(())
        }

        /// Force reserves to match actual balances (emergency function)
        #[ink(message)]
        pub fn sync(&mut self) -> Result<()> {
            self.ensure_not_locked()?;

            // Get actual balances via cross-contract calls
            let this = self.env().account_id();
            let balance0 = self._token_balance_of(self.token0, this)?;
            let balance1 = self._token_balance_of(self.token1, this)?;

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
            let amount_in_with_fee = amount_in
                .checked_mul(
                    FEE_DENOMINATOR
                        .checked_sub(FEE_NUMERATOR)
                        .ok_or(Error::Overflow)?,
                )
                .ok_or(Error::Overflow)?;

            let numerator = amount_in_with_fee
                .checked_mul(reserve_out)
                .ok_or(Error::Overflow)?;

            let denominator = reserve_in
                .checked_mul(FEE_DENOMINATOR)
                .ok_or(Error::Overflow)?
                .checked_add(amount_in_with_fee)
                .ok_or(Error::Overflow)?;

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
                .checked_mul(FEE_DENOMINATOR)
                .ok_or(Error::Overflow)?;

            let denominator = reserve_out
                .checked_sub(amount_out)
                .ok_or(Error::Overflow)?
                .checked_mul(
                    FEE_DENOMINATOR
                        .checked_sub(FEE_NUMERATOR)
                        .ok_or(Error::Overflow)?,
                )
                .ok_or(Error::Overflow)?;

            let amount_in = numerator
                .checked_div(denominator)
                .ok_or(Error::InsufficientLiquidity)?
                .checked_add(1)
                .ok_or(Error::Overflow)?; // Round up

            Ok(amount_in)
        }

        /// Upgrades the contract code hash (factory only)
        ///
        /// Allows the pair contract to be upgraded to a new implementation
        /// while preserving storage state. Only the factory that created
        /// this pair can trigger upgrades.
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

        /// Transfer tokens via PSP22 cross-contract call
        ///
        /// Calls the `transfer` method on a PSP22 token contract
        fn _token_transfer(&self, token: AccountId, to: AccountId, amount: Balance) -> Result<()> {
            // PSP22::transfer selector is 0xdb20f9f5
            let selector = [0xdb, 0x20, 0xf9, 0xf5];

            // Build cross-contract call with empty data (PSP22 compliance)
            let data: ink::prelude::vec::Vec<u8> = ink::prelude::vec::Vec::new();
            let result = build_call::<Environment>()
                .call(token)
                .exec_input(
                    ExecutionInput::new(Selector::new(selector))
                        .push_arg(to)
                        .push_arg(amount)
                        .push_arg(data),
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
        /// Calls the `balance_of` method on a PSP22 token contract.
        /// Returns `Err(BalanceQueryFailed)` if the cross-contract call fails.
        fn _token_balance_of(&self, token: AccountId, account: AccountId) -> Result<Balance> {
            // PSP22::balance_of selector is 0x65682523
            let selector = [0x65, 0x68, 0x25, 0x23];

            let result = build_call::<Environment>()
                .call(token)
                .exec_input(ExecutionInput::new(Selector::new(selector)).push_arg(account))
                .returns::<Balance>()
                .try_invoke();

            match result {
                Ok(Ok(balance)) => Ok(balance),
                _ => Err(Error::BalanceQueryFailed),
            }
        }

        /// Update reserves and TWAP price accumulators
        ///
        /// TWAP prices are encoded in UQ64.64 fixed-point and use wrapping
        /// arithmetic (designed to overflow). Consumers should subtract two
        /// readings and divide by elapsed time: `(acc_now - acc_before) / dt`.
        fn _update(&mut self, balance0: Balance, balance1: Balance) -> Result<()> {
            // Update price oracle (TWAP with UQ64.64 fixed-point, wrapping overflow)
            let block_timestamp = self.env().block_timestamp();
            let time_elapsed = block_timestamp.wrapping_sub(self.block_timestamp_last);

            if time_elapsed > 0 && self.reserve0 > 0 && self.reserve1 > 0 {
                // Price0 = reserve1 / reserve0 encoded as UQ64.64
                let price0 = (self.reserve1 << FIXED_POINT_SHIFT).wrapping_div(self.reserve0);
                self.price0_cumulative_last = self
                    .price0_cumulative_last
                    .wrapping_add(price0.wrapping_mul(time_elapsed as u128));

                // Price1 = reserve0 / reserve1 encoded as UQ64.64
                let price1 = (self.reserve0 << FIXED_POINT_SHIFT).wrapping_div(self.reserve1);
                self.price1_cumulative_last = self
                    .price1_cumulative_last
                    .wrapping_add(price1.wrapping_mul(time_elapsed as u128));
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

        /// Transfer LP tokens (internal)
        fn _transfer(&mut self, from: AccountId, to: AccountId, value: Balance) -> Result<()> {
            if to == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            let from_balance = self.balance_of(from);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert(
                from,
                &from_balance.checked_sub(value).ok_or(Error::Overflow)?,
            );

            let to_balance = self.balance_of(to);
            self.balances
                .insert(to, &to_balance.checked_add(value).ok_or(Error::Overflow)?);

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

        /// Ensure contract is not paused
        fn ensure_not_paused(&self) -> Result<()> {
            if self.paused {
                Err(Error::ContractPaused)
            } else {
                Ok(())
            }
        }

        /// Pause the contract (factory only)
        #[ink(message)]
        pub fn pause(&mut self) -> Result<()> {
            if self.env().caller() != self.factory {
                return Err(Error::NotAuthorized);
            }
            self.paused = true;
            Ok(())
        }

        /// Unpause the contract (factory only)
        #[ink(message)]
        pub fn unpause(&mut self) -> Result<()> {
            if self.env().caller() != self.factory {
                return Err(Error::NotAuthorized);
            }
            self.paused = false;
            Ok(())
        }

        /// Returns whether the contract is paused
        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.paused
        }

        /// Multiply two u128 values and return a 256-bit result as (hi, lo).
        ///
        /// Used for invariant check to prevent overflow with large reserves.
        fn mul_u256(a: u128, b: u128) -> (u128, u128) {
            let a_lo = a & 0xFFFF_FFFF_FFFF_FFFF_u128;
            let a_hi = a >> 64;
            let b_lo = b & 0xFFFF_FFFF_FFFF_FFFF_u128;
            let b_hi = b >> 64;

            let lo_lo = a_lo * b_lo;
            let hi_lo = a_hi * b_lo;
            let lo_hi = a_lo * b_hi;
            let hi_hi = a_hi * b_hi;

            let mid = (lo_lo >> 64)
                + (hi_lo & 0xFFFF_FFFF_FFFF_FFFF_u128)
                + (lo_hi & 0xFFFF_FFFF_FFFF_FFFF_u128);

            let lo =
                (lo_lo & 0xFFFF_FFFF_FFFF_FFFF_u128) | ((mid & 0xFFFF_FFFF_FFFF_FFFF_u128) << 64);
            let hi = hi_hi + (hi_lo >> 64) + (lo_hi >> 64) + (mid >> 64);

            (hi, lo)
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

        #[ink::test]
        fn mul_u256_works() {
            // Small values
            let (hi, lo) = Pair::mul_u256(10, 20);
            assert_eq!(hi, 0);
            assert_eq!(lo, 200);

            // Values that would overflow u128
            let a: u128 = u128::MAX / 2;
            let b: u128 = 4;
            let (hi, _) = Pair::mul_u256(a, b);
            assert!(hi > 0); // Must overflow into hi

            // Identity
            let (hi, lo) = Pair::mul_u256(1, u128::MAX);
            assert_eq!(hi, 0);
            assert_eq!(lo, u128::MAX);

            // Zero
            let (hi, lo) = Pair::mul_u256(0, u128::MAX);
            assert_eq!(hi, 0);
            assert_eq!(lo, 0);

            // Symmetry
            let (hi_a, lo_a) = Pair::mul_u256(12345, 67890);
            let (hi_b, lo_b) = Pair::mul_u256(67890, 12345);
            assert_eq!(hi_a, hi_b);
            assert_eq!(lo_a, lo_b);
        }

        #[ink::test]
        fn mul_u256_large_values() {
            // Two large values that would definitely overflow u128
            // 10^18 * 10^18 = 10^36 fits in u128 (max ~3.4e38)
            let a: u128 = 1_000_000_000_000_000_000; // 10^18
            let b: u128 = 1_000_000_000_000_000_000; // 10^18
            let (hi, lo) = Pair::mul_u256(a, b);
            assert_eq!(hi, 0);
            assert_eq!(lo, a * b);

            // 10^20 * 10^20 = 10^40 overflows u128(max ~3.4e38)
            let c: u128 = 100_000_000_000_000_000_000; // 10^20
            let (hi2, _lo2) = Pair::mul_u256(c, c);
            assert!(hi2 > 0);

            // MAX * MAX
            let (hi, lo) = Pair::mul_u256(u128::MAX, u128::MAX);
            // u128::MAX * u128::MAX = (2^128 - 1)^2 = 2^256 - 2^129 + 1
            assert_eq!(lo, 1);
            assert_eq!(hi, u128::MAX - 1);
        }

        #[ink::test]
        fn increase_decrease_allowance_works() {
            let (token0, token1) = create_tokens();
            let mut pair = Pair::new(token0, token1);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Increase allowance
            pair.increase_allowance(accounts.bob, 100).unwrap();
            assert_eq!(pair.allowance(accounts.alice, accounts.bob), 100);

            // Increase again
            pair.increase_allowance(accounts.bob, 50).unwrap();
            assert_eq!(pair.allowance(accounts.alice, accounts.bob), 150);

            // Decrease
            pair.decrease_allowance(accounts.bob, 30).unwrap();
            assert_eq!(pair.allowance(accounts.alice, accounts.bob), 120);

            // Decrease more than current should fail
            let result = pair.decrease_allowance(accounts.bob, 200);
            assert_eq!(result, Err(Error::InsufficientAllowance));
        }

        #[ink::test]
        fn transfer_to_zero_address_fails() {
            let (token0, token1) = create_tokens();
            let mut pair = Pair::new(token0, token1);
            let zero = AccountId::from([0u8; 32]);

            let result = pair.transfer(zero, 100, ink::prelude::vec::Vec::new());
            assert_eq!(result, Err(Error::ZeroAddress));
        }

        #[ink::test]
        fn transfer_insufficient_balance_fails() {
            let (token0, token1) = create_tokens();
            let mut pair = Pair::new(token0, token1);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let result = pair.transfer(accounts.bob, 100, ink::prelude::vec::Vec::new());
            assert_eq!(result, Err(Error::InsufficientBalance));
        }

        #[ink::test]
        fn approve_and_allowance_works() {
            let (token0, token1) = create_tokens();
            let mut pair = Pair::new(token0, token1);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            pair.approve(accounts.bob, 500).unwrap();
            assert_eq!(pair.allowance(accounts.alice, accounts.bob), 500);

            // Overwrite
            pair.approve(accounts.bob, 200).unwrap();
            assert_eq!(pair.allowance(accounts.alice, accounts.bob), 200);
        }

        #[ink::test]
        fn transfer_from_insufficient_allowance_fails() {
            let (token0, token1) = create_tokens();
            let mut pair = Pair::new(token0, token1);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            let result = pair.transfer_from(
                accounts.alice,
                accounts.bob,
                100,
                ink::prelude::vec::Vec::new(),
            );
            assert_eq!(result, Err(Error::InsufficientAllowance));
        }

        #[ink::test]
        fn get_amount_out_zero_input_fails() {
            let (token0, token1) = create_tokens();
            let pair = Pair::new(token0, token1);

            let result = pair.get_amount_out(0, 1000, 1000);
            assert_eq!(result, Err(Error::InsufficientInputAmount));
        }

        #[ink::test]
        fn get_amount_out_zero_reserves_fails() {
            let (token0, token1) = create_tokens();
            let pair = Pair::new(token0, token1);

            assert_eq!(
                pair.get_amount_out(100, 0, 1000),
                Err(Error::InsufficientLiquidity)
            );
            assert_eq!(
                pair.get_amount_out(100, 1000, 0),
                Err(Error::InsufficientLiquidity)
            );
        }

        #[ink::test]
        fn get_amount_in_zero_output_fails() {
            let (token0, token1) = create_tokens();
            let pair = Pair::new(token0, token1);

            let result = pair.get_amount_in(0, 1000, 1000);
            assert_eq!(result, Err(Error::InsufficientOutputAmount));
        }

        #[ink::test]
        fn get_amount_in_zero_reserves_fails() {
            let (token0, token1) = create_tokens();
            let pair = Pair::new(token0, token1);

            assert_eq!(
                pair.get_amount_in(100, 0, 1000),
                Err(Error::InsufficientLiquidity)
            );
            assert_eq!(
                pair.get_amount_in(100, 1000, 0),
                Err(Error::InsufficientLiquidity)
            );
        }

        #[ink::test]
        fn get_amount_in_output_exceeds_reserve_fails() {
            let (token0, token1) = create_tokens();
            let pair = Pair::new(token0, token1);

            assert_eq!(
                pair.get_amount_in(1001, 1000, 1000),
                Err(Error::InsufficientLiquidity)
            );
        }

        #[ink::test]
        fn get_reserves_initially_zero() {
            let (token0, token1) = create_tokens();
            let pair = Pair::new(token0, token1);

            let (r0, r1, ts) = pair.get_reserves();
            assert_eq!(r0, 0);
            assert_eq!(r1, 0);
            assert_eq!(ts, 0);
        }

        #[ink::test]
        fn balance_of_returns_zero_for_unknown() {
            let (token0, token1) = create_tokens();
            let pair = Pair::new(token0, token1);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            assert_eq!(pair.balance_of(accounts.django), 0);
        }
    }
}

#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod faucet {
    use ink::storage::Mapping;

    /// Faucet contract for distributing test DALLA tokens
    #[ink(storage)]
    pub struct Faucet {
        /// Tracks last claim time per account
        last_claim: Mapping<AccountId, BlockNumber>,
        /// Amount to dispense per claim (in base units)
        drip_amount: Balance,
        /// Cooldown period in blocks
        cooldown: u32,
        /// Contract owner who can refill and adjust settings
        owner: AccountId,
        /// Total tokens claimed
        total_claimed: Balance,
        /// Total number of claims
        claim_count: u32,
    }

    /// Errors that can occur during faucet operations
    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        /// Claim too soon (still in cooldown)
        TooSoon,
        /// Insufficient faucet balance
        InsufficientBalance,
        /// Not the contract owner
        NotOwner,
        /// Transfer failed
        TransferFailed,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    /// Events emitted by the faucet
    #[ink(event)]
    pub struct Claimed {
        #[ink(topic)]
        account: AccountId,
        amount: Balance,
        block: BlockNumber,
    }

    #[ink(event)]
    pub struct Refilled {
        #[ink(topic)]
        from: AccountId,
        amount: Balance,
    }

    #[ink(event)]
    pub struct SettingsUpdated {
        drip_amount: Balance,
        cooldown: u32,
    }

    impl Faucet {
        /// Create a new faucet contract
        ///
        /// # Arguments
        /// * `drip_amount` - Amount to dispense per claim (e.g., 1000 DALLA = 1000 * 10^12)
        /// * `cooldown` - Blocks between claims (e.g., 100 blocks ≈ 10 minutes)
        #[ink(constructor, payable)]
        pub fn new(drip_amount: Balance, cooldown: u32) -> Self {
            Self {
                last_claim: Mapping::default(),
                drip_amount,
                cooldown,
                owner: Self::env().caller(),
                total_claimed: 0,
                claim_count: 0,
            }
        }

        /// Claim test DALLA tokens
        #[ink(message)]
        pub fn claim(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let current_block = self.env().block_number();

            // Check cooldown
            if let Some(last) = self.last_claim.get(&caller) {
                let blocks_since = current_block.saturating_sub(last);
                if blocks_since < self.cooldown {
                    return Err(Error::TooSoon);
                }
            }

            // Check faucet balance
            let balance = self.env().balance();
            if balance < self.drip_amount {
                return Err(Error::InsufficientBalance);
            }

            // Transfer tokens
            if self.env().transfer(caller, self.drip_amount).is_err() {
                return Err(Error::TransferFailed);
            }

            // Update state
            self.last_claim.insert(caller, &current_block);
            self.total_claimed = self.total_claimed.saturating_add(self.drip_amount);
            self.claim_count = self.claim_count.saturating_add(1);

            // Emit event
            self.env().emit_event(Claimed {
                account: caller,
                amount: self.drip_amount,
                block: current_block,
            });

            Ok(())
        }

        /// Refill the faucet (anyone can refill)
        #[ink(message, payable)]
        pub fn refill(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let amount = self.env().transferred_value();

            self.env().emit_event(Refilled {
                from: caller,
                amount,
            });

            Ok(())
        }

        /// Update faucet settings (owner only)
        #[ink(message)]
        pub fn update_settings(
            &mut self,
            drip_amount: Option<Balance>,
            cooldown: Option<u32>,
        ) -> Result<()> {
            self.ensure_owner()?;

            if let Some(amount) = drip_amount {
                self.drip_amount = amount;
            }

            if let Some(cd) = cooldown {
                self.cooldown = cd;
            }

            self.env().emit_event(SettingsUpdated {
                drip_amount: self.drip_amount,
                cooldown: self.cooldown,
            });

            Ok(())
        }

        /// Transfer ownership (owner only)
        #[ink(message)]
        pub fn transfer_ownership(&mut self, new_owner: AccountId) -> Result<()> {
            self.ensure_owner()?;
            self.owner = new_owner;
            Ok(())
        }

        /// Withdraw all funds (owner only, emergency use)
        #[ink(message)]
        pub fn emergency_withdraw(&mut self) -> Result<()> {
            self.ensure_owner()?;

            let balance = self.env().balance();
            if self.env().transfer(self.owner, balance).is_err() {
                return Err(Error::TransferFailed);
            }

            Ok(())
        }

        // === Query Functions ===

        /// Get the current drip amount
        #[ink(message)]
        pub fn drip_amount(&self) -> Balance {
            self.drip_amount
        }

        /// Get the cooldown period
        #[ink(message)]
        pub fn cooldown(&self) -> u32 {
            self.cooldown
        }

        /// Get the contract owner
        #[ink(message)]
        pub fn owner(&self) -> AccountId {
            self.owner
        }

        /// Get the last claim block for an account
        #[ink(message)]
        pub fn last_claim_block(&self, account: AccountId) -> Option<BlockNumber> {
            self.last_claim.get(&account)
        }

        /// Check if an account can claim now
        #[ink(message)]
        pub fn can_claim(&self, account: AccountId) -> bool {
            let current_block = self.env().block_number();

            match self.last_claim.get(&account) {
                Some(last) => {
                    let blocks_since = current_block.saturating_sub(last);
                    blocks_since >= self.cooldown
                }
                None => true, // First claim
            }
        }

        /// Get blocks remaining until next claim
        #[ink(message)]
        pub fn blocks_until_claim(&self, account: AccountId) -> u32 {
            let current_block = self.env().block_number();

            match self.last_claim.get(&account) {
                Some(last) => {
                    let blocks_since = current_block.saturating_sub(last);
                    if blocks_since >= self.cooldown {
                        0
                    } else {
                        self.cooldown.saturating_sub(blocks_since)
                    }
                }
                None => 0, // Can claim now
            }
        }

        /// Get faucet balance
        #[ink(message)]
        pub fn balance(&self) -> Balance {
            self.env().balance()
        }

        /// Get total claimed amount
        #[ink(message)]
        pub fn total_claimed(&self) -> Balance {
            self.total_claimed
        }

        /// Get total number of claims
        #[ink(message)]
        pub fn claim_count(&self) -> u32 {
            self.claim_count
        }

        /// Get faucet statistics
        #[ink(message)]
        pub fn stats(&self) -> (Balance, u32, Balance) {
            (self.total_claimed, self.claim_count, self.env().balance())
        }

        // === Helper Functions ===

        /// Ensure caller is the owner
        fn ensure_owner(&self) -> Result<()> {
            if self.env().caller() != self.owner {
                return Err(Error::NotOwner);
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn new_works() {
            let faucet = Faucet::new(1000, 100);
            assert_eq!(faucet.drip_amount(), 1000);
            assert_eq!(faucet.cooldown(), 100);
            assert_eq!(faucet.total_claimed(), 0);
            assert_eq!(faucet.claim_count(), 0);
        }

        #[ink::test]
        fn can_claim_works() {
            let faucet = Faucet::new(1000, 100);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // New account should be able to claim
            assert!(faucet.can_claim(accounts.bob));
        }

        #[ink::test]
        fn update_settings_works() {
            let mut faucet = Faucet::new(1000, 100);

            assert!(faucet.update_settings(Some(2000), Some(200)).is_ok());
            assert_eq!(faucet.drip_amount(), 2000);
            assert_eq!(faucet.cooldown(), 200);
        }

        #[ink::test]
        fn only_owner_can_update() {
            let mut faucet = Faucet::new(1000, 100);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Non-owner cannot update
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(
                faucet.update_settings(Some(2000), None),
                Err(Error::NotOwner)
            );
        }

        #[ink::test]
        fn transfer_ownership_works() {
            let mut faucet = Faucet::new(1000, 100);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            assert!(faucet.transfer_ownership(accounts.bob).is_ok());
            assert_eq!(faucet.owner(), accounts.bob);
        }
    }
}

#![cfg_attr(not(feature = "std"), no_std, no_main)]

//! # Access Control Library for ink! Smart Contracts
//!
//! This library provides secure access control patterns for smart contracts:
//! - **Ownable**: Single owner with transfer/renounce capabilities
//! - **AccessControl**: Role-based permissions (admin, minter, pauser, etc.)
//! - **Pausable**: Emergency stop functionality
//!
//! ## Usage
//!
//! ### Ownable Pattern
//! ```ignore
//! use access_control::ownable::*;
//!
//! #[ink(storage)]
//! pub struct MyContract {
//!     ownable: OwnableData,
//! }
//!
//! impl MyContract {
//!     #[ink(message)]
//!     pub fn admin_function(&mut self) -> Result<()> {
//!         self.ownable.ensure_owner(self.env().caller())?;
//!         // ... protected logic
//!     }
//! }
//! ```
//!
//! ### AccessControl Pattern
//! ```ignore
//! use access_control::access_control::*;
//!
//! #[ink(storage)]
//! pub struct MyContract {
//!     access_control: AccessControlData,
//! }
//!
//! const MINTER_ROLE: RoleType = 1;
//!
//! impl MyContract {
//!     #[ink(message)]
//!     pub fn mint(&mut self, to: AccountId, amount: Balance) -> Result<()> {
//!         self.access_control.ensure_role(self.env().caller(), MINTER_ROLE)?;
//!         // ... mint logic
//!     }
//! }
//! ```

use ink::prelude::vec::Vec;
use ink::storage::Mapping;
use scale::{Decode, Encode};

// ============================================================================
// Common Types & Errors
// ============================================================================

pub type RoleType = u8;

#[derive(Debug, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum AccessError {
    /// Caller is not the owner
    NotOwner,
    /// Caller is missing required role
    MissingRole,
    /// New owner is zero address
    ZeroAddress,
    /// Contract is paused
    Paused,
    /// Contract is not paused
    NotPaused,
}

pub type Result<T> = core::result::Result<T, AccessError>;

// ============================================================================
// Ownable Module
// ============================================================================

pub mod ownable {
    use super::*;
    use ink::env::DefaultEnvironment;
    use ink::primitives::AccountId;

    /// Storage for Ownable pattern
    #[derive(Debug, Default, Encode, Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct OwnableData {
        owner: Option<AccountId>,
    }

    /// Events for Ownable
    #[ink::event]
    pub struct OwnershipTransferred {
        #[ink(topic)]
        previous_owner: Option<AccountId>,
        #[ink(topic)]
        new_owner: Option<AccountId>,
    }

    impl OwnableData {
        /// Initialize Ownable with initial owner
        pub fn new(owner: AccountId) -> Self {
            Self { owner: Some(owner) }
        }

        /// Get current owner
        pub fn owner(&self) -> Option<AccountId> {
            self.owner
        }

        /// Ensure caller is owner (reverts if not)
        pub fn ensure_owner(&self, caller: AccountId) -> Result<()> {
            match self.owner {
                Some(owner) if owner == caller => Ok(()),
                _ => Err(AccessError::NotOwner),
            }
        }

        /// Check if account is owner
        pub fn is_owner(&self, account: AccountId) -> bool {
            match self.owner {
                Some(owner) => owner == account,
                None => false,
            }
        }

        /// Transfer ownership to new owner
        ///
        /// Requirements:
        /// - Caller must be current owner
        /// - New owner cannot be zero address
        pub fn transfer_ownership<E: ink::env::Environment>(
            &mut self,
            caller: AccountId,
            new_owner: AccountId,
            emit_event: impl FnOnce(OwnershipTransferred),
        ) -> Result<()> {
            self.ensure_owner(caller)?;

            if new_owner == AccountId::from([0u8; 32]) {
                return Err(AccessError::ZeroAddress);
            }

            let previous_owner = self.owner;
            self.owner = Some(new_owner);

            emit_event(OwnershipTransferred {
                previous_owner,
                new_owner: Some(new_owner),
            });

            Ok(())
        }

        /// Renounce ownership (leaves contract without owner)
        ///
        /// Requirements:
        /// - Caller must be current owner
        pub fn renounce_ownership<E: ink::env::Environment>(
            &mut self,
            caller: AccountId,
            emit_event: impl FnOnce(OwnershipTransferred),
        ) -> Result<()> {
            self.ensure_owner(caller)?;

            let previous_owner = self.owner;
            self.owner = None;

            emit_event(OwnershipTransferred {
                previous_owner,
                new_owner: None,
            });

            Ok(())
        }
    }
}

// ============================================================================
// AccessControl Module
// ============================================================================

pub mod access_control {
    use super::*;
    use ink::primitives::AccountId;
    use ink::storage::Mapping;

    /// Common role constants
    pub const DEFAULT_ADMIN_ROLE: RoleType = 0;
    pub const MINTER_ROLE: RoleType = 1;
    pub const BURNER_ROLE: RoleType = 2;
    pub const PAUSER_ROLE: RoleType = 3;
    pub const UPGRADER_ROLE: RoleType = 4;

    /// Storage for AccessControl pattern
    #[derive(Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct AccessControlData {
        /// Role assignments: (role, account) => has_role
        roles: Mapping<(RoleType, AccountId), ()>,
        /// Role admins: role => admin_role
        role_admins: Mapping<RoleType, RoleType>,
    }

    impl Default for AccessControlData {
        fn default() -> Self {
            Self {
                roles: Mapping::default(),
                role_admins: Mapping::default(),
            }
        }
    }

    /// Events for AccessControl
    #[ink::event]
    pub struct RoleGranted {
        #[ink(topic)]
        role: RoleType,
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        sender: AccountId,
    }

    #[ink::event]
    pub struct RoleRevoked {
        #[ink(topic)]
        role: RoleType,
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        sender: AccountId,
    }

    #[ink::event]
    pub struct RoleAdminChanged {
        #[ink(topic)]
        role: RoleType,
        previous_admin_role: RoleType,
        new_admin_role: RoleType,
    }

    impl AccessControlData {
        /// Initialize AccessControl with default admin
        pub fn new(admin: AccountId) -> Self {
            let mut data = Self::default();
            data.roles.insert((DEFAULT_ADMIN_ROLE, admin), &());
            data.role_admins.insert(DEFAULT_ADMIN_ROLE, &DEFAULT_ADMIN_ROLE);
            data
        }

        /// Check if account has role
        pub fn has_role(&self, role: RoleType, account: AccountId) -> bool {
            self.roles.contains((role, account))
        }

        /// Ensure caller has role (reverts if not)
        pub fn ensure_role(&self, caller: AccountId, role: RoleType) -> Result<()> {
            if self.has_role(role, caller) {
                Ok(())
            } else {
                Err(AccessError::MissingRole)
            }
        }

        /// Get admin role for a role
        pub fn get_role_admin(&self, role: RoleType) -> RoleType {
            self.role_admins
                .get(role)
                .unwrap_or(DEFAULT_ADMIN_ROLE)
        }

        /// Grant role to account
        ///
        /// Requirements:
        /// - Caller must have admin role for the target role
        pub fn grant_role(
            &mut self,
            caller: AccountId,
            role: RoleType,
            account: AccountId,
            emit_event: impl FnOnce(RoleGranted),
        ) -> Result<()> {
            let admin_role = self.get_role_admin(role);
            self.ensure_role(caller, admin_role)?;

            if !self.has_role(role, account) {
                self.roles.insert((role, account), &());

                emit_event(RoleGranted {
                    role,
                    account,
                    sender: caller,
                });
            }

            Ok(())
        }

        /// Revoke role from account
        ///
        /// Requirements:
        /// - Caller must have admin role for the target role
        pub fn revoke_role(
            &mut self,
            caller: AccountId,
            role: RoleType,
            account: AccountId,
            emit_event: impl FnOnce(RoleRevoked),
        ) -> Result<()> {
            let admin_role = self.get_role_admin(role);
            self.ensure_role(caller, admin_role)?;

            if self.has_role(role, account) {
                self.roles.remove((role, account));

                emit_event(RoleRevoked {
                    role,
                    account,
                    sender: caller,
                });
            }

            Ok(())
        }

        /// Renounce role for caller
        ///
        /// Allows account to give up their own role
        pub fn renounce_role(
            &mut self,
            caller: AccountId,
            role: RoleType,
            emit_event: impl FnOnce(RoleRevoked),
        ) -> Result<()> {
            if self.has_role(role, caller) {
                self.roles.remove((role, caller));

                emit_event(RoleRevoked {
                    role,
                    account: caller,
                    sender: caller,
                });
            }

            Ok(())
        }

        /// Set admin role for a role
        ///
        /// Requirements:
        /// - Caller must have DEFAULT_ADMIN_ROLE
        pub fn set_role_admin(
            &mut self,
            caller: AccountId,
            role: RoleType,
            admin_role: RoleType,
            emit_event: impl FnOnce(RoleAdminChanged),
        ) -> Result<()> {
            self.ensure_role(caller, DEFAULT_ADMIN_ROLE)?;

            let previous_admin_role = self.get_role_admin(role);
            self.role_admins.insert(role, &admin_role);

            emit_event(RoleAdminChanged {
                role,
                previous_admin_role,
                new_admin_role: admin_role,
            });

            Ok(())
        }
    }
}

// ============================================================================
// Pausable Module
// ============================================================================

pub mod pausable {
    use super::*;
    use ink::primitives::AccountId;

    /// Storage for Pausable pattern
    #[derive(Debug, Default, Encode, Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct PausableData {
        paused: bool,
    }

    /// Events for Pausable
    #[ink::event]
    pub struct Paused {
        #[ink(topic)]
        account: AccountId,
    }

    #[ink::event]
    pub struct Unpaused {
        #[ink(topic)]
        account: AccountId,
    }

    impl PausableData {
        /// Initialize Pausable (starts unpaused)
        pub fn new() -> Self {
            Self { paused: false }
        }

        /// Check if contract is paused
        pub fn is_paused(&self) -> bool {
            self.paused
        }

        /// Ensure contract is not paused (reverts if paused)
        pub fn ensure_not_paused(&self) -> Result<()> {
            if self.paused {
                Err(AccessError::Paused)
            } else {
                Ok(())
            }
        }

        /// Ensure contract is paused (reverts if not paused)
        pub fn ensure_paused(&self) -> Result<()> {
            if !self.paused {
                Err(AccessError::NotPaused)
            } else {
                Ok(())
            }
        }

        /// Pause contract
        ///
        /// Requirements:
        /// - Contract must not be paused
        pub fn pause(&mut self, caller: AccountId, emit_event: impl FnOnce(Paused)) -> Result<()> {
            self.ensure_not_paused()?;
            self.paused = true;

            emit_event(Paused { account: caller });

            Ok(())
        }

        /// Unpause contract
        ///
        /// Requirements:
        /// - Contract must be paused
        pub fn unpause(
            &mut self,
            caller: AccountId,
            emit_event: impl FnOnce(Unpaused),
        ) -> Result<()> {
            self.ensure_paused()?;
            self.paused = false;

            emit_event(Unpaused { account: caller });

            Ok(())
        }
    }
}

// ============================================================================
// Example Contract Using Access Control
// ============================================================================

#[cfg(feature = "example")]
#[ink::contract]
mod example_contract {
    use super::access_control::*;
    use super::ownable::*;
    use super::pausable::*;
    use super::*;

    #[ink(storage)]
    pub struct ExampleContract {
        ownable: OwnableData,
        access_control: AccessControlData,
        pausable: PausableData,
        value: u128,
    }

    impl ExampleContract {
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                ownable: OwnableData::new(caller),
                access_control: AccessControlData::new(caller),
                pausable: PausableData::new(),
                value: 0,
            }
        }

        /// Owner-only function
        #[ink(message)]
        pub fn owner_function(&mut self, new_value: u128) -> Result<()> {
            let caller = self.env().caller();
            self.ownable.ensure_owner(caller)?;
            self.value = new_value;
            Ok(())
        }

        /// Role-based function (requires MINTER_ROLE)
        #[ink(message)]
        pub fn minter_function(&mut self) -> Result<()> {
            let caller = self.env().caller();
            self.access_control.ensure_role(caller, MINTER_ROLE)?;
            self.value = self.value.saturating_add(1);
            Ok(())
        }

        /// Pausable function (reverts when paused)
        #[ink(message)]
        pub fn pausable_function(&mut self) -> Result<()> {
            self.pausable.ensure_not_paused()?;
            self.value = self.value.saturating_add(1);
            Ok(())
        }

        /// Pause contract (owner only)
        #[ink(message)]
        pub fn pause(&mut self) -> Result<()> {
            let caller = self.env().caller();
            self.ownable.ensure_owner(caller)?;
            self.pausable.pause(caller, |event| {
                self.env().emit_event(event);
            })
        }

        /// Unpause contract (owner only)
        #[ink(message)]
        pub fn unpause(&mut self) -> Result<()> {
            let caller = self.env().caller();
            self.ownable.ensure_owner(caller)?;
            self.pausable.unpause(caller, |event| {
                self.env().emit_event(event);
            })
        }

        /// Grant role (admin only)
        #[ink(message)]
        pub fn grant_role(&mut self, role: RoleType, account: AccountId) -> Result<()> {
            let caller = self.env().caller();
            self.access_control.grant_role(caller, role, account, |event| {
                self.env().emit_event(event);
            })
        }

        /// Revoke role (admin only)
        #[ink(message)]
        pub fn revoke_role(&mut self, role: RoleType, account: AccountId) -> Result<()> {
            let caller = self.env().caller();
            self.access_control
                .revoke_role(caller, role, account, |event| {
                    self.env().emit_event(event);
                })
        }

        /// Check if account has role
        #[ink(message)]
        pub fn has_role(&self, role: RoleType, account: AccountId) -> bool {
            self.access_control.has_role(role, account)
        }

        /// Get current value
        #[ink(message)]
        pub fn get_value(&self) -> u128 {
            self.value
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn ownable_works() {
            let mut contract = ExampleContract::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Owner can call owner_function
            assert!(contract.owner_function(42).is_ok());
            assert_eq!(contract.get_value(), 42);

            // Non-owner cannot call owner_function
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert_eq!(contract.owner_function(100), Err(AccessError::NotOwner));
        }

        #[ink::test]
        fn access_control_works() {
            let mut contract = ExampleContract::new();
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            // Grant MINTER_ROLE to Bob
            contract.grant_role(MINTER_ROLE, accounts.bob).unwrap();
            assert!(contract.has_role(MINTER_ROLE, accounts.bob));

            // Bob can call minter_function
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
            assert!(contract.minter_function().is_ok());

            // Charlie cannot call minter_function
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.charlie);
            assert_eq!(contract.minter_function(), Err(AccessError::MissingRole));
        }

        #[ink::test]
        fn pausable_works() {
            let mut contract = ExampleContract::new();

            // Function works when not paused
            assert!(contract.pausable_function().is_ok());

            // Pause contract
            contract.pause().unwrap();

            // Function reverts when paused
            assert_eq!(contract.pausable_function(), Err(AccessError::Paused));

            // Unpause contract
            contract.unpause().unwrap();

            // Function works again
            assert!(contract.pausable_function().is_ok());
        }
    }
}

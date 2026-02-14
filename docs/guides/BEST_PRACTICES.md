# Best Practices for ink! Smart Contracts on BelizeChain 🎯

**Version**: 1.0  
**Platform**: BelizeChain GEM / ink! 4.0  
**Target Audience**: ink! smart contract developers

This guide covers best practices, design patterns, and optimization techniques for building secure, efficient, and maintainable smart contracts on BelizeChain.

---

## 📋 Table of Contents

1. [Project Setup & Structure](#project-setup--structure)
2. [Security Patterns](#security-patterns)
3. [Gas Optimization](#gas-optimization)
4. [Storage Management](#storage-management)
5. [Error Handling](#error-handling)
6. [Event Design](#event-design)
7. [Testing Strategies](#testing-strategies)
8. [Code Quality](#code-quality)
9. [Documentation](#documentation)
10. [BelizeChain-Specific Features](#belizechain-specific-features)

---

## 🏗️ Project Setup & Structure

### Workspace Organization

```bash
my-project/
├── Cargo.toml          # Workspace manifest
├── README.md
├── .gitignore
├── contracts/
│   ├── token/         # One contract per directory
│   │   ├── Cargo.toml
│   │   └── lib.rs
│   └── marketplace/
│       ├── Cargo.toml
│       └── lib.rs
├── sdk/               # TypeScript/JavaScript SDK
│   ├── index.js
│   └── index.d.ts
└── tests/             # Integration tests
    └── e2e.rs
```

**✅ DO**:
- One contract per directory
- Clear naming conventions (`token`, `marketplace`, not `contract1`)
- Separate SDK and integration tests

**❌ DON'T**:
- Mix multiple contracts in one file
- Use generic names like `my_contract`

### Cargo.toml Best Practices

```toml
[package]
name = "token_contract"
version = "1.0.0"
edition = "2021"

[dependencies]
ink = { version = "5.0.0", default-features = false }

# Use workspace inheritance for consistency
[workspace.package]
version = "1.0.0"
authors = ["Your Team <dev@example.com>"]
edition = "2021"

[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Better optimization (slower compile)
overflow-checks = true # Keep safety checks
```

---

## 🔐 Security Patterns

### 1. Access Control

**Use Ownable for Simple Cases**:

```rust
use access_control::ownable::*;

#[ink(storage)]
pub struct MyContract {
    ownable: OwnableData,
}

impl MyContract {
    #[ink(constructor)]
    pub fn new() -> Self {
        Self {
            ownable: OwnableData::new(Self::env().caller()),
        }
    }

    #[ink(message)]
    pub fn admin_function(&mut self) -> Result<()> {
        self.ownable.ensure_owner(self.env().caller())?;
        // Protected logic here
        Ok(())
    }
}
```

**Use AccessControl for Complex Permissions**:

```rust
use access_control::access_control::*;

const MINTER_ROLE: RoleType = 1;
const BURNER_ROLE: RoleType = 2;

#[ink(storage)]
pub struct MyContract {
    access_control: AccessControlData,
}

impl MyContract {
    #[ink(message)]
    pub fn mint(&mut self, to: AccountId, amount: Balance) -> Result<()> {
        self.access_control.ensure_role(self.env().caller(), MINTER_ROLE)?;
        // Mint logic
        Ok(())
    }
}
```

### 2. Checks-Effects-Interactions Pattern

**✅ CORRECT**:

```rust
#[ink(message)]
pub fn withdraw(&mut self, amount: Balance) -> Result<()> {
    let caller = self.env().caller();
    
    // 1. CHECKS: Validate conditions first
    let balance = self.balances.get(caller).unwrap_or(0);
    if balance < amount {
        return Err(Error::InsufficientBalance);
    }
    
    // 2. EFFECTS: Update state second
    self.balances.insert(caller, &(balance - amount));
    
    // 3. INTERACTIONS: External calls last
    self.env().transfer(caller, amount)?;
    
    Ok(())
}
```

**❌ WRONG** (vulnerable to reentrancy):

```rust
#[ink(message)]
pub fn withdraw(&mut self, amount: Balance) -> Result<()> {
    let caller = self.env().caller();
    let balance = self.balances.get(caller).unwrap_or(0);
    
    // BAD: External call before state update!
    self.env().transfer(caller, amount)?;
    
    // Too late - attacker can reenter before this
    self.balances.insert(caller, &(balance - amount));
    
    Ok(())
}
```

### 3. Input Validation

**Always Validate Inputs**:

```rust
#[ink(message)]
pub fn transfer(&mut self, to: AccountId, amount: Balance) -> Result<()> {
    let caller = self.env().caller();
    
    // Validate zero address
    if to == AccountId::from([0u8; 32]) {
        return Err(Error::ZeroAddress);
    }
    
    // Validate zero amount (if not allowed)
    if amount == 0 {
        return Err(Error::ZeroAmount);
    }
    
    // Validate sufficient balance
    let balance = self.balance_of(caller);
    if balance < amount {
        return Err(Error::InsufficientBalance);
    }
    
    // Proceed with transfer
    self._transfer(caller, to, amount)
}
```

### 4. Pausable Pattern

```rust
use access_control::pausable::*;

#[ink(storage)]
pub struct MyContract {
    pausable: PausableData,
    ownable: OwnableData,
}

impl MyContract {
    #[ink(message)]
    pub fn critical_function(&mut self) -> Result<()> {
        // Automatically reverts if paused
        self.pausable.ensure_not_paused()?;
        
        // Business logic
        Ok(())
    }
    
    #[ink(message)]
    pub fn pause(&mut self) -> Result<()> {
        let caller = self.env().caller();
        self.ownable.ensure_owner(caller)?;
        self.pausable.pause(caller, |event| {
            self.env().emit_event(event);
        })
    }
}
```

---

## ⛽ Gas Optimization

### 1. Use Saturating Arithmetic

**✅ DO** (no panic, predictable gas):

```rust
let new_balance = balance.saturating_add(amount);
let new_balance = balance.saturating_sub(amount);
let new_balance = balance.saturating_mul(multiplier);
```

**❌ DON'T** (can panic, unpredictable gas):

```rust
let new_balance = balance + amount;  // Panics on overflow!
let new_balance = balance - amount;  // Panics on underflow!
```

### 2. Cache Storage Reads

**✅ EFFICIENT**:

```rust
#[ink(message)]
pub fn process(&mut self) {
    // Read once, cache in variable
    let total = self.total_supply.get().unwrap_or(0);
    
    let result1 = total * 2;
    let result2 = total / 3;
    let result3 = total.saturating_add(100);
    
    // Write once
    self.total_supply.set(&total);
}
```

**❌ INEFFICIENT** (multiple reads):

```rust
#[ink(message)]
pub fn process(&mut self) {
    let result1 = self.total_supply.get().unwrap_or(0) * 2;
    let result2 = self.total_supply.get().unwrap_or(0) / 3;  // Read again!
    let result3 = self.total_supply.get().unwrap_or(0).saturating_add(100);  // Read again!
}
```

### 3. Batch Operations

**✅ PROVIDE BATCH FUNCTIONS**:

```rust
#[ink(message)]
pub fn batch_transfer(
    &mut self,
    recipients: Vec<AccountId>,
    amounts: Vec<Balance>,
) -> Result<()> {
    if recipients.len() != amounts.len() {
        return Err(Error::ArrayLengthMismatch);
    }
    
    // Limit array size to prevent DoS
    if recipients.len() > 100 {
        return Err(Error::BatchTooLarge);
    }
    
    for (to, amount) in recipients.iter().zip(amounts.iter()) {
        self._transfer(self.env().caller(), *to, *amount)?;
    }
    
    Ok(())
}
```

### 4. Minimize Storage Writes

**✅ EFFICIENT** (one write):

```rust
#[ink(message)]
pub fn update_config(&mut self, a: u32, b: u32, c: u32) {
    // Update entire struct at once
    let config = Config { a, b, c };
    self.config.set(&config);
}
```

**❌ INEFFICIENT** (three writes):

```rust
#[ink(message)]
pub fn update_config(&mut self, a: u32, b: u32, c: u32) {
    self.config.a.set(&a);  // Write 1
    self.config.b.set(&b);  // Write 2
    self.config.c.set(&c);  // Write 3
}
```

---

## 💾 Storage Management

### 1. Choose Right Storage Type

```rust
use ink::storage::{Mapping, Lazy};
use ink::prelude::vec::Vec;

#[ink(storage)]
pub struct MyContract {
    // Use Mapping for unbounded key-value pairs
    // (balances can grow to millions of users)
    balances: Mapping<AccountId, Balance>,
    
    // Use Lazy for large, infrequently accessed data
    metadata: Lazy<LargeMetadata>,
    
    // Use primitive types for frequently accessed data
    total_supply: Balance,
    owner: AccountId,
    
    // ❌ AVOID: Vec for unbounded collections
    // all_users: Vec<AccountId>,  // Don't do this!
}
```

### 2. Storage Layout Optimization

**Pack Small Types Together**:

```rust
#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub struct CompactData {
    // Pack small types together (32 bytes total)
    value1: u64,      // 8 bytes
    value2: u64,      // 8 bytes
    value3: u64,      // 8 bytes
    flag: bool,       // 1 byte
    status: u8,       // 1 byte
    // 6 bytes padding
}
```

### 3. Clear Unused Storage

**Free Storage When Done**:

```rust
#[ink(message)]
pub fn remove_user(&mut self, user: AccountId) -> Result<()> {
    // Free storage slot (saves gas for future operations)
    self.balances.remove(user);
    self.metadata.remove(user);
    
    self.env().emit_event(UserRemoved { user });
    Ok(())
}
```

---

## ⚠️ Error Handling

### 1. Define Custom Error Types

**✅ BEST PRACTICE**:

```rust
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    /// Insufficient balance for transfer
    InsufficientBalance,
    /// Not authorized to perform operation
    NotAuthorized,
    /// Token ID does not exist
    TokenNotFound,
    /// Zero address not allowed
    ZeroAddress,
    /// Amount cannot be zero
    ZeroAmount,
    /// Array length mismatch
    ArrayLengthMismatch,
    /// Contract is paused
    Paused,
}

pub type Result<T> = core::result::Result<T, Error>;
```

**❌ DON'T** use string errors:

```rust
// DON'T DO THIS - wastes gas and hard to handle
Err("Insufficient balance".into())
```

### 2. Propagate Errors with `?`

**✅ CLEAN**:

```rust
#[ink(message)]
pub fn transfer(&mut self, to: AccountId, amount: Balance) -> Result<()> {
    self.pausable.ensure_not_paused()?;  // Auto return on error
    self.ownable.ensure_owner(self.env().caller())?;
    self._transfer(self.env().caller(), to, amount)?;
    Ok(())
}
```

**❌ VERBOSE**:

```rust
#[ink(message)]
pub fn transfer(&mut self, to: AccountId, amount: Balance) -> Result<()> {
    match self.pausable.ensure_not_paused() {
        Ok(_) => {},
        Err(e) => return Err(e),
    }
    // Too verbose!
}
```

---

## 📢 Event Design

### 1. Emit Events for All State Changes

**✅ COMPLETE EXAMPLE**:

```rust
#[ink(event)]
pub struct Transfer {
    #[ink(topic)]  // Indexed for filtering
    from: Option<AccountId>,
    #[ink(topic)]  // Indexed for filtering
    to: Option<AccountId>,
    value: Balance,  // Not indexed (data only)
}

#[ink(message)]
pub fn transfer(&mut self, to: AccountId, amount: Balance) -> Result<()> {
    let caller = self.env().caller();
    
    // ... transfer logic ...
    
    // Always emit event!
    self.env().emit_event(Transfer {
        from: Some(caller),
        to: Some(to),
        value: amount,
    });
    
    Ok(())
}
```

### 2. Use Topics Wisely (Max 3)

**✅ GOOD** (3 indexed topics):

```rust
#[ink(event)]
pub struct TradeExecuted {
    #[ink(topic)]
    trader: AccountId,      // Topic 1: Filter by trader
    #[ink(topic)]
    token_in: AccountId,    // Topic 2: Filter by input token
    #[ink(topic)]
    token_out: AccountId,   // Topic 3: Filter by output token
    amount_in: Balance,     // Data: Amount in
    amount_out: Balance,    // Data: Amount out
    timestamp: u64,         // Data: Timestamp
}
```

**❌ TOO MANY** (more than 3 indexed):

```rust
#[ink(event)]
pub struct TradeExecuted {
    #[ink(topic)] trader: AccountId,
    #[ink(topic)] token_in: AccountId,
    #[ink(topic)] token_out: AccountId,
    #[ink(topic)] amount_in: Balance,  // ❌ 4th topic won't work!
}
```

---

## 🧪 Testing Strategies

### 1. Comprehensive Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[ink::test]
    fn transfer_works() {
        let mut contract = MyContract::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        
        // Test successful path
        assert!(contract.transfer(accounts.bob, 100).is_ok());
        assert_eq!(contract.balance_of(accounts.alice), 900);
        assert_eq!(contract.balance_of(accounts.bob), 100);
    }

    #[ink::test]
    fn transfer_fails_insufficient_balance() {
        let mut contract = MyContract::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        
        // Test error path
        assert_eq!(
            contract.transfer(accounts.bob, 10_000),
            Err(Error::InsufficientBalance)
        );
    }

    #[ink::test]
    fn only_owner_can_mint() {
        let mut contract = MyContract::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        
        // Switch caller to Bob (not owner)
        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);
        
        // Should fail
        assert_eq!(
            contract.mint(accounts.charlie, 100),
            Err(Error::NotAuthorized)
        );
    }
}
```

### 2. Test Edge Cases

```rust
#[ink::test]
fn edge_cases() {
    let mut contract = MyContract::new();
    let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
    
    // Zero amount
    assert_eq!(contract.transfer(accounts.bob, 0), Err(Error::ZeroAmount));
    
    // Zero address
    let zero_addr = AccountId::from([0u8; 32]);
    assert_eq!(contract.transfer(zero_addr, 100), Err(Error::ZeroAddress));
    
    // Self-transfer
    assert!(contract.transfer(accounts.alice, 100).is_ok());
    
    // Max value
    assert!(contract.mint(accounts.bob, u128::MAX).is_ok());
}
```

---

## 📝 Code Quality

### 1. Use Constants for Magic Numbers

**✅ READABLE**:

```rust
const MAX_SUPPLY: Balance = 1_000_000_000;
const DECIMALS: u8 = 18;
const MAX_BATCH_SIZE: usize = 100;

#[ink(message)]
pub fn mint(&mut self, amount: Balance) -> Result<()> {
    let new_supply = self.total_supply.saturating_add(amount);
    if new_supply > MAX_SUPPLY {
        return Err(Error::SupplyCapExceeded);
    }
    // ...
}
```

**❌ UNCLEAR**:

```rust
#[ink(message)]
pub fn mint(&mut self, amount: Balance) -> Result<()> {
    if self.total_supply.saturating_add(amount) > 1_000_000_000 {
        return Err(Error::SupplyCapExceeded);
    }
    // What is 1_000_000_000? Why that number?
}
```

### 2. Small, Focused Functions

**✅ GOOD** (single responsibility):

```rust
fn _transfer(&mut self, from: AccountId, to: AccountId, amount: Balance) -> Result<()> {
    self._check_transfer(from, to, amount)?;
    self._update_balances(from, to, amount)?;
    self._emit_transfer_event(from, to, amount);
    Ok(())
}

fn _check_transfer(&self, from: AccountId, to: AccountId, amount: Balance) -> Result<()> {
    // Validation only
}

fn _update_balances(&mut self, from: AccountId, to: AccountId, amount: Balance) -> Result<()> {
    // State updates only
}

fn _emit_transfer_event(&self, from: AccountId, to: AccountId, amount: Balance) {
    // Event emission only
}
```

---

## 📚 Documentation

### 1. Document All Public Functions

```rust
/// Transfer tokens from caller to another account
///
/// # Arguments
/// * `to` - Recipient account
/// * `amount` - Amount to transfer (in token base units)
///
/// # Errors
/// * `InsufficientBalance` - Caller has insufficient balance
/// * `ZeroAddress` - Recipient is zero address
/// * `ZeroAmount` - Amount is zero
///
/// # Events
/// * Emits `Transfer` event on success
#[ink(message)]
pub fn transfer(&mut self, to: AccountId, amount: Balance) -> Result<()> {
    // ...
}
```

---

## 🌐 BelizeChain-Specific Features

### 1. Privacy Commitments (Blake2-256)

```rust
use ink::env::hash::{Blake2x256, HashOutput};

/// Compute salary commitment for privacy-preserving payroll
pub fn compute_salary_commitment(amount: Balance, salt: &[u8]) -> [u8; 32] {
    let mut input = amount.to_le_bytes().to_vec();
    input.extend_from_slice(salt);
    
    let mut output = <Blake2x256 as HashOutput>::Type::default();
    ink::env::hash_bytes::<Blake2x256>(&input, &mut output);
    output
}
```

### 2. Integration with BelizeChain Pallets (Future)

```rust
// When chain extensions become available:

#[ink(message)]
pub fn store_metadata_on_dag(&self, data: Vec<u8>) -> Result<Hash> {
    // Call Pakit chain extension
    // pakit_extension::store_dag_block(data)
    todo!("Requires Pakit chain extension")
}
```

---

## ✅ Final Checklist Before Deploy

- [ ] All tests pass (`cargo test --all`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Linter passes (`cargo clippy`)
- [ ] Security audit checklist completed
- [ ] Documentation complete
- [ ] Events emit for all state changes
- [ ] Access control on admin functions
- [ ] Input validation on all public functions
- [ ] No `unwrap()` or `expect()` in production paths
- [ ] Gas optimization reviewed
- [ ] Deployed to testnet and tested

---

**Last Updated**: February 14, 2026  
**License**: MIT  
**Maintained By**: BelizeChain Team

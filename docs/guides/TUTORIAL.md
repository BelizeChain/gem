# The Gem Tutorial Series 🎓

**Complete step-by-step guides for BelizeChain smart contract development**

---

## 📚 Table of Contents

1. [Tutorial 1: Hello BelizeChain](#tutorial-1-hello-belizechain) (5 minutes)
2. [Tutorial 2: Token Economy](#tutorial-2-token-economy) (15 minutes)
3. [Tutorial 3: NFT Collection](#tutorial-3-nft-collection) (20 minutes)
4. [Tutorial 4: Build a DAO](#tutorial-4-build-a-dao) (30 minutes)
5. [Tutorial 5: NFT Marketplace](#tutorial-5-nft-marketplace) (45 minutes)

**Total Learning Time**: ~2 hours  
**Prerequisites**: Rust, cargo-contract installed, BelizeChain node running

---

## Tutorial 1: Hello BelizeChain

**Goal**: Deploy your first smart contract in 5 minutes!

### Step 1: Create Your Project (1 minute)

```bash
# Create new contract
cargo contract new hello_belize
cd hello_belize

# You'll see this structure:
# hello_belize/
# ├── Cargo.toml
# └── lib.rs
```

### Step 2: Write Your Contract (2 minutes)

Open `lib.rs` and replace with:

```rust
#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod hello_belize {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct HelloBelize {
        /// Stored message
        message: String,
        /// Message history per account
        messages: Mapping<AccountId, String>,
    }

    #[ink(event)]
    pub struct MessageSet {
        #[ink(topic)]
        from: AccountId,
        message: String,
    }

    impl HelloBelize {
        /// Constructor
        #[ink(constructor)]
        pub fn new(init_message: String) -> Self {
            Self {
                message: init_message,
                messages: Mapping::default(),
            }
        }

        /// Set the message
        #[ink(message)]
        pub fn set_message(&mut self, new_message: String) {
            let caller = self.env().caller();
            self.message = new_message.clone();
            self.messages.insert(caller, &new_message);
            
            self.env().emit_event(MessageSet {
                from: caller,
                message: new_message,
            });
        }

        /// Get the message
        #[ink(message)]
        pub fn get_message(&self) -> String {
            self.message.clone()
        }

        /// Get account's last message
        #[ink(message)]
        pub fn get_account_message(&self, account: AccountId) -> Option<String> {
            self.messages.get(&account)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn set_get_works() {
            let mut contract = HelloBelize::new("Hello!".to_string());
            assert_eq!(contract.get_message(), "Hello!");
            
            contract.set_message("Goodbye!".to_string());
            assert_eq!(contract.get_message(), "Goodbye!");
        }
    }
}
```

### Step 3: Add Workspace Declaration

Edit `Cargo.toml` to add after `edition = "2021"`:

```toml
[workspace]
```

### Step 4: Build (1 minute)

```bash
cargo contract build --release

# You'll see:
# ✓ Original wasm size: 25.0K, Optimized: 8.2K
# ✓ Contract artifacts ready in target/ink/
```

### Step 5: Deploy (1 minute)

```bash
# Make sure your node is running!
# In another terminal: belizechain-node --dev --tmp

cargo contract instantiate \
    --constructor new \
    --args "Hello BelizeChain!" \
    --suri //Alice \
    --url ws://localhost:9944 \
    --skip-confirm \
    --execute

# Copy the contract address from output!
```

### Step 6: Interact

```bash
# Set a new message
cargo contract call \
    --contract YOUR_CONTRACT_ADDRESS \
    --message set_message \
    --args "Welcome to Belize!" \
    --suri //Alice \
    --skip-confirm \
    --execute

# Get the message
cargo contract call \
    --contract YOUR_CONTRACT_ADDRESS \
    --message get_message \
    --suri //Alice \
    --dry-run
```

🎉 **Congratulations!** You just deployed your first BelizeChain smart contract!

---

## Tutorial 2: Token Economy

**Goal**: Create your own PSP22 token (like ERC20)

### What You'll Build

A fungible token with:
- Custom name and symbol
- Initial supply and max cap
- Transfer and approval functions
- Minting capability

### Step 1: Create Project

```bash
cargo contract new my_token
cd my_token
```

### Step 2: Update Cargo.toml

Add workspace declaration and dependencies:

```toml
[package]
name = "my_token"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
ink = { version = "5.1.1", default-features = false }

[features]
default = ["std"]
std = ["ink/std"]
```

### Step 3: Implement PSP22 Standard

Replace `lib.rs` with:

```rust
#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod my_token {
    use ink::storage::Mapping;

    #[ink(storage)]
    pub struct MyToken {
        /// Total token supply
        total_supply: Balance,
        /// Maximum supply cap
        max_supply: Balance,
        /// Account balances
        balances: Mapping<AccountId, Balance>,
        /// Spending allowances
        allowances: Mapping<(AccountId, AccountId), Balance>,
        /// Contract owner
        owner: AccountId,
    }

    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        InsufficientBalance,
        InsufficientAllowance,
        MaxSupplyExceeded,
        NotOwner,
    }

    pub type Result<T> = core::result::Result<T, Error>;

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

    impl MyToken {
        /// Create new token with initial supply
        #[ink(constructor)]
        pub fn new(initial_supply: Balance, max_supply: Balance) -> Self {
            let caller = Self::env().caller();
            let mut balances = Mapping::default();
            balances.insert(caller, &initial_supply);

            Self::env().emit_event(Transfer {
                from: None,
                to: Some(caller),
                value: initial_supply,
            });

            Self {
                total_supply: initial_supply,
                max_supply,
                balances,
                allowances: Mapping::default(),
                owner: caller,
            }
        }

        /// Get token name
        #[ink(message)]
        pub fn name(&self) -> String {
            "My Token".to_string()
        }

        /// Get token symbol
        #[ink(message)]
        pub fn symbol(&self) -> String {
            "MTK".to_string()
        }

        /// Get decimals
        #[ink(message)]
        pub fn decimals(&self) -> u8 {
            12
        }

        /// Get total supply
        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        /// Get balance
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balances.get(&owner).unwrap_or(0)
        }

        /// Transfer tokens
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let from = self.env().caller();
            self.transfer_from_to(from, to, value)
        }

        /// Approve spending
        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, value: Balance) -> Result<()> {
            let owner = self.env().caller();
            self.allowances.insert((owner, spender), &value);

            self.env().emit_event(Approval {
                owner,
                spender,
                value,
            });

            Ok(())
        }

        /// Get allowance
        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.allowances.get((owner, spender)).unwrap_or(0)
        }

        /// Transfer from approved account
        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) -> Result<()> {
            let caller = self.env().caller();
            let allowance = self.allowance(from, caller);
            
            if allowance < value {
                return Err(Error::InsufficientAllowance);
            }

            self.transfer_from_to(from, to, value)?;
            self.allowances.insert((from, caller), &(allowance - value));

            Ok(())
        }

        /// Mint new tokens (owner only)
        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, value: Balance) -> Result<()> {
            if self.env().caller() != self.owner {
                return Err(Error::NotOwner);
            }

            let new_supply = self.total_supply.saturating_add(value);
            if new_supply > self.max_supply {
                return Err(Error::MaxSupplyExceeded);
            }

            self.total_supply = new_supply;
            let balance = self.balance_of(to);
            self.balances.insert(to, &(balance.saturating_add(value)));

            self.env().emit_event(Transfer {
                from: None,
                to: Some(to),
                value,
            });

            Ok(())
        }

        /// Internal transfer
        fn transfer_from_to(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) -> Result<()> {
            let from_balance = self.balance_of(from);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert(from, &(from_balance - value));
            let to_balance = self.balance_of(to);
            self.balances.insert(to, &(to_balance.saturating_add(value)));

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value,
            });

            Ok(())
        }
    }
}
```

### Step 4: Build and Test

```bash
# Build
cargo contract build --release

# Deploy
cargo contract instantiate \
    --constructor new \
    --args 1000000000000000 2000000000000000 \
    --suri //Alice \
    --skip-confirm \
    --execute

# Transfer tokens
cargo contract call \
    --contract YOUR_ADDRESS \
    --message transfer \
    --args 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty 100000000000000 \
    --suri //Alice \
    --skip-confirm \
    --execute

# Check balance
cargo contract call \
    --contract YOUR_ADDRESS \
    --message balance_of \
    --args 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty \
    --dry-run
```

🎉 **Success!** You've created a fully functional token!

---

## Tutorial 3: NFT Collection

**Goal**: Create and mint NFTs with metadata

### What You'll Build

A PSP34 NFT contract with:
- Unique token IDs
- IPFS metadata URIs
- Transfer and approval system
- Minting and burning

### Step 1: Create Project

```bash
cargo contract new my_nft
cd my_nft
```

### Step 2: Implement PSP34 (Abbreviated)

Key storage structure:

```rust
#[ink(storage)]
pub struct MyNft {
    /// Token owners
    token_owner: Mapping<u32, AccountId>,
    /// Owner's token count
    owned_tokens_count: Mapping<AccountId, u32>,
    /// Token metadata URIs
    token_uri: Mapping<u32, String>,
    /// Next token ID
    next_token_id: u32,
    owner: AccountId,
}
```

Key functions to implement:

```rust
// Mint NFT
#[ink(message)]
pub fn mint(&mut self, to: AccountId, uri: String) -> Result<u32> {
    // Check owner
    // Assign token ID
    // Store metadata
    // Update counts
    // Emit event
}

// Transfer NFT
#[ink(message)]
pub fn transfer(&mut self, to: AccountId, token_id: u32) -> Result<()> {
    // Verify ownership
    // Update mappings
    // Emit event
}

// Get token URI
#[ink(message)]
pub fn token_uri(&self, token_id: u32) -> Option<String> {
    self.token_uri.get(&token_id)
}
```

See `gem/beli_nft/lib.rs` for complete implementation!

### Step 3: Deploy and Mint

```bash
# Deploy
cargo contract instantiate \
    --constructor new \
    --args "My NFT Collection" "MNFT" \
    --suri //Alice \
    --skip-confirm \
    --execute

# Mint NFT with IPFS URI
cargo contract call \
    --contract YOUR_ADDRESS \
    --message mint \
    --args 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty "ipfs://QmYourHash" \
    --suri //Alice \
    --skip-confirm \
    --execute
```

🎉 **Awesome!** You've minted your first NFT!

---

## Tutorial 4: Build a DAO

**Goal**: Create a governance contract with voting

### What You'll Build

A DAO contract with:
- Proposal creation
- Voting mechanism
- Quorum requirements
- Execution after passing

### Step 1: Define Proposal Structure

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(StorageLayout))]
pub struct Proposal {
    pub proposer: AccountId,
    pub description: String,
    pub votes_for: u128,
    pub votes_against: u128,
    pub end_block: BlockNumber,
    pub executed: bool,
    pub status: ProposalStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "std", derive(StorageLayout))]
pub enum ProposalStatus {
    Pending,
    Active,
    Passed,
    Rejected,
    Executed,
}
```

### Step 2: Core DAO Functions

```rust
/// Create proposal
#[ink(message)]
pub fn create_proposal(&mut self, description: String) -> Result<u32> {
    let caller = self.env().caller();
    let proposal_id = self.next_proposal_id;
    let end_block = self.env().block_number() + self.voting_period;

    let proposal = Proposal {
        proposer: caller,
        description,
        votes_for: 0,
        votes_against: 0,
        end_block,
        executed: false,
        status: ProposalStatus::Active,
    };

    self.proposals.insert(proposal_id, &proposal);
    self.next_proposal_id += 1;

    Ok(proposal_id)
}

/// Vote on proposal
#[ink(message)]
pub fn vote(&mut self, proposal_id: u32, support: bool) -> Result<()> {
    let caller = self.env().caller();
    
    // Check not already voted
    // Get voting weight
    // Update vote counts
    // Emit event

    Ok(())
}

/// Execute passed proposal
#[ink(message)]
pub fn execute_proposal(&mut self, proposal_id: u32) -> Result<()> {
    // Check passed
    // Check quorum
    // Execute action
    // Mark executed

    Ok(())
}
```

### Step 3: Integration with Token Voting

```rust
// Get voting weight from token balance
fn get_voting_power(&self, account: AccountId) -> u128 {
    if let Some(token_address) = self.dalla_token {
        // Cross-contract call to get balance
        // Use balance as voting weight
    } else {
        1 // Default: 1 vote per account
    }
}
```

See `gem/simple_dao/lib.rs` for complete implementation!

---

## Tutorial 5: NFT Marketplace

**Goal**: Build a complete marketplace with royalties

### What You'll Build

A marketplace that:
- Lists NFTs for sale
- Handles DALLA payments
- Pays royalties to creators
- Uses escrow for safety

### Architecture

```
┌─────────────────────────────────────────────┐
│              Marketplace Contract           │
├─────────────────────────────────────────────┤
│                                             │
│  create_listing(nft_id, price, royalty)    │
│           ↓                                 │
│  NFT transferred to marketplace             │
│           ↓                                 │
│  purchase(listing_id)                       │
│           ↓                                 │
│  Buyer pays DALLA                           │
│           ↓                                 │
│  Royalty paid to creator                    │
│           ↓                                 │
│  Remaining DALLA to seller                  │
│           ↓                                 │
│  NFT transferred to buyer                   │
│                                             │
└─────────────────────────────────────────────┘
```

### Step 1: Listing Structure

```rust
#[derive(Debug, Clone)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub struct Listing {
    pub seller: AccountId,
    pub nft_contract: AccountId,
    pub token_id: u32,
    pub price: Balance,
    pub royalty_recipient: AccountId,
    pub royalty_bps: u32, // Basis points (100 = 1%)
    pub active: bool,
}
```

### Step 2: Create Listing

```rust
#[ink(message)]
pub fn create_listing(
    &mut self,
    nft_contract: AccountId,
    token_id: u32,
    price: Balance,
    royalty_bps: u32,
) -> Result<u32> {
    let caller = self.env().caller();

    // Step 1: Verify caller owns the NFT
    let owner = self.get_nft_owner(nft_contract, token_id)?;
    if owner != caller {
        return Err(Error::NotOwner);
    }

    // Step 2: Transfer NFT to marketplace (escrow)
    self.transfer_nft_to_marketplace(nft_contract, token_id)?;

    // Step 3: Create listing
    let listing_id = self.next_listing_id;
    let listing = Listing {
        seller: caller,
        nft_contract,
        token_id,
        price,
        royalty_recipient: caller, // Creator = seller initially
        royalty_bps,
        active: true,
    };

    self.listings.insert(listing_id, &listing);
    self.next_listing_id += 1;

    Ok(listing_id)
}
```

### Step 3: Purchase with Royalties

```rust
#[ink(message)]
pub fn purchase(&mut self, listing_id: u32) -> Result<()> {
    let buyer = self.env().caller();
    let listing = self.get_listing(listing_id)?;

    if !listing.active {
        return Err(Error::ListingNotActive);
    }

    // Calculate royalty
    let royalty = (listing.price * listing.royalty_bps as u128) / 10000;
    let seller_amount = listing.price - royalty;

    // Step 1: Transfer DALLA from buyer to marketplace
    self.transfer_dalla_from_buyer(buyer, listing.price)?;

    // Step 2: Pay royalty to creator
    if royalty > 0 {
        self.transfer_dalla(listing.royalty_recipient, royalty)?;
    }

    // Step 3: Pay seller
    self.transfer_dalla(listing.seller, seller_amount)?;

    // Step 4: Transfer NFT to buyer
    self.transfer_nft_to_buyer(
        listing.nft_contract,
        listing.token_id,
        buyer,
    )?;

    // Step 5: Mark listing as sold
    let mut updated_listing = listing;
    updated_listing.active = false;
    self.listings.insert(listing_id, &updated_listing);

    Ok(())
}
```

### Step 4: Cross-Contract Calls

```rust
// Transfer DALLA tokens
fn transfer_dalla(&self, to: AccountId, amount: Balance) -> Result<()> {
    use ink::env::call::{build_call, ExecutionInput, Selector};

    build_call::<Environment>()
        .call(self.dalla_contract)
        .gas_limit(0) // Use all available gas
        .exec_input(
            ExecutionInput::new(Selector::new(ink::selector_bytes!("transfer")))
                .push_arg(to)
                .push_arg(amount)
        )
        .returns::<Result<()>>()
        .invoke()
}

// Transfer NFT
fn transfer_nft_to_buyer(
    &self,
    nft_contract: AccountId,
    token_id: u32,
    buyer: AccountId,
) -> Result<()> {
    use ink::env::call::{build_call, ExecutionInput, Selector};

    build_call::<Environment>()
        .call(nft_contract)
        .gas_limit(0)
        .exec_input(
            ExecutionInput::new(Selector::new(ink::selector_bytes!("transfer")))
                .push_arg(buyer)
                .push_arg(token_id)
        )
        .returns::<Result<()>>()
        .invoke()
}
```

### Step 5: Complete Flow Example

```bash
# 1. Deploy marketplace
cargo contract instantiate \
    --constructor new \
    --args DALLA_ADDRESS NFT_ADDRESS \
    --suri //Alice

# 2. Approve marketplace to transfer NFT
cargo contract call \
    --contract NFT_ADDRESS \
    --message approve \
    --args MARKETPLACE_ADDRESS 1 \
    --suri //Alice

# 3. Create listing (10 DALLA, 5% royalty)
cargo contract call \
    --contract MARKETPLACE_ADDRESS \
    --message create_listing \
    --args NFT_ADDRESS 1 10000000000000 500 \
    --suri //Alice

# 4. Buyer approves DALLA spending
cargo contract call \
    --contract DALLA_ADDRESS \
    --message approve \
    --args MARKETPLACE_ADDRESS 10000000000000 \
    --suri //Bob

# 5. Purchase NFT
cargo contract call \
    --contract MARKETPLACE_ADDRESS \
    --message purchase \
    --args 0 \
    --suri //Bob
```

🎉 **Congratulations!** You've built a complete NFT marketplace!

---

## 🎓 What's Next?

### Advanced Topics
- **Testing**: Write comprehensive unit and integration tests
- **Gas Optimization**: Reduce transaction costs
- **Upgradability**: Proxy patterns for contract upgrades
- **Security**: Reentrancy guards, access control

### Resources
- 📖 [API Reference](API_REFERENCE.md) - Complete function docs
- 🔗 [Integration Guide](INTEGRATION_GUIDE.md) - Cross-contract patterns
- 🚀 [Quick Start](QUICK_START.md) - 5-minute setup
- 💬 [Discord Community](#) - Get help

### Example Contracts
All tutorial code is available in `gem/`:
- `hello-belizechain/` - Tutorial 1
- `dalla_token/` - Tutorial 2
- `beli_nft/` - Tutorial 3
- `simple_dao/` - Tutorial 4
- `marketplace/` - Tutorial 5 (in development)

---

## 💡 Tips & Best Practices

### 1. Always Use Saturating Arithmetic
```rust
// ✅ GOOD
let new_balance = balance.saturating_add(amount);

// ❌ BAD (can panic!)
let new_balance = balance + amount;
```

### 2. Emit Events for All State Changes
```rust
self.env().emit_event(Transfer {
    from: Some(sender),
    to: Some(recipient),
    value: amount,
});
```

### 3. Check Inputs First
```rust
// Validate BEFORE state changes
if amount == 0 {
    return Err(Error::ZeroAmount);
}

if balance < amount {
    return Err(Error::InsufficientBalance);
}

// Then update state
self.balances.insert(account, &(balance - amount));
```

### 4. Use Result Types
```rust
pub type Result<T> = core::result::Result<T, Error>;

#[ink(message)]
pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
    // ...
}
```

### 5. Test Everything
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[ink::test]
    fn transfer_works() {
        let mut contract = MyToken::new(1000);
        assert!(contract.transfer(accounts.bob, 100).is_ok());
        assert_eq!(contract.balance_of(accounts.bob), 100);
    }
}
```

---

## 🐛 Troubleshooting

### Build Fails: "workspace" Error
**Solution**: Add `[workspace]` to Cargo.toml

### "Cannot find selector" Error
**Solution**: Function name mismatch in cross-contract call

### Gas Limit Exceeded
**Solution**: Increase `--gas` or optimize contract

### Contract Not Found
**Solution**: Check contract address and network connection

---

## 🎉 Congratulations!

You've completed the Gem tutorial series! You now know how to:

✅ Create and deploy smart contracts  
✅ Build fungible tokens (PSP22)  
✅ Create NFT collections (PSP34)  
✅ Implement DAO governance  
✅ Build complex marketplaces  

**Welcome to the BelizeChain developer community!** 💎🇧🇿

---

**Last Updated**: November 4, 2025  
**Version**: 1.0.0  
**License**: MIT

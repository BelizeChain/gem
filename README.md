# 💎 The GEM (General Ecosystem Machinery)

> **Production-grade smart contract ecosystem for BelizeChain**  
> ink! 5.0 | Substrate Contracts | DeFi Primitives

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![ink! 5.0](https://img.shields.io/badge/ink!-5.0-blue)](https://use.ink/)
[![BelizeChain](https://img.shields.io/badge/BelizeChain-v0.2.0--alpha-brightgreen)](https://github.com/BelizeChain/belizechain)

**The GEM** is BelizeChain's comprehensive smart contract platform powered by ink! and `pallet-contracts`. It provides production-ready contracts, DeFi primitives, security frameworks, and seamless runtime integration.

---

## 🎯 Core Features

### 🔄 BelizeX - Decentralized Exchange
Production-ready Uniswap V2-style automated market maker:
- **Factory Contract**: Creates and manages trading pairs with deterministic addresses
- **Pair Contract**: Constant product AMM (x*y=k) with liquidity pools and TWAP oracle  
- **Router Contract**: User-friendly swap interface with multi-hop routing and slippage protection
- **PSP22 Integration**: Seamless cross-contract token transfers
- **Complete SDK**: BelizeXSDK module with 22 methods (JavaScript + TypeScript)
- **Comprehensive Documentation**: Architecture guides, deployment instructions, and integration examples

📖 **[BelizeX Documentation](docs/belizex/README.md)** | **[Deployment Guide](docs/belizex/DEPLOYMENT.md)** | **[SDK Example](sdk/examples/belizex-swap.js)**

### 🔐 Security & Standards
Enterprise-grade patterns and frameworks:
- **PSP37 Multi-Token** (650 lines): Batch operations with mixed fungible/NFT support
- **Access Control Library** (550 lines): Ownable, RBAC, and Pausable patterns
- **Security Audit Checklist**: 149-point comprehensive audit guide
- **Best Practices Guide**: Gas optimization, storage management, testing strategies
- **Runtime Integration**: MeshNetworkSDK and PrivacySDK for BelizeChain pallets

---

## What is The GEM?

The GEM brings programmable smart contracts to BelizeChain with unique capabilities:

- 💰 Interact with DALLA/bBZD tokens through the Economy pallet
- 🤖 Access AI predictions via Nawal integration (federated learning + genome evolution)
- ⚛️ Execute quantum computations through Kinich (Azure Quantum + post-quantum cryptography)
- 📦 Store data on sovereign DAG storage via Pakit (with quantum-resistant compression)
- 🆔 Verify identities using BelizeID (SSN/Passport + KYC)
- 🏛️ Participate in governance and proposals
- 🌐 Register .bz domains via BNS (Belize Name Service)
- 📡 Access Meshtastic LoRa mesh network for off-grid transactions

---

## 🚀 For Developers

### 📚 Quick Links

- **[Quick Reference](docs/guides/QUICK_REFERENCE.md)** - Fast lookup for all contracts and features
- **[Tutorial Series](docs/guides/TUTORIAL.md)** - 5 step-by-step guides (beginner to advanced)
- **[Contributing Guide](docs/guides/CONTRIBUTING.md)** - How to contribute to the project
- **[Best Practices](docs/guides/BEST_PRACTICES.md)** - Development patterns and optimization
- **[SDK Documentation](docs/sdk/README.md)** - JavaScript/TypeScript SDK

### 💧 Get Test Tokens

**Testnet Faucet**: Available (contract built and ready for deployment)
- **Drip Amount**: 1000 DALLA per claim
- **Cooldown**: 100 blocks (~10 minutes)
- **Usage**: `gem faucet claim` or use the [SDK](docs/sdk/README.md)

### 📦 Install SDK

```bash
npm install @belizechain/gem-sdk @polkadot/api @polkadot/api-contract
```

```javascript
const { GemSDK } = require('@belizechain/gem-sdk');

const sdk = new GemSDK('ws://localhost:9944');
await sdk.connect();

// Transfer 100 DALLA
await sdk.dallaTransfer(
    '5GD4w5...NVsNB',
    alice,
    bob.address,
    100000000000000
);
```

See [SDK examples](sdk/examples/) for complete code samples.

### 🎓 Learning Path

1. **Beginner** (30 minutes): Start with [Quick Reference](docs/guides/QUICK_REFERENCE.md) → Understand all features
2. **Intermediate** (2 hours): Follow [Tutorial Series](docs/guides/TUTORIAL.md) → Build tokens, NFTs, DAOs
3. **Advanced** (ongoing): Read [Best Practices](docs/guides/BEST_PRACTICES.md) → Optimization and patterns

### 📊 Production Contracts

| Contract | Address | Size | Status |
|----------|---------|------|--------|
| **DALLA Token (PSP22)** | `5GD4w5...NVsNB` | 10.5 KB | ✅ Live |
| **BeliNFT (PSP34)** | `5Ho6Ks...iFQL7` | 14.9 KB | ✅ Live |
| **Simple DAO** | TBD | 12.9 KB | 🟡 Built |
| **Faucet** | TBD | 7.5 KB | 🟡 Built |

### 🌐 Community

- **Discord**: [https://discord.gg/belizechain](https://discord.gg/belizechain)
- **GitHub**: [https://github.com/BelizeChain/gem](https://github.com/BelizeChain/gem)
- **Main Project**: [https://github.com/BelizeChain/belizechain](https://github.com/BelizeChain/belizechain)
- **Forum**: [https://forum.belizechain.org](https://forum.belizechain.org)

---

## Quick Start

### Prerequisites

1. **Install cargo-contract** (ink! contract development tool):
```bash
cargo install cargo-contract --force
```

2. **Build BelizeChain** with GEM support:
```bash
cd /home/wicked/BelizeChain/belizechain
cargo build --release
```

### Create Your First Contract

```bash
cd gem
cargo contract new my_first_contract
cd my_first_contract
cargo contract build
```

### Deploy to BelizeChain

```bash
# Start local node
./target/release/belizechain-node --dev --tmp

# In another terminal, deploy your contract
cargo contract instantiate \
    --constructor new \
    --args "Hello BelizeChain" \
    --suri //Alice \
    --url ws://localhost:9944
```

## Example Contracts

### 1. Hello BelizeChain
Simple counter contract demonstrating basic storage and operations.

```rust
#![cfg_attr(not(feature = "std"), no_std)]

#[ink::contract]
mod hello_belizechain {
    #[ink(storage)]
    pub struct HelloBelizeChain {
        message: String,
        counter: u32,
    }

    impl HelloBelizeChain {
        #[ink(constructor)]
        pub fn new(message: String) -> Self {
            Self { message, counter: 0 }
        }

        #[ink(message)]
        pub fn get_message(&self) -> String {
            self.message.clone()
        }

        #[ink(message)]
        pub fn increment(&mut self) {
            self.counter += 1;
        }

        #[ink(message)]
        pub fn get_counter(&self) -> u32 {
            self.counter
        }
    }
}
```

### 2. BelizeToken (ERC20)
Standard fungible token implementation.

**Status**: DALLA token (PSP22 standard) is complete and deployed. See `dalla_token/` for implementation.

### 3. Nawal AI Oracle
Contract that queries AI predictions from the Nawal federated learning network.
- **Architecture**: 117M parameter BelizeChainLLM with genome evolution
- **Privacy**: Differential privacy (DP-SGD) for national datasets
- **Status**: In development. Requires chain extension integration.

### 4. Kinich Quantum Lottery
Lottery using quantum random number generation via Azure Quantum.
- **Backends**: Azure Quantum, IBM Quantum, ionq, Rigetti
- **Security**: Post-quantum cryptography (CRYSTALS-Dilithium, CRYSTALS-Kyber)
- **Status**: In development. Requires chain extension integration.

## Contract Limits

| Parameter | Value | Description |
|-----------|-------|-------------|
| Max Code Size | 256 KB | Maximum WASM bytecode size |
| Max Storage Key | 128 bytes | Maximum storage key length |
| Deposit per Item | 1 DALLA | Cost per storage item |
| Deposit per Byte | 0.0001 DALLA | Cost per byte of storage |
| Default Deposit Limit | 1000 DALLA | Maximum deposit for instantiation |
| Call Stack Depth | 5 | Maximum nested contract calls |
| Transient Storage | 256 KB | Temporary storage per call |

## Architecture

The Gem integrates with BelizeChain's runtime through:

1. **pallet-contracts**: Substrate's battle-tested smart contract execution engine (Polkadot SDK stable2512)
2. **Cross-Pallet Calls**: Direct integration with all 15 BelizeChain pallets
3. **Chain Extensions**: Custom APIs for Nawal, Kinich, and Pakit (in development)
4. **Post-Quantum Ready**: Prepared for quantum-resistant cryptographic upgrades

## Development Status

### ✅ Completed
- **Smart Contract Runtime**: pallet-contracts integration (Polkadot SDK stable2512)
- **Token Standards**:
  - DALLA Token (PSP22 standard) - 459 lines, production-ready
  - BeliNFT (PSP34 standard) - 649 lines, production-ready
  - **PSP37 Multi-Token** - ✨ NEW: 650 lines, batch operations, mixed fungible/NFT support
- **Access Control Library** - ✨ NEW: Ownable, AccessControl (RBAC), Pausable patterns (550 lines)
- **Governance & Utilities**:
  - Simple DAO governance contract - 536 lines, production-ready
  - Faucet contract for testnet - 327 lines, production-ready
  - Hello BelizeChain example - 309 lines, production-ready
- **Developer Tools**:
  - JavaScript/TypeScript SDK with full contract support (7 modules)
  - MeshNetworkSDK and PrivacySDK for BelizeChain integrations
  - Comprehensive tutorial series (5 guides)
- **Documentation** - ✨ NEW:
  - Security Audit Checklist (149 points)
  - Best Practices Guide (gas optimization, testing, security patterns)
  - 12-week Extension Roadmap (GEM → production-grade platform)
- **Runtime Integration**: All 16 BelizeChain pallets (Economy, Identity, Governance, Compliance, Staking, Oracle, Payroll, Interoperability, BelizeX, LandLedger, Consensus, Quantum, Community, BNS, Mesh, Contracts)

### 🚧 In Development
- **Chain Extensions**: Advanced integrations for Nawal AI (federated learning), Kinich Quantum (quantum RNG), Pakit Storage (DAG-based storage), and Mesh Network (LoRa relay)
- **Additional DeFi Primitives**: Lending protocols, staking contracts, yield farming
- **GEM CLI Tool**: Professional scaffolding and deployment tool (`gem contract init`, `gem belizex deploy`)
- **Contract Templates Library**: Production-ready templates for common patterns (multisig, vesting, marketplace, gaming)
- **Advanced SDK Features**: Contract upgrade helpers, migration tools, gas optimization utilities

### 📋 Roadmap
- Enhanced testnet deployment tools and contract verification
- Interactive documentation portal with live code examples
- Cross-contract design patterns and composition techniques
- Integration guides for all BelizeChain pallets (Economy, Governance, Identity, etc.)

## Resources

- **ink! Documentation**: https://use.ink/
- **cargo-contract Guide**: https://github.com/paritytech/cargo-contract
- **Substrate Contracts**: https://docs.substrate.io/tutorials/smart-contracts/
- **BelizeChain Developer Hub**: https://github.com/BelizeChain/belizechain

## Support

Join our developer community:
- Discord: https://discord.gg/belizechain
- Forum: https://forum.belizechain.org
- GitHub Issues: https://github.com/BelizeChain/gem/issues

---

**Built with 💎 for the sovereign nation of Belize**

# 💎 The Gem - BelizeChain Smart Contracts

**The Gem** is BelizeChain's smart contract platform powered by ink! and `pallet-contracts`.

## What is The Gem?

The Gem brings programmable smart contracts to BelizeChain, enabling developers to build decentralized applications that can:

- 💰 Interact with DALLA/bBZD tokens through the Economy pallet
- 🤖 Access AI predictions via Nawal integration (federated learning + genome evolution)
- ⚛️ Execute quantum computations through Kinich (Azure Quantum + post-quantum cryptography)
- 📦 Store data on IPFS/Arweave via Pakit (with quantum-resistant compression)
- 🆔 Verify identities using BelizeID (SSN/Passport + KYC)
- 🏛️ Participate in governance and proposals
- 🌐 Register .bz domains via BNS (Belize Name Service)

---

## 🚀 For Developers

### 📚 Quick Links

- **[5-Minute Quick Start](QUICK_START.md)** - Get your first contract deployed
- **[Tutorial Series](TUTORIAL.md)** - 5 step-by-step guides (beginner to advanced)
- **[API Reference](API_REFERENCE.md)** - Complete function documentation with gas estimates
- **[Integration Guide](INTEGRATION_GUIDE.md)** - Cross-contract patterns and best practices
- **[SDK Documentation](sdk/README.md)** - JavaScript/TypeScript SDK

### 💧 Get Test Tokens

**Testnet Faucet**: Available (contract built and ready for deployment)
- **Drip Amount**: 1000 DALLA per claim
- **Cooldown**: 100 blocks (~10 minutes)
- **Usage**: `gem faucet claim` or use the [SDK](sdk/README.md)

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

1. **Beginner** (1 hour): Start with [Quick Start](QUICK_START.md) → Deploy your first contract
2. **Intermediate** (2 hours): Follow [Tutorial Series](TUTORIAL.md) → Build tokens, NFTs, DAOs
3. **Advanced** (ongoing): Read [Integration Guide](INTEGRATION_GUIDE.md) → Cross-contract patterns

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

2. **Build BelizeChain** with Spike support:
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
- pallet-contracts integration in runtime (Polkadot SDK stable2512)
- DALLA Token (PSP22 standard) - 459 lines, production-ready
- BeliNFT (PSP34 standard) - 649 lines, production-ready
- Simple DAO governance contract - 536 lines, production-ready
- Faucet contract for testnet - 327 lines, production-ready
- Hello BelizeChain example contract - 309 lines, production-ready
- JavaScript/TypeScript SDK with full contract support
- Comprehensive tutorial series and documentation
- Integration with all 15 BelizeChain pallets (Economy, Identity, Governance, Compliance, Staking, Oracle, Payroll, Interoperability, BelizeX, LandLedger, Consensus, Quantum, Community, BNS, Contracts)

### 🚧 In Progress
- Chain extensions for Nawal AI (federated learning inference), Kinich Quantum (quantum RNG + PQC), and Pakit Storage (IPFS/Arweave)
- CLI wrapper around cargo-contract for improved developer experience
- Additional example contracts (marketplace, lending, staking)
- Post-quantum cryptography integration (CRYSTALS-Dilithium signatures, CRYSTALS-Kyber key exchange)

### 📋 Planned
- Testnet contract deployment and verification
- Documentation portal with interactive examples
- Contract upgrade patterns and migration tools
- Advanced cross-contract integration patterns

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

# The Gem - Quick Reference

## 🚀 Quick Start

### For Users
```bash
# Install SDK
npm install @belizechain/gem-sdk

# Use in your project
const { GemSDK } = require('@belizechain/gem-sdk');
const sdk = new GemSDK('ws://localhost:9944');
await sdk.connect();
```

### For Developers
```bash
# Clone repository
git clone https://github.com/BelizeChain/gem.git
cd gem

# Install cargo-contract
cargo install cargo-contract --force

# Build all contracts
cargo contract build --release --manifest-path dalla_token/Cargo.toml
cargo contract build --release --manifest-path beli_nft/Cargo.toml
cargo contract build --release --manifest-path simple_dao/Cargo.toml
cargo contract build --release --manifest-path faucet/Cargo.toml
cargo contract build --release --manifest-path hello-belizechain/Cargo.toml

# Or build workspace (requires cargo-contract workspace support)
cargo contract build --release
```

## 📦 What's Included

### Smart Contracts (Rust + ink!)
- **DALLA Token** - PSP22 fungible token (459 lines)
- **BeliNFT** - PSP34 NFT collection (649 lines)
- **Simple DAO** - Governance contract (536 lines)
- **Faucet** - Testnet token distribution (327 lines)
- **Hello BelizeChain** - Example contract (309 lines)

### SDK (JavaScript/TypeScript)
- Complete Polkadot.js wrapper
- TypeScript definitions included
- 5 working examples
- All contract ABIs bundled

### Documentation
- **README.md** - Main documentation
- **TUTORIAL.md** - 5-part tutorial series (~2 hours)
- **CONTRIBUTING.md** - Contribution guidelines
- **CHANGELOG.md** - Version history
- **SDK README** - Complete SDK reference

## 🎯 Key Features

### PSP22 (DALLA Token)
- ✅ Transfer, approve, transferFrom
- ✅ Minting and burning
- ✅ Total supply tracking
- ✅ Allowance management
- ✅ Event emission

### PSP34 (BeliNFT)
- ✅ Mint, transfer, burn
- ✅ Metadata URIs
- ✅ Collection management
- ✅ Enumeration support
- ✅ Approval system

### DAO Governance
- ✅ Proposal creation
- ✅ Token-weighted voting
- ✅ Configurable quorum
- ✅ Execution after pass
- ✅ NFT membership (optional)

### Faucet
- ✅ Configurable drip amount
- ✅ Cooldown period
- ✅ Owner refill
- ✅ Claim tracking
- ✅ Statistics

## 🔗 Important Links

- **GitHub**: https://github.com/BelizeChain/gem
- **Main Project**: https://github.com/BelizeChain/belizechain
- **Discord**: https://discord.gg/belizechain
- **Forum**: https://forum.belizechain.org

## 📊 Project Stats

- **Repository Size**: 308KB (optimized)
- **Total Contracts**: 5
- **Total Code Lines**: 2,280+ (Rust)
- **SDK Size**: 88KB
- **Documentation**: 1,000+ lines
- **License**: MIT

## 🛠️ Development Commands

```bash
# Build contract
cargo contract build --release

# Run tests
cargo test

# Check contract
cargo contract check

# Deploy contract
cargo contract instantiate \
    --constructor new \
    --args "..." \
    --suri //Alice

# Call contract
cargo contract call \
    --contract ADDRESS \
    --message METHOD \
    --args "..." \
    --suri //Alice
```

## 🔐 Security

- No unsafe code
- Saturating arithmetic
- Input validation
- Proper error handling
- Event emission
- No known vulnerabilities

## 📈 Version History

- **v1.0.0** (2026-01-29) - Production release
- **v0.9.0** (2026-01-15) - Beta release
- **v0.1.0** (2025-12-01) - Initial development

## 🎓 Learning Path

1. **Beginner** (1 hour)
   - Read README.md
   - Deploy Hello BelizeChain
   - Run SDK examples

2. **Intermediate** (2 hours)
   - Complete TUTORIAL.md (5 parts)
   - Build custom token
   - Create NFT collection

3. **Advanced** (ongoing)
   - Build DAO
   - Integrate with Nawal/Kinich/Pakit
   - Contribute to project

## 💎 Built for BelizeChain

The Gem is part of BelizeChain's sovereign digital infrastructure for the nation of Belize.

**Main Features Integration:**
- 💰 Economy pallet (DALLA/bBZD dual currency)
- 🤖 Nawal AI (117M parameter federated learning + genome evolution)
- ⚛️ Kinich Quantum (Azure Quantum + post-quantum cryptography)
- 📦 Pakit Storage (IPFS/Arweave + quantum-resistant compression)
- 🆔 BelizeID (SSN/Passport + KYC/AML)
- 🌐 BNS (.bz domains + decentralized hosting)
- 🏛️ Governance (15 custom pallets)

---

**Version**: 1.0.0  
**Status**: Production Ready  
**License**: MIT  
**Maintained By**: BelizeChain Team

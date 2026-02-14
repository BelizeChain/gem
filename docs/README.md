# GEM Documentation

Welcome to The GEM (General Ecosystem Machinery) documentation! This folder contains comprehensive guides, tutorials, and references for developing smart contracts on BelizeChain.

---

## 📚 Documentation Structure

### 🎓 [Guides](guides/)
Essential learning resources and development guides:
- **[Quick Reference](guides/QUICK_REFERENCE.md)** - Fast lookup for all contracts, features, and commands
- **[Tutorial Series](guides/TUTORIAL.md)** - 5 step-by-step tutorials from beginner to advanced
- **[Contributing Guide](guides/CONTRIBUTING.md)** - How to contribute to the GEM project
- **[Best Practices](guides/BEST_PRACTICES.md)** - Development patterns, gas optimization, and security
- **[Security Audit Checklist](guides/SECURITY_AUDIT_CHECKLIST.md)** - 149-point comprehensive security review

### 🔄 [BelizeX Documentation](belizex/)
Complete guide to the BelizeX decentralized exchange:
- **[BelizeX Overview](belizex/README.md)** - Architecture, AMM mathematics, and feature guide
- **[Deployment Guide](belizex/DEPLOYMENT.md)** - Step-by-step deployment instructions
- **[Integration Tests](belizex/INTEGRATION_TESTS.md)** - Testing scenarios and E2E templates

### 📦 [SDK Documentation](sdk/)
JavaScript/TypeScript SDK for BelizeChain smart contracts:
- **[SDK Reference](sdk/README.md)** - Complete API documentation with examples
- **SDK Modules**: GemSDK, BelizeXSDK, MeshNetworkSDK, PrivacySDK

---

## 🚀 Quick Start

**New to GEM?** Start here:
1. Read the [Quick Reference](guides/QUICK_REFERENCE.md) (10 minutes)
2. Follow [Tutorial 1: Hello BelizeChain](guides/TUTORIAL.md#tutorial-1-hello-belizechain) (5 minutes)
3. Deploy your first contract and start building!

**Building a DApp?**
1. Install the [SDK](sdk/README.md)
2. Review [Best Practices](guides/BEST_PRACTICES.md)
3. Check out the [BelizeX integration example](../sdk/examples/belizex-swap.js)

**Contributing?**
1. Read the [Contributing Guide](guides/CONTRIBUTING.md)
2. Review the [Security Audit Checklist](guides/SECURITY_AUDIT_CHECKLIST.md)
3. Follow the coding standards and submit a PR!

---

## 📖 Additional Resources

### Main Repository Documentation
- **[Main README](../README.md)** - Project overview and getting started
- **[CHANGELOG](../CHANGELOG.md)** - Version history and release notes
- **[LICENSE](../LICENSE)** - MIT License

### Contract Documentation
Each contract has its own README:
- **[DALLA Token (PSP22)](../dalla_token/)** - Fungible token implementation
- **[BeliNFT (PSP34)](../beli_nft/)** - NFT collection
- **[PSP37 Multi-Token](../psp37_multi_token/)** - Mixed fungible/NFT tokens
- **[Access Control Library](../access_control/)** - Security patterns
- **[Simple DAO](../simple_dao/)** - Governance contract
- **[Faucet](../faucet/)** - Testnet token distribution

### BelizeX Contracts
- **[Factory](../dex/factory/)** - Trading pair creation
- **[Pair](../dex/pair/)** - AMM liquidity pools
- **[Router](../dex/router/)** - User-friendly swap interface

---

## 🛠️ Development Workflow

```bash
# 1. Build a contract
cd contract_name/
cargo contract build --release

# 2. Run tests
cargo test

# 3. Deploy to local node
cargo contract instantiate \
    --constructor new \
    --args "arg1" "arg2" \
    --suri //Alice \
    --url ws://localhost:9944

# 4. Use with SDK
npm install @belizechain/gem-sdk
node sdk/examples/your-example.js
```

---

## 🎯 Learning Paths

### Path 1: Smart Contract Developer
1. [Quick Reference](guides/QUICK_REFERENCE.md) - Understand the ecosystem
2. [Tutorial Series](guides/TUTORIAL.md) - Build 5 contracts
3. [Best Practices](guides/BEST_PRACTICES.md) - Write production code
4. [Security Checklist](guides/SECURITY_AUDIT_CHECKLIST.md) - Audit your work

### Path 2: DApp Developer
1. [SDK Documentation](sdk/README.md) - Learn the SDK
2. [BelizeX Guide](belizex/README.md) - Understand BelizeX
3. Check SDK examples in `../sdk/examples/`
4. Build your DApp!

### Path 3: Contributor
1. [Contributing Guide](guides/CONTRIBUTING.md) - Understand the process
2. [Best Practices](guides/BEST_PRACTICES.md) - Follow standards
3. Pick an issue and contribute!

---

## 📊 Documentation Stats

- **Total Documentation**: 3,500+ lines across 11+ guides
- **Tutorials**: 5 comprehensive step-by-step guides
- **Security Checklist**: 149 audit points
- **SDK Examples**: 8+ working code examples
- **Contract Templates**: 11 production-ready contracts

---

## 🌐 Community & Support

- **Discord**: [https://discord.gg/belizechain](https://discord.gg/belizechain)
- **Forum**: [https://forum.belizechain.org](https://forum.belizechain.org)
- **GitHub Issues**: [https://github.com/BelizeChain/gem/issues](https://github.com/BelizeChain/gem/issues)
- **Main Project**: [https://github.com/BelizeChain/belizechain](https://github.com/BelizeChain/belizechain)

---

## 📝 Documentation Guidelines

When contributing documentation:
- Use clear, concise language
- Provide working code examples
- Include both happy path and error scenarios
- Link to related documentation
- Keep formatting consistent
- Update this index when adding new docs

---

**Built with 💎 for the sovereign nation of Belize**

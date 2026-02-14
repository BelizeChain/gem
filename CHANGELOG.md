# Changelog

All notable changes to The GEM will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Chain extensions for Nawal AI, Kinich Quantum, and Pakit Storage
- Additional DeFi primitives (lending, staking, yield farming)
- GEM CLI tool for scaffolding and deployment
- Enhanced contract templates library
- Interactive documentation portal

## [1.3.0] - 2026-02-14

### Added
- **BelizeX Decentralized Exchange**: Complete AMM implementation (1,942 lines total)
  - **Factory Contract**: Creates and manages trading pairs (350 lines, 8 tests)
    - One pair per unique token combination with sorted addresses
    - Deterministic pair addresses for predictability
    - Fee recipient management
  - **Pair Contract**: Constant product AMM using x * y = k formula (832 lines, 4 tests)
    - Liquidity provision with LP tokens using √(x*y) formula
    - Token swaps with 0.3% trading fee
    - Time-weighted average price (TWAP) oracle
    - Reentrancy protection and minimum liquidity lock (1000 tokens)
  - **Router Contract**: User-friendly trading interface (760 lines, 4 tests)
    - Slippage protection for safe trading
    - Transaction deadline enforcement
    - Multi-hop routing (e.g., A→B→C swaps)
    - Optimal liquidity calculations
- **PSP22 Cross-Contract Integration**: Enhanced token transfer capabilities
  - `transfer()`, `balance_of()`, `transfer_from()` selectors
  - Safe cross-contract communication with `try_invoke()` pattern
  - Comprehensive error handling
- **BelizeXSDK Module**: JavaScript/TypeScript SDK for exchange interaction (790 lines)
  - 22 methods covering Factory, Pair, and Router operations
  - Full TypeScript definitions (240 lines, 8 interfaces)
  - Price impact calculation and amount formatting utilities
  - Optimized gas limits and contract instance caching
- **BelizeX Documentation Suite**:
  - Complete architectural guide with AMM mathematics (419 lines)
  - Deployment guide with step-by-step instructions (300 lines)
  - Integration test scenarios and E2E templates (150 lines)
  - Complete SDK usage example with swap workflow (250 lines)

### Changed
- Restructured BelizeX as cargo workspace with modular packages (factory/, pair/, router/)
- Updated SDK exports to include BelizeXSDK alongside existing modules
- Enhanced TypeScript definitions in index.d.ts for better IDE support
- Improved workspace organization for better maintainability

## [1.2.0] - 2026-02-10

### Added
- **PSP37 Multi-Token Contract**: Industry-standard multi-token implementation (650 lines, 12 tests)
  - Manages multiple fungible and non-fungible tokens in a single contract
  - Batch transfer operations for gas efficiency
  - Operator approvals for delegated token management
  - Mint/burn functionality with granular access control
  - Token URI metadata support for NFTs
- **Access Control Library**: Reusable security patterns (550 lines)
  - **Ownable**: Single owner pattern with transfer and renounce capabilities
  - **AccessControl**: Role-based permissions (admin, minter, burner, pauser, upgrader)
  - **Pausable**: Emergency stop functionality for critical situations
  - Modular design for easy integration into any contract
- **Security Framework**:
  - Comprehensive Security Audit Checklist (149 critical points)
  - Best Practices Guide covering gas optimization, storage management, security patterns, and testing
  - Code quality standards and documentation guidelines
- **SDK Extensions**:
  - **MeshNetworkSDK** (`sdk/meshNetwork.js`): Full pallet-belize-mesh integration
    - Node registration for all types (Client, Router, Gateway, ValidatorRelay, EmergencyBeacon)
    - LoRa transaction relay with 87-byte compressed format
    - Relay mining rewards tracking and statistics
    - District-based node discovery
    - Meshtastic hardware support (T-Beam, Heltec V3, RAK WisBlock, Station G2)
  - **PrivacySDK** (`sdk/privacy.js`): Privacy-preserving utilities
    - Salary and payment commitment computation (pallet-belize-payroll)
    - Vote commitments for future DAO privacy features
    - Computation commitments for Proof-of-Useful-Work staking
    - Batch operations and validation utilities
- **SDK Examples**:
  - `examples/mesh-network.js` - Complete mesh node workflow demonstration
  - `examples/privacy-payroll.js` - Privacy-preserving payroll system demo

### Changed
- Updated documentation to reflect BelizeChain v0.2.0+ runtime changes
- Improved integration references: Pakit now uses DAG-based sovereign storage
- Enhanced mesh network capabilities with pallet-belize-mesh integration  
- Reorganized SDK modules for better discoverability

### Fixed
- Removed deprecated `.gas_limit()` calls for ink! 5.0 compatibility
- Updated cross-contract call patterns to use ink! 5.0 `build_call` API
- Fixed documentation comment syntax throughout codebase
- Corrected TypeScript type definitions for better IDE support

### Notes
- All contracts remain fully compatible with BelizeChain v0.2.0+ runtime
- Commitment hash support provides forward compatibility with privacy features
- No breaking changes to existing contract APIs or SDK interfaces

## [1.0.0] - 2026-01-29

### Added
- **DALLA Token (PSP22)**: Complete fungible token implementation with minting, burning, and allowances
- **BeliNFT (PSP34)**: Full-featured NFT contract with metadata support and enumeration
- **Simple DAO**: Governance contract with proposal creation and token-weighted voting
- **Faucet**: Testnet token distribution with configurable drip amounts and cooldowns
- **Hello BelizeChain**: Example contract demonstrating basic storage and events
- **JavaScript/TypeScript SDK**: Complete SDK with type definitions and examples
- **Tutorial Series**: 5 comprehensive tutorials from beginner to advanced
- **Documentation**: README, TUTORIAL, API reference, and integration guides
- **pallet-contracts Integration**: Full Substrate contracts pallet integration in BelizeChain runtime

### Infrastructure
- MIT License
- Contributing guidelines
- Proper .gitignore for Rust and Node.js
- CI/CD ready structure
- Comprehensive test coverage

### Security
- Saturating arithmetic throughout
- Input validation on all external calls
- Proper error handling
- Event emission for state changes
- No unsafe code blocks

## [0.9.0] - 2026-01-15

### Added
- Initial contract implementations
- Basic SDK structure
- Documentation framework

### Changed
- Migrated from Spike to The Gem branding
- Updated all contract comments and documentation

## [0.1.0] - 2025-12-01

### Added
- Project initialization
- Contract scaffolding
- Development environment setup

---

[1.0.0]: https://github.com/BelizeChain/gem/releases/tag/v1.0.0
[0.9.0]: https://github.com/BelizeChain/gem/releases/tag/v0.9.0
[0.1.0]: https://github.com/BelizeChain/gem/releases/tag/v0.1.0

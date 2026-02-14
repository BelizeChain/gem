# Security Audit Checklist for ink! Smart Contracts 🔒

**Version**: 1.0  
**Last Updated**: February 14, 2026  
**Platform**: BelizeChain GEM / ink! 4.0  

This comprehensive checklist covers security best practices for auditing ink! smart contracts on BelizeChain. Use this before deploying contracts to production.

---

## ✅ Pre-Audit Preparation

- [ ] Contract compiles without errors (`cargo contract build`)
- [ ] All tests pass (`cargo test --all`)
- [ ] Code formatted (`cargo fmt --check`)
- [ ] Linter passes (`cargo clippy --all-targets -- -D warnings`)
- [ ] Documentation complete (all public functions documented)
- [ ] Git commit history clean (no sensitive data, keys, or secrets)

---

## 🔐 Access Control & Authorization

### Ownership & Admin Functions
- [ ] **Owner initialization**: Contract has explicit owner set in constructor
- [ ] **Owner validation**: All admin functions check `ensure_owner()` or equivalent
- [ ] **Transfer ownership**: Includes `transfer_ownership()` with proper validation
- [ ] **Renounce ownership**: Includes `renounce_ownership()` if applicable
- [ ] **Zero address check**: New owner cannot be zero address `AccountId::from([0u8; 32])`
- [ ] **Event emission**: `OwnershipTransferred` event emitted on ownership changes

### Role-Based Access Control (RBAC)
- [ ] **Role definitions**: Roles clearly defined as constants (ADMIN, MINTER, BURNER, etc.)
- [ ] **Role checks**: All protected functions use `ensure_role()` or `has_role()`
- [ ] **Role admins**: Each role has defined admin role (who can grant/revoke)
- [ ] **DEFAULT_ADMIN_ROLE**: Super admin role properly configured
- [ ] **Role events**: `RoleGranted` and `RoleRevoked` events emit correctly
- [ ] **Renounce role**: Users can renounce own roles (not others)

### Function Modifiers
- [ ] **Caller validation**: All state-changing functions validate `self.env().caller()`
- [ ] **Approval checks**: Transfer-from functions check allowances/approvals
- [ ] **Pausable**: Critical functions respect pause state if `Pausable` is used
- [ ] **Reentrancy guard**: Cross-contract calls protected (ReentrancyGuard if needed)

---

## 💰 Token Economics & Balance Management

### PSP22 / PSP34 / PSP37 Compliance
- [ ] **Standard compliance**: Implements all required PSP22/34/37 functions
- [ ] **Transfer validation**: Cannot transfer to zero address
- [ ] **Balance checks**: All transfers check sufficient balance
- [ ] **Approval flows**: `approve()`, `transfer_from()`, `allowance()` work correctly
- [ ] **Event emission**: `Transfer`, `Approval` events emit for all state changes
- [ ] **Metadata**: Token name, symbol, decimals set correctly (PSP22)

### Arithmetic Safety
- [ ] **No unchecked math**: All arithmetic uses saturating operations
  - `saturating_add()`, `saturating_sub()`, `saturating_mul()`
  - Avoid `+`, `-`, `*` operators (can panic!)
- [ ] **Overflow protection**: No possibility of overflow/underflow
- [ ] **checked_add** for critical operations**: Returns `Option<T>` and handles `None`
- [ ] **Division by zero**: All divisions check for zero divisor
- [ ] **Precision loss**: Checked for division operations with small numbers

### Supply Management
- [ ] **Initial supply**: Total supply set correctly in constructor
- [ ] **Mint function**: Increases total supply atomically
- [ ] **Burn function**: Decreases total supply atomically
- [ ] **Supply cap**: Enforced if contract has max supply limit
- [ ] **Supply tracking**: `total_supply()` always equals sum of all balances

---

## 🔄 State Management & Storage

### Storage Patterns
- [ ] **Mapping usage**: `Mapping<K, V>` used for unbounded collections
- [ ] **Vec limits**: `Vec<T>` size limited (prevents gas exhaustion)
- [ ] **Lazy loading**: Large storage uses `ink::storage::Lazy` where appropriate
- [ ] **Default values**: `Mapping::get()` handles missing keys (returns `Option` or default)
- [ ] **Storage clearing**: Removed items properly deleted (`Mapping::remove()`)

### State Consistency
- [ ] **Atomic updates**: Related state changes grouped together
- [ ] **Check-Effects-Interactions**: Pattern followed (check conditions, update state, then external call)
- [ ] **State reversion**: Early returns prevent partial state updates
- [ ] **Invariants**: Contract invariants documented and maintained
  - Example: `total_supply == sum(all_balances)`

### Cross-Contract Calls
- [ ] **Reentrancy protection**: State updated BEFORE cross-contract calls
- [ ] **Call result handling**: External call results checked (don't ignore `Result`)
- [ ] **Gas limits**: Cross-contract calls specify explicit gas limits
- [ ] **Trusted contracts**: Only call known, audited contracts
- [ ] **Call failure handling**: Proper error handling for failed cross-contract calls

---

## 🚨 Input Validation & Error Handling

### Function Parameters
- [ ] **Zero checks**: Amount/value parameters reject zero if invalid
- [ ] **Address validation**: No transfers/approvals to zero address
- [ ] **Array lengths**: Batch operations check equal array lengths
- [ ] **Range validation**: Values within acceptable ranges
- [ ] **String lengths**: Metadata fields have max length checks

### Error Types
- [ ] **Custom errors**: All errors defined in enum (not string messages)
- [ ] **Error coverage**: Every failure case has specific error
- [ ] **Error propagation**: Errors properly propagated with `?` operator
- [ ] **Result types**: All fallible functions return `Result<T, Error>`
- [ ] **Panic prevention**: No `unwrap()`, `expect()` in production code (use `?` or handle)

### Edge Cases
- [ ] **Empty transfers**: Handle zero-amount transfers gracefully
- [ ] **Self-transfers**: Check behavior of transferring to self
- [ ] **First interaction**: Contract works correctly with fresh deployment
- [ ] **Last interaction**: Handles total withdrawal/burn scenarios
- [ ] **Duplicate operations**: Idempotent operations behave correctly

---

## 🎯 Events & Logging

### Event Emission
- [ ] **All state changes**: Every state modification emits event
- [ ] **Indexed topics**: Important fields marked with `#[ink(topic)]` (max 3 indexed fields)
- [ ] **Event data**: Events include all relevant data for offchain indexing
- [ ] **Before/after states**: Events capture both old and new values where relevant
- [ ] **No sensitive data**: Events don't leak private information

### Event Coverage
- [ ] **Transfer events**: All token transfers (incl. mint/burn)
- [ ] **Approval events**: All approval changes
- [ ] **Ownership events**: Ownership transfers
- [ ] **Role events**: Role grants/revokes
- [ ] **Pause events**: Pause/unpause state changes
- [ ] **Custom events**: Business logic emits relevant events

---

## ⛽ Gas Optimization & DoS Prevention

### Gas Efficiency
- [ ] **Storage reads**: Minimized (cache in local variables)
- [ ] **Storage writes**: Batched where possible
- [ ] **Loop bounds**: All loops have fixed/limited iterations
- [ ] **Batch operations**: Provided for common multi-item operations
- [ ] **Lazy evaluation**: Expensive computations only when needed

### Denial of Service (DoS) Prevention
- [ ] **Unbounded loops**: No loops over user-controlled arrays
- [ ] **Gas griefing**: External calls can't drain all gas
- [ ] **Block gas limit**: Operations fit within block gas limit
- [ ] **Array size limits**: Max size enforced for user-provided arrays
- [ ] **Mapping iteration**: No iteration over large Mappings

### Resource Limits
- [ ] **String length limits**: URIs, names, symbols have max length (e.g., 256 bytes)
- [ ] **Array size limits**: Batch operations limited (e.g., max 100 items)
- [ ] **Call depth limits**: No deep call chains
- [ ] **Storage growth**: Contract storage bounded or has cleanup mechanism

---

## 🧪 Testing & Code Quality

### Test Coverage
- [ ] **Unit tests**: All functions have unit tests
- [ ] **Edge cases**: Tests cover zero, max, overflow, underflow scenarios
- [ ] **Error paths**: Tests verify all error conditions
- [ ] **Access control**: Tests verify unauthorized access fails
- [ ] **Integration tests**: E2E tests with multiple contracts if applicable
- [ ] **Coverage > 80%**: Code coverage exceeds 80% (use `cargo tarpaulin`)

### Test Quality
- [ ] **Test accounts**: Uses default test accounts (alice, bob, charlie)
- [ ] **Caller switching**: Tests use `set_caller()` to test different users
- [ ] **Assertions**: Clear, descriptive assertions
- [ ] **Test independence**: Tests can run in any order
- [ ] **Mock data**: Test data is realistic

### Code Quality
- [ ] **DRY principle**: No repeated code (use helper functions)
- [ ] **Function length**: Functions < 50 lines (ideally < 30)
- [ ] **Cognitive complexity**: Low cyclomatic complexity
- [ ] **Magic numbers**: Constants used instead of magic numbers
- [ ] **TODO/FIXME**: No unresolved TODOs/FIXMEs in production code

---

## 🔬 Advanced Security Checks

### Cryptography
- [ ] **Random generation**: Uses secure on-chain randomness (not timestamp)
- [ ] **Hashing**: Uses `blake2b_256` or `sha2_256` from `ink::env::hash`
- [ ] **Signature verification**: ECDSA signatures verified correctly if used
- [ ] **Nonce management**: Nonces incremented atomically if used

### MEV & Front-Running
- [ ] **Price manipulation**: Trades use slippage protection
- [ ] **Oracle attacks**: Price feeds validated (not single source)
- [ ] **Commit-reveal**: Sensitive operations use commit-reveal pattern if needed
- [ ] **Time-based attacks**: Block timestamp not used for critical logic

### Upgrade Patterns
- [ ] **Proxy compatibility**: Storage layout compatible if using proxy pattern
- [ ] **Initialization**: `initialize()` function protected (only callable once)
- [ ] **Delegated calls**: Delegate call targets validated
- [ ] **Storage collisions**: No storage slot conflicts in upgradeable contracts

### Privacy & Confidentiality
- [ ] **ZK commitments**: Privacy commitments use proper hashing (Blake2-256)
- [ ] **Salt randomness**: Commitment salts have sufficient entropy (min 16 bytes)
- [ ] **Reveal validation**: Commitments validated against reveals
- [ ] **Data leakage**: No sensitive data exposed in events or public functions

---

## 📦 Deployment & Operations

### Pre-Deployment
- [ ] **Final audit**: External audit completed (recommended for high-value contracts)
- [ ] **Testnet deployment**: Deployed and tested on testnet
- [ ] **ABI review**: Exported ABI matches expected interface
- [ ] **Metadata check**: Contract metadata complete and accurate
- [ ] **Constructor args**: Constructor arguments documented

### Post-Deployment
- [ ] **Contract verification**: Source code verified on block explorer
- [ ] **Ownership transfer**: If applicable, ownership transferred to multi-sig
- [ ] **Pause mechanism**: Emergency pause function available
- [ ] **Upgrade path**: Upgrade mechanism documented (if applicable)
- [ ] **Monitoring**: Events monitored for unusual activity

### Documentation
- [ ] **README**: Complete with usage examples
- [ ] **Architecture**: High-level design documented
- [ ] **Functions**: All public functions documented with `///` comments
- [ ] **Parameters**: Parameter meanings clear
- [ ] **Examples**: SDK usage examples provided
- [ ] **Known limitations**: Limitations and assumptions documented

---

## 🐞 Common Vulnerabilities Checked

### Critical Vulnerabilities
- [ ] **Reentrancy**: No reentrancy vulnerabilities (state updated before external calls)
- [ ] **Integer overflow/underflow**: All arithmetic saturating or checked
- [ ] **Access control bypass**: All protected functions check permissions
- [ ] **Unauthorized mint/burn**: Only authorized roles can mint/burn
- [ ] **Denial of service**: No unbounded loops or gas exhaustion vectors

### High Severity
- [ ] **Approval race condition**: Safe approval pattern used (set to zero first)
- [ ] **Unsafe delegatecall**: No unsafe delegate calls
- [ ] **Unprotected self-destruct**: No self-destruct or equivalent without protection
- [ ] **Timestamp dependence**: Block timestamp not used for critical logic
- [ ] **Signature replay**: Signatures include nonce/chainId if used

### Medium Severity
- [ ] **Front-running**: Sensitive operations protected from front-running
- [ ] **Oracle manipulation**: Price feeds from multiple sources if used
- [ ] **Gas griefing**: External calls bounded
- [ ] **Unexpected ether**: Contract handles unexpected balance (if applicable)
- [ ] **Storage collision**: No storage slot collisions in upgradeable contracts

---

## 📊 Audit Summary Template

```markdown
## Security Audit Summary

**Contract**: [Name]
**Version**: [Version]
**Auditor**: [Name/Company]
**Date**: [Date]

### Checklist Score
- Access Control: ___/18 ✅
- Token Economics: ___/18 ✅
- State Management: ___/16 ✅
- Input Validation: ___/15 ✅
- Events & Logging: ___/11 ✅
- Gas Optimization: ___/12 ✅
- Testing: ___/13 ✅
- Advanced Security: ___/16 ✅
- Deployment: ___/15 ✅
- Common Vulnerabilities: ___/15 ✅

**Total Score**: ___/149 (___%)

### Critical Issues
- [None / List issues]

### High Severity Issues
- [None / List issues]

### Medium Severity Issues
- [None / List issues]

### Low Severity / Informational
- [None / List issues]

### Recommendations
1. [Recommendation 1]
2. [Recommendation 2]

### Approval Status
- [ ] ✅ APPROVED for production deployment
- [ ] ⚠️  APPROVED with recommendations
- [ ] ❌ NOT APPROVED (critical issues found)

**Auditor Signature**: ___________________
```

---

## 🛠️ Automated Tools

### Recommended Tools
1. **cargo-clippy**: Rust linting
   ```bash
   cargo clippy --all-targets -- -D warnings
   ```

2. **cargo-tarpaulin**: Code coverage
   ```bash
   cargo tarpaulin --out Html --output-dir coverage
   ```

3. **cargo-fuzz**: Fuzzing (detect edge cases)
   ```bash
   cargo install cargo-fuzz
   cargo fuzz run fuzz_target_1
   ```

4. **cargo-audit**: Dependency vulnerability scanning
   ```bash
   cargo install cargo-audit
   cargo audit
   ```

5. **cargo-geiger**: Unsafe code detection
   ```bash
   cargo install cargo-geiger
   cargo geiger
   ```

---

## 📚 Additional Resources

### BelizeChain-Specific
- [GEM Best Practices](/docs/BEST_PRACTICES.md)
- [Chain Extensions Guide](/docs/CHAIN_EXTENSIONS.md) (when available)
- [Privacy Features Guide](/docs/PRIVACY_FEATURES.md) (when available)

### ink! Resources
- [ink! Security Guidelines](https://use.ink/basics/security)
- [ink! Common Pitfalls](https://use.ink/basics/contract-testing#common-pitfalls)
- [OpenBrush Security Patterns](https://github.com/Brushfam/openbrush-contracts)

### General Smart Contract Security
- [Smart Contract Weakness Classification (SWC)](https://swcregistry.io/)
- [Consensys Smart Contract Best Practices](https://consensys.github.io/smart-contract-best-practices/)

---

## ✨ Final Notes

**Risk Assessment**:
- **Low Risk**: Community tools, test contracts, non-financial applications
- **Medium Risk**: Token contracts, small-scale DeFi (< $100K TVL)
- **High Risk**: DEX, lending protocols, large-scale DeFi (> $100K TVL)
- **Critical Risk**: Cross-chain bridges, custody solutions, > $1M TVL

**Recommended For High/Critical Risk**:
- ✅ External professional audit (2-4 weeks)
- ✅ Bug bounty program ($5K-$50K rewards)
- ✅ Gradual rollout (start with caps/limits)
- ✅ Multi-signature admin control
- ✅ Monitoring and alerting system
- ✅ Incident response plan

---

**Last Updated**: February 14, 2026  
**License**: MIT  
**Maintained By**: BelizeChain Team

*This checklist is continuously improved. Suggest improvements via GitHub issues.*

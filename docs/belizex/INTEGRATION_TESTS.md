# BelizeX Integration Tests

Integration tests for the complete BelizeX suite (Factory + Pair + Router + PSP22 tokens).

## Test Scenarios

### 1. Factory Tests
- ✅ Create pair
- ✅ Get pair address
- ✅ List all pairs
- ✅ Fee management

### 2. Pair Tests (Liquidity)
- Add initial liquidity (first LP)
- Add more liquidity (subsequent LPs)
- Remove liquidity
- Check LP token balances

### 3. Pair Tests (Swaps)
- Swap token0 for token1
- Swap token1 for token0
- Verify price impact
- Verify 0.3% fee collection

### 4. Router Tests
- Add liquidity via Router
- Remove liquidity via Router
- Swap exact tokens for tokens
- Swap tokens for exact tokens
- Multi-hop swap (A → B → C)
- Slippage protection
- Deadline enforcement

### 5. End-to-End Tests
- Full cycle: Create pair → Add liquidity → Swap → Remove liquidity
- Multiple users trading
- Price discovery mechanism
- TWAP oracle updates

## Running Tests

```bash
# Unit tests (individual contracts)
cargo test --manifest-path dex/factory/Cargo.toml
cargo test --manifest-path dex/pair/Cargo.toml
cargo test --manifest-path dex/router/Cargo.toml

# E2E tests (requires deployed contracts)
cargo test --manifest-path dex/factory/Cargo.toml --features e2e-tests
cargo test --manifest-path dex/pair/Cargo.toml --features e2e-tests
cargo test --manifest-path dex/router/Cargo.toml --features e2e-tests
```

## Test Data

Example token amounts for testing:
- Initial liquidity: 1000 DALLA + 2000 BZC
- Swap amount: 100 DALLA
- Expected output: ~181 BZC (after 0.3% fee)

## Integration Test Template

```rust
#[ink_e2e::test]
async fn e2e_full_dex_cycle<Client: E2EBackend>(mut client: Client) -> E2EResult<()> {
    // 1. Deploy tokens (DALLA, BZC)
    let dalla = deploy_token(&mut client, "DALLA", 21_000_000 * 10^12).await?;
    let bzc = deploy_token(&mut client, "BZC", 100_000_000 * 10^12).await?;
    
    // 2. Deploy Factory
    let factory = deploy_factory(&mut client).await?;
    
    // 3. Create trading pair
    let pair = factory.create_pair(dalla, bzc).await?;
    
    // 4. Add initial liquidity
    dalla.transfer(pair, 1000 * 10^12).await?;
    bzc.transfer(pair, 2000 * 10^12).await?;
    let liquidity = pair.mint(alice).await?;
    
    assert!(liquidity > 0);
    
    // 5. Swap tokens
    dalla.transfer(pair, 100 * 10^12).await?;
    let amount_out = pair.swap(0, 181 * 10^12, bob).await?;
    
    // 6. Verify balances
    let bob_bzc = bzc.balance_of(bob).await?;
    assert_eq!(bob_bzc, 181 * 10^12);
    
    Ok(())
}
```

## Status

- [x] Unit tests written
- [ ] E2E test framework setup
- [ ] E2E test implementation
- [ ] Gas optimization tests
- [ ] Security tests
- [ ] Load tests

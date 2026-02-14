# BelizeX Deployment Guide

This guide provides complete instructions for deploying the GEM BelizeX suite to BelizeChain.

## 📋 Prerequisites

### 1. Install cargo-contract

```bash
cargo install cargo-contract --force --locked
```

Verify installation:
```bash
cargo contract --version
# Should show: cargo-contract-contract 4.x.x
```

### 2. Install Substrate Contracts Node (Optional for local testing)

```bash
cargo install contracts-node --git https://github.com/paritytech/substrate-contracts-node.git --force --locked
```

### 3. Install Polkadot.js Apps Dependencies

```bash
cd sdk
npm install
```

---

## 🏗️ Build Contracts

### Generate Contract ABIs and WASM

Build all three BelizeX contracts:

```bash
# Navigate to BelizeX directory
cd /home/wicked/Projects/gem/dex

# Build Factory
cargo contract build --release --manifest-path factory/Cargo.toml

# Build Pair
cargo contract build --release --manifest-path pair/Cargo.toml

# Build Router  
cargo contract build --release --manifest-path router/Cargo.toml
```

**Output locations**:
- Factory: `dex/factory/target/ink/belizex_factory.contract`, `belizex_factory.wasm`, `belizex_factory.json`
- Pair: `dex/pair/target/ink/belizex_pair.contract`, `belizex_pair.wasm`, `belizex_pair.json`
- Router: `dex/router/target/ink/belizex_router.contract`, `belizex_router.wasm`, `belizex_router.json`

### Copy ABIs to SDK

```bash
# Create SDK contracts directory
mkdir -p /home/wicked/Projects/gem/sdk/contracts

# Copy ABIs
cp dex/factory/target/ink/belizex_factory.json sdk/contracts/
cp dex/pair/target/ink/belizex_pair.json sdk/contracts/
cp dex/router/target/ink/belizex_router.json sdk/contracts/
```

---

## 🚀 Deploy to BelizeChain

### Option 1: Using Polkadot.js Apps UI

1. **Connect to BelizeChain**:
   - Navigate to https://polkadot.js.org/apps/
   - Settings → Connect to custom endpoint: `ws://localhost:9944` (or your BelizeChain node)

2. **Deploy Factory Contract**:
   - Developer → Contracts → Upload & Deploy Code
   - Upload `belizex_factory.contract`
   - Constructor: `new(fee_to_setter)`
     - `fee_to_setter`: Your admin account address
   - Deploy and note the contract address

3. **Upload Pair Code** (Don't instantiate yet):
   - Upload & Deploy Code → Upload Only
   - Upload `belizex_pair.contract`
   - Note the **code hash** (needed for Factory)

4. **Deploy Router Contract**:
   - Upload & Deploy Code
   - Upload `belizex_router.contract`
   - Constructor: `new(factory, wbzc)`
     - `factory`: Factory contract address from step 2
     - `wbzc`: Wrapped BZC token address (deploy if needed)
   - Deploy and note the contract address

### Option 2: Using cargo-contract CLI

```bash
# Deploy Factory
cargo contract instantiate factory/Cargo.toml \
  --constructor new \
  --args <FEE_TO_SETTER_ADDRESS> \
  --suri //Alice \
  --url ws://localhost:9944 \
  --execute

# Upload Pair code (get code hash)
cargo contract upload pair/Cargo.toml \
  --suri //Alice \
  --url ws://localhost:9944 \
  --execute

# Deploy Router
cargo contract instantiate router/Cargo.toml \
  --constructor new \
  --args <FACTORY_ADDRESS> <WBZC_ADDRESS> \
  --suri //Alice \
  --url ws://localhost:9944 \
  --execute
```

### Option 3: Using SDK Deployment Script

Create `sdk/deploy-dex.js`:

```javascript
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { CodePromise, ContractPromise } = require('@polkadot/api-contract');
const fs = require('fs');

async function main() {
    // Connect
    const provider = new WsProvider('ws://localhost:9944');
    const api = await ApiPromise.create({ provider });
    
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    
    console.log('🔗 Connected to BelizeChain');
    
    // Load ABIs and WASM
    const factoryAbi = JSON.parse(fs.readFileSync('contracts/belizex_factory.json', 'utf8'));
    const pairAbi = JSON.parse(fs.readFileSync('contracts/belizex_pair.json', 'utf8'));
    const routerAbi = JSON.parse(fs.readFileSync('contracts/belizex_router.json', 'utf8'));
    
    const factoryWasm = fs.readFileSync('../dex/factory/target/ink/belizex_factory.wasm');
    const pairWasm = fs.readFileSync('../dex/pair/target/ink/belizex_pair.wasm');
    const routerWasm = fs.readFileSync('../dex/router/target/ink/belizex_router.wasm');
    
    // Deploy Factory
    console.log('\\n🏭 Deploying Factory...');
    const factoryCode = new CodePromise(api, factoryAbi, factoryWasm);
    const factoryTx = factoryCode.tx.new({ gasLimit: api.registry.createType('WeightV2', {
        refTime: 100000000000,
        proofSize: 131072,
    })}, alice.address);
    
    const factoryAddress = await new Promise((resolve) => {
        factoryTx.signAndSend(alice, ({ contract, status }) => {
            if (status.isInBlock || status.isFinalized && contract) {
                console.log(`✅ Factory deployed: ${contract.address}`);
                resolve(contract.address);
            }
        });
    });
    
    // Upload Pair code
    console.log('\\n📦 Uploading Pair code...');
    const pairCode = new CodePromise(api, pairAbi, pairWasm);
    const pairCodeHash = pairCode.code.hash.toHex();
    console.log(`✅ Pair code hash: ${pairCodeHash}`);
    
    // Deploy Router
    console.log('\\n🔀 Deploying Router...');
    const wbzcAddress = 'YOUR_WBZC_ADDRESS'; // Replace with actual WBZC address
    const routerCode = new CodePromise(api, routerAbi, routerWasm);
    const routerTx = routerCode.tx.new({ gasLimit: api.registry.createType('WeightV2', {
        refTime: 100000000000,
        proofSize: 131072,
    })}, factoryAddress.toString(), wbzcAddress);
    
    const routerAddress = await new Promise((resolve) => {
        routerTx.signAndSend(alice, ({ contract, status }) => {
            if (status.isInBlock || status.isFinalized && contract) {
                console.log(`✅ Router deployed: ${contract.address}`);
                resolve(contract.address);
            }
        });
    });
    
    console.log('\\n🎉 Deployment complete!');
    console.log(`Factory: ${factoryAddress}`);
    console.log(`Router: ${routerAddress}`);
    console.log(`Pair Code Hash: ${pairCodeHash}`);
    
    await api.disconnect();
}

main().catch(console.error);
```

Run:
```bash
cd sdk
node deploy-dex.js
```

---

## 🧪 Testing Deployment

### 1. Create a Trading Pair

Using SDK:

```javascript
const { BelizeXSDK } = require('./dex');
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');

async function createPair() {
    const api = await ApiPromise.create({ 
        provider: new WsProvider('ws://localhost:9944') 
    });
    
    const dex = new BelizeXSDK(api, {
        factory: 'FACTORY_ADDRESS',
        router: 'ROUTER_ADDRESS'
    });
    
    await dex.init({
        factory: require('./contracts/belizex_factory.json'),
        pair: require('./contracts/belizex_pair.json'),
        router: require('./contracts/belizex_router.json')
    });
    
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    
    // Create DALLA/BZC pair
    const pairAddress = await dex.createPair(
        'DALLA_ADDRESS',
        'BZC_ADDRESS',
        alice
    );
    
    console.log(`✅ Pair created: ${pairAddress}`);
}

createPair().catch(console.error);
```

### 2. Add Liquidity

```javascript
const amountA = dex.parseAmount('1000', 12); // 1000 DALLA
const amountB = dex.parseAmount('2000', 12); // 2000 BZC
const deadline = Math.floor(Date.now() / 1000) + 300; // 5 min

const { amountA: actualA, amountB: actualB, liquidity } = await dex.addLiquidity(
    {
        tokenA: 'DALLA_ADDRESS',
        tokenB: 'BZC_ADDRESS',
        amountADesired: amountA,
        amountBDesired: amountB,
        amountAMin: dex.parseAmount('990', 12), // 1% slippage
        amountBMin: dex.parseAmount('1980', 12),
        to: alice.address,
        deadline
    },
    alice
);

console.log(`✅ Liquidity added: ${dex.formatAmount(liquidity)} LP tokens`);
```

### 3. Execute Swap

```javascript
const swapAmountIn = dex.parseAmount('100', 12); // 100 DALLA
const path = ['DALLA_ADDRESS', 'BZC_ADDRESS'];

// Get expected output
const amounts = await dex.getAmountsOut(swapAmountIn, path);
const expectedOut = amounts[1];

// Calculate price impact
const { reserve0, reserve1 } = await dex.getReserves('PAIR_ADDRESS');
const priceImpact = dex.calculatePriceImpact(swapAmountIn, reserve0, reserve1);
console.log(`Price impact: ${priceImpact.toFixed(2)}%`);

// Execute swap with 1% slippage
const minOutput = (BigInt(expectedOut) * BigInt(99) / BigInt(100)).toString();

const swapAmounts = await dex.swapExactTokensForTokens(
    {
        amountIn: swapAmountIn,
        amountOutMin: minOutput,
        path,
        to: alice.address,
        deadline: Math.floor(Date.now() / 1000) + 300
    },
    alice
);

console.log(`✅ Swapped: ${dex.formatAmount(swapAmounts[0])} → ${dex.formatAmount(swapAmounts[1])}`);
```

---

## 📊 Monitoring

### Check Pair Reserves

```javascript
const { reserve0, reserve1, blockTimestamp } = await dex.getReserves('PAIR_ADDRESS');
console.log(`Reserves: ${reserve0} / ${reserve1}`);
console.log(`Price: ${parseFloat(reserve1) / parseFloat(reserve0)}`);
```

### Check LP Token Balance

```javascript
const lpBalance = await dex.getBalanceOf('PAIR_ADDRESS', alice.address);
console.log(`LP Balance: ${dex.formatAmount(lpBalance)}`);
```

### List All Pairs

```javascript
const totalPairs = await dex.getAllPairsLength();
console.log(`Total pairs: ${totalPairs}`);

for (let i = 0; i < totalPairs; i++) {
    const pairAddress = await dex.getPairByIndex(i);
    const { reserve0, reserve1 } = await dex.getReserves(pairAddress);
    console.log(`Pair ${i}: ${pairAddress} - Reserves: ${reserve0}/${reserve1}`);
}
```

---

## 🔧 Configuration

### Gas Limits

Adjust in SDK if needed:

```javascript
// In dex.js, modify gas limits
const gasLimit = api.registry.createType('WeightV2', {
    refTime: 15000000000,  // Increase if transactions fail
    proofSize: 262144,     // Increase for complex operations
});
```

### Fee Settings (Factory Admin Only)

```javascript
// Set fee recipient
await factory.tx.setFeeTo({ gasLimit }, feeRecipientAddress).signAndSend(admin);

// Set fee to setter (transfer admin)
await factory.tx.setFeeToSetter({ gasLimit }, newAdminAddress).signAndSend(admin);
```

---

## 🛡️ Security Checklist

Before deploying to production:

- [ ] **Audit Contracts**: Run security audit using tools like Mythril or manual review
- [ ] **Test E2E Flows**: Execute complete workflow (create pair → add liquidity → swap → remove)
- [ ] **Verify Gas Limits**: Test on testnet with realistic transaction sizes
- [ ] **Check Slippage Protection**: Ensure `amountMin` parameters prevent front-running
- [ ] **Test Deadline Enforcement**: Verify expired transactions revert
- [ ] **Validate K-Value Protection**: Confirm swaps maintain or increase K
- [ ] **Test Reentrancy Guards**: Attempt reentrancy attacks
- [ ] **Verify LP Token Accounting**: Check mint/burn calculations
- [ ] **Test Multi-Hop Swaps**: Execute 2-hop and 3-hop paths
- [ ] **Monitor Initial Liquidity**: Ensure MINIMUM_LIQUIDITY is locked
- [ ] **Set Fee Recipient**: Configure `fee_to` address for protocol fees
- [ ] **Backup Admin Keys**: Secure factory admin private key

---

## 📝 Contract Addresses Template

After deployment, update [DEPLOYMENT.md](DEPLOYMENT.md):

```markdown
# BelizeX Deployment Addresses

**Network**: BelizeChain Mainnet  
**Date**: 2026-02-14

## Core Contracts

- **Factory**: `5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty`
- **Router**: `5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y`  
- **Pair Code Hash**: `0x1234...abcd`

## Token Addresses

- **DALLA (PSP22)**: `5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy`
- **BZC (Wrapped)**: `5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw`

## Trading Pairs

| Pair | Address | Initial Liquidity |
|------|---------|-------------------|
| DALLA/BZC | `5C4hrmcuz...` | 1000 DALLA / 2000 BZC |

## Admin Accounts

- **Fee To Setter**: `//Alice` (Production: Use multisig)
- **Fee Recipient**: Not set (fees burned)

## Deployment Logs

```
[2026-02-14 10:30:00] Factory deployed: 5FHneW46...
[2026-02-14 10:31:15] Pair code uploaded: 0x1234...
[2026-02-14 10:32:30] Router deployed: 5FLSigC9...
[2026-02-14 10:35:00] DALLA/BZC pair created: 5C4hrmcuz...
[2026-02-14 10:40:00] Initial liquidity added: 1000/2000
```
```

---

## 🆘 Troubleshooting

### Build Errors

**Issue**: `cargo contract` command not found  
**Solution**: Install cargo-contract (see Prerequisites)

**Issue**: WASM build fails  
**Solution**: Ensure Rust nightly is installed: `rustup target add wasm32-unknown-unknown`

### Deployment Errors

**Issue**: Transaction fails with "OutOfGas"  
**Solution**: Increase gas limits in deployment scripts

**Issue**: "Module not found" error  
**Solution**: Verify contract compiles locally first: `cargo build`

### Runtime Errors

**Issue**: Swap fails with "InsufficientLiquidity"  
**Solution**: Ensure pair has liquidity added first

**Issue**: "Expired" error on transaction  
**Solution**: Increase deadline or submit transaction faster

**Issue**: "InsufficientOutputAmount" (slippage)  
**Solution**: Increase slippage tolerance in `amountOutMin` parameter

---

## 📚 Additional Resources

- [BelizeX README](../dex/README.md) - Architecture and math formulas
- [Integration Tests](../dex/INTEGRATION_TESTS.md) - Test scenarios
- [SDK Example](examples/belizex-swap.js) - Complete usage example
- [Uniswap V2 Whitepaper](https://uniswap.org/whitepaper.pdf) - Original AMM design
- [ink! Documentation](https://use.ink/) - Smart contract framework
- [Polkadot.js](https://polkadot.js.org/) - JavaScript API

---

**Version**: 1.2.0  
**Last Updated**: February 14, 2026  
**Status**: Production Ready ✅

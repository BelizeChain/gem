/**
 * BelizeX Example - Complete BelizeX workflow
 * 
 * Demonstrates:
 * 1. Creating a trading pair
 * 2. Adding liquidity
 * 3. Swapping tokens
 * 4. Removing liquidity
 */

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { BelizeXSDK } = require('../dex');

async function main() {
    // Connect to BelizeChain
    const provider = new WsProvider('ws://localhost:9944');
    const api = await ApiPromise.create({ provider });
    
    console.log('✅ Connected to BelizeChain');
    console.log(`   Chain: ${await api.rpc.system.chain()}`);
    
    // Initialize accounts
    const keyring = new Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');
    const bob = keyring.addFromUri('//Bob');
    
    console.log('\n📝 Account addresses:');
    console.log(`   Alice: ${alice.address}`);
    console.log(`   Bob: ${bob.address}`);
    
    // Initialize BelizeX SDK
    const belizex = new BelizeXSDK(api, {
        factory: 'YOUR_FACTORY_ADDRESS',
        router: 'YOUR_ROUTER_ADDRESS'
    });
    
    await belizex.init({
        factory: require('../contracts/dex_factory.json'),
        pair: require('../contracts/dex_pair.json'),
        router: require('../contracts/dex_router.json')
    });
    
    // Token addresses (replace with deployed addresses)
    const DALLA_ADDRESS = 'YOUR_DALLA_ADDRESS';
    const BZC_ADDRESS = 'YOUR_BZC_ADDRESS';
    
    console.log('\n📝 Token addresses:');
    console.log(`   DALLA: ${DALLA_ADDRESS}`);
    console.log(`   BZC: ${BZC_ADDRESS}`);
    
    // ============================================================================
    // Step 1: Create trading pair
    // ============================================================================
    
    console.log('\n🏭 Creating DALLA/BZC trading pair...');
    
    let pairAddress = await belizex.getPairAddress(DALLA_ADDRESS, BZC_ADDRESS);
    
    if (!pairAddress) {
        // Pair doesn't exist, create it
        pairAddress = await belizex.createPair(DALLA_ADDRESS, BZC_ADDRESS, alice);
        console.log(`✅ Pair created: ${pairAddress}`);
    } else {
        console.log(`✅ Pair already exists: ${pairAddress}`);
    }
    
    // ============================================================================
    // Step 2: Add liquidity
    // ============================================================================
    
    console.log('\n💰 Adding liquidity...');
    
    const amountA = belizex.parseAmount('1000', 12); // 1000 DALLA
    const amountB = belizex.parseAmount('2000', 12); // 2000 BZC
    const minA = belizex.parseAmount('990', 12);     // 1% slippage
    const minB = belizex.parseAmount('1980', 12);    // 1% slippage
    const deadline = Math.floor(Date.now() / 1000) + 300; // 5 minutes
    
    console.log(`   Amount A (DALLA): ${belizex.formatAmount(amountA)}`);
    console.log(`   Amount B (BZC): ${belizex.formatAmount(amountB)}`);
    console.log(`   Deadline: ${new Date(deadline * 1000).toISOString()}`);
    
    const { amountA: actualA, amountB: actualB, liquidity } = await belizex.addLiquidity(
        {
            tokenA: DALLA_ADDRESS,
            tokenB: BZC_ADDRESS,
            amountADesired: amountA,
            amountBDesired: amountB,
            amountAMin: minA,
            amountBMin: minB,
            to: alice.address,
            deadline
        },
        alice
    );
    
    console.log(`✅ Liquidity added:`);
    console.log(`   DALLA deposited: ${belizex.formatAmount(actualA)}`);
    console.log(`   BZC deposited: ${belizex.formatAmount(actualB)}`);
    console.log(`   LP tokens received: ${belizex.formatAmount(liquidity)}`);
    
    // ============================================================================
    // Step 3: Check reserves
    // ============================================================================
    
    console.log('\n📊 Checking pair reserves...');
    
    const { reserve0, reserve1, blockTimestamp } = await belizex.getReserves(pairAddress);
    
    console.log(`   Reserve 0: ${belizex.formatAmount(reserve0)}`);
    console.log(`   Reserve 1: ${belizex.formatAmount(reserve1)}`);
    console.log(`   Last update: ${new Date(blockTimestamp * 1000).toISOString()}`);
    
    // Calculate price
    const price = parseFloat(reserve1) / parseFloat(reserve0);
    console.log(`   Price: 1 DALLA = ${price.toFixed(4)} BZC`);
    
    // ============================================================================
    // Step 4: Swap tokens
    // ============================================================================
    
    console.log('\n🔄 Swapping 100 DALLA for BZC...');
    
    const swapAmountIn = belizex.parseAmount('100', 12);
    const path = [DALLA_ADDRESS, BZC_ADDRESS];
    const swapDeadline = Math.floor(Date.now() / 1000) + 300;
    
    // Get expected output amount
    const amounts = await belizex.getAmountsOut(swapAmountIn, path);
    const expectedOut = amounts[1];
    
    console.log(`   Input DALLA: ${belizex.formatAmount(swapAmountIn)}`);
    console.log(`   Expected BZC: ${belizex.formatAmount(expectedOut)}`);
    
    // Calculate price impact
    const priceImpact = belizex.calculatePriceImpact(swapAmountIn, reserve0, reserve1);
    console.log(`   Price impact: ${priceImpact.toFixed(2)}%`);
    
    // Set slippage tolerance (1%)
    const minOutput = (BigInt(expectedOut) * BigInt(99) / BigInt(100)).toString();
    
    // Execute swap
    const swapAmounts = await belizex.swapExactTokensForTokens(
        {
            amountIn: swapAmountIn,
            amountOutMin: minOutput,
            path,
            to: bob.address,
            deadline: swapDeadline
        },
        alice
    );
    
    console.log(`✅ Swap executed:`);
    console.log(`   DALLA in: ${belizex.formatAmount(swapAmounts[0])}`);
    console.log(`   BZC out: ${belizex.formatAmount(swapAmounts[1])}`);
    
    // ============================================================================
    // Step 5: Check updated reserves
    // ============================================================================
    
    console.log('\n📊 Checking updated reserves...');
    
    const { reserve0: newReserve0, reserve1: newReserve1 } = await belizex.getReserves(pairAddress);
    
    console.log(`   Reserve 0: ${belizex.formatAmount(newReserve0)}`);
    console.log(`   Reserve 1: ${belizex.formatAmount(newReserve1)}`);
    
    const newPrice = parseFloat(newReserve1) / parseFloat(newReserve0);
    console.log(`   New price: 1 DALLA = ${newPrice.toFixed(4)} BZC`);
    console.log(`   Price change: ${((newPrice - price) / price * 100).toFixed(2)}%`);
    
    // ============================================================================
    // Step 6: Check LP balance
    // ============================================================================
    
    console.log('\n💎 Checking LP token balance...');
    
    const lpBalance = await belizex.getBalanceOf(pairAddress, alice.address);
    console.log(`   Alice LP tokens: ${belizex.formatAmount(lpBalance)}`);
    
    // ============================================================================
    // Step 7: Remove liquidity (50%)
    // ============================================================================
    
    console.log('\n↩️  Removing 50% liquidity...');
    
    const liquidityToRemove = (BigInt(lpBalance) / BigInt(2)).toString();
    const removeDeadline = Math.floor(Date.now() / 1000) + 300;
    
    console.log(`   LP tokens to burn: ${belizex.formatAmount(liquidityToRemove)}`);
    
    const { amountA: returnedA, amountB: returnedB } = await belizex.removeLiquidity(
        {
            tokenA: DALLA_ADDRESS,
            tokenB: BZC_ADDRESS,
            liquidity: liquidityToRemove,
            amountAMin: '0', // For demo, in production use proper slippage
            amountBMin: '0',
            to: alice.address,
            deadline: removeDeadline
        },
        alice
    );
    
    console.log(`✅ Liquidity removed:`);
    console.log(`   DALLA returned: ${belizex.formatAmount(returnedA)}`);
    console.log(`   BZC returned: ${belizex.formatAmount(returnedB)}`);
    
    // ============================================================================
    // Step 8: Final stats
    // ============================================================================
    
    console.log('\n📈 Final statistics:');
    
    const totalPairs = await belizex.getAllPairsLength();
    const totalSupply = await belizex.getTotalSupply(pairAddress);
    const finalLpBalance = await belizex.getBalanceOf(pairAddress, alice.address);
    
    console.log(`   Total pairs created: ${totalPairs}`);
    console.log(`   Pair total LP supply: ${belizex.formatAmount(totalSupply)}`);
    console.log(`   Alice remaining LP: ${belizex.formatAmount(finalLpBalance)}`);
    console.log(`   LP ownership: ${(parseFloat(finalLpBalance) / parseFloat(totalSupply) * 100).toFixed(2)}%`);
    
    console.log('\n✅ BelizeX workflow complete!');
    
    await api.disconnect();
}

main()
    .catch((error) => {
        console.error('❌ Error:', error);
        process.exit(1);
    });

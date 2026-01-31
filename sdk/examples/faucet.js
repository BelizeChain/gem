/**
 * Claim tokens from testnet faucet
 * 
 * Usage: node examples/faucet.js
 */

const { GemSDK } = require('../index');

// CONFIGURATION - Update these values
const FAUCET_CONTRACT = '5ExampleFaucetAddress'; // Update when deployed
const NODE_URL = 'ws://localhost:9944';

async function main() {
    console.log('💧 Faucet Example\n');

    const sdk = new GemSDK(NODE_URL);
    await sdk.connect();

    // Get account
    const bob = sdk.getAccount('//Bob');
    console.log(`Account: ${bob.address}\n`);

    try {
        // Get faucet stats
        console.log('📊 Faucet Statistics:');
        const stats = await sdk.faucetStats(FAUCET_CONTRACT);
        console.log(`   Total Claimed: ${stats.totalClaimed}`);
        console.log(`   Claim Count: ${stats.claimCount}`);
        console.log(`   Drip Amount: ${stats.dripAmount}`);
        console.log(`   Cooldown: ${stats.cooldown} blocks\n`);

        // Check if can claim
        console.log('🔍 Checking eligibility...');
        const canClaim = await sdk.faucetCanClaim(FAUCET_CONTRACT, bob.address);

        if (!canClaim) {
            console.log('❌ Cannot claim yet (cooldown active)');
            return;
        }

        console.log('✅ Eligible to claim\n');

        // Claim tokens
        console.log('💧 Claiming tokens...');
        await sdk.faucetClaim(FAUCET_CONTRACT, bob);

        console.log('✅ Tokens claimed successfully!\n');

        // Show new stats
        console.log('📊 Updated Faucet Statistics:');
        const newStats = await sdk.faucetStats(FAUCET_CONTRACT);
        console.log(`   Total Claimed: ${newStats.totalClaimed}`);
        console.log(`   Claim Count: ${newStats.claimCount}\n`);

    } catch (error) {
        console.error('\n❌ Claim failed:', error.message);
        console.error('\nPossible reasons:');
        console.error('   - Cooldown period not elapsed');
        console.error('   - Faucet balance depleted');
        console.error('   - Contract address incorrect\n');
    }

    await sdk.disconnect();
}

main().catch(console.error);

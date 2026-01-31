/**
 * Transfer DALLA tokens between accounts
 * 
 * Usage: node examples/transfer.js
 */

const { GemSDK } = require('../index');

// CONFIGURATION - Update these values
const DALLA_CONTRACT = '5GD4w5DP6VUBtFt7F9LB9EDzGcpbzFwvR9CWVNVsNB';
const NODE_URL = 'ws://localhost:9944';

async function main() {
    console.log('💸 DALLA Token Transfer Example\n');

    const sdk = new GemSDK(NODE_URL);
    await sdk.connect();

    // Get accounts
    const alice = sdk.getAccount('//Alice');
    const bob = sdk.getAccount('//Bob');

    console.log(`Sender: ${alice.address}`);
    console.log(`Recipient: ${bob.address}\n`);

    try {
        // Get initial balances
        console.log('📊 Initial Balances:');
        const aliceBalanceBefore = await sdk.dallaBalanceOf(DALLA_CONTRACT, alice.address);
        const bobBalanceBefore = await sdk.dallaBalanceOf(DALLA_CONTRACT, bob.address);
        
        console.log(`   Alice: ${(parseInt(aliceBalanceBefore) / 1e12).toFixed(2)} DALLA`);
        console.log(`   Bob: ${(parseInt(bobBalanceBefore) / 1e12).toFixed(2)} DALLA\n`);

        // Transfer 100 DALLA
        const amount = 100 * 1e12; // 100 DALLA
        console.log(`💸 Transferring ${amount / 1e12} DALLA...`);

        await sdk.dallaTransfer(
            DALLA_CONTRACT,
            alice,
            bob.address,
            amount
        );

        // Get final balances
        console.log('\n📊 Final Balances:');
        const aliceBalanceAfter = await sdk.dallaBalanceOf(DALLA_CONTRACT, alice.address);
        const bobBalanceAfter = await sdk.dallaBalanceOf(DALLA_CONTRACT, bob.address);
        
        console.log(`   Alice: ${(parseInt(aliceBalanceAfter) / 1e12).toFixed(2)} DALLA`);
        console.log(`   Bob: ${(parseInt(bobBalanceAfter) / 1e12).toFixed(2)} DALLA\n`);

        // Show difference
        const aliceDiff = (parseInt(aliceBalanceBefore) - parseInt(aliceBalanceAfter)) / 1e12;
        const bobDiff = (parseInt(bobBalanceAfter) - parseInt(bobBalanceBefore)) / 1e12;
        
        console.log(`✅ Transfer successful!`);
        console.log(`   Alice sent: ${aliceDiff.toFixed(2)} DALLA`);
        console.log(`   Bob received: ${bobDiff.toFixed(2)} DALLA\n`);

    } catch (error) {
        console.error('\n❌ Transfer failed:', error.message);
    }

    await sdk.disconnect();
}

main().catch(console.error);

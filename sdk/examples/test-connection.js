/**
 * Test BelizeChain node connection
 *
 * Usage: node examples/test-connection.js
 */

const { GemSDK } = require('../index');

async function main() {
  console.log('🔗 Testing BelizeChain connection...\n');

  try {
    // Create SDK instance
    const sdk = new GemSDK('ws://localhost:9944');

    // Connect to node
    await sdk.connect();

    // Get chain info
    const chain = await sdk.api.rpc.system.chain();
    const nodeName = await sdk.api.rpc.system.name();
    const nodeVersion = await sdk.api.rpc.system.version();

    console.log(`\n📊 Chain Information:`);
    console.log(`   Chain: ${chain.toString()}`);
    console.log(`   Node: ${nodeName.toString()}`);
    console.log(`   Version: ${nodeVersion.toString()}`);

    // Get genesis hash
    const genesisHash = sdk.api.genesisHash.toHex();
    console.log(`   Genesis: ${genesisHash}`);

    // Get current block
    const header = await sdk.api.rpc.chain.getHeader();
    console.log(`   Block #${header.number.toNumber()}`);

    // Test account
    const alice = sdk.getAccount('//Alice');
    console.log(`\n👤 Test Account: ${alice.address}`);

    const balance = await sdk.getBalance(alice.address);
    console.log(`   Balance: ${(parseInt(balance.free) / 1e12).toFixed(2)} DALLA`);

    // Disconnect
    await sdk.disconnect();

    console.log('\n✅ Connection test successful!\n');
  } catch (error) {
    console.error('\n❌ Connection failed:', error.message);
    console.error('\nMake sure BelizeChain node is running:');
    console.error('   ./target/release/belizechain-node --dev --tmp\n');
    process.exit(1);
  }
}

main().catch(console.error);

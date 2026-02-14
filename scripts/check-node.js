#!/usr/bin/env node
/**
 * Check if BelizeChain node is running and ready for deployment
 * 
 * Usage:
 *   node scripts/check-node.js
 *   node scripts/check-node.js --url=ws://localhost:9944
 */

const { ApiPromise, WsProvider } = require('@polkadot/api');

async function checkNode(url) {
  console.log(`🔍 Checking BelizeChain node at ${url}...`);
  
  try {
    const provider = new WsProvider(url, 5000);
    const api = await ApiPromise.create({ provider });
    
    // Get node information
    const [chain, nodeName, nodeVersion, health] = await Promise.all([
      api.rpc.system.chain(),
      api.rpc.system.name(),
      api.rpc.system.version(),
      api.rpc.system.health()
    ]);
    
    console.log(`\n✅ Node is running and healthy!`);
    console.log(`   Chain: ${chain}`);
    console.log(`   Node: ${nodeName} v${nodeVersion}`);
    console.log(`   Peers: ${health.peers.toNumber()}`);
    console.log(`   Is syncing: ${health.isSyncing.valueOf()}`);
    console.log(`   Should have peers: ${health.shouldHavePeers.valueOf()}`);
    
    // Check contracts pallet
    const contractsAvailable = api.tx.contracts !== undefined;
    if (contractsAvailable) {
      console.log(`   ✅ Contracts pallet: Available`);
    } else {
      console.log(`   ❌ Contracts pallet: Not available`);
      console.log(`      This node doesn't support smart contracts!`);
    }
    
    await api.disconnect();
    return true;
  } catch (error) {
    console.log(`\n❌ Node check failed: ${error.message}`);
    console.log(`\n💡 Troubleshooting:`);
    console.log(`   1. Make sure a BelizeChain node is running`);
    console.log(`   2. For local development, start with:`);
    console.log(`      substrate-contracts-node --dev --tmp`);
    console.log(`   3. Check the WebSocket URL is correct: ${url}`);
    console.log(`   4. Verify firewall/network settings`);
    return false;
  }
}

// Parse arguments
const args = process.argv.slice(2);
let url = process.env.BELIZECHAIN_NODE_URL || 'ws://localhost:9944';

for (const arg of args) {
  if (arg.startsWith('--url=')) {
    url = arg.split('=')[1];
  } else if (arg === '--help' || arg === '-h') {
    console.log(`
BelizeChain Node Health Check

Usage:
  node scripts/check-node.js [--url=<websocket-url>]

Options:
  --url=<url>    WebSocket URL to check (default: ws://localhost:9944)
  --help, -h     Show this help

Environment Variables:
  BELIZECHAIN_NODE_URL    Default WebSocket URL

Examples:
  node scripts/check-node.js
  node scripts/check-node.js --url=wss://testnet.belizechain.io
  BELIZECHAIN_NODE_URL=ws://127.0.0.1:9944 node scripts/check-node.js
    `);
    process.exit(0);
  }
}

checkNode(url).then(success => {
  process.exit(success ? 0 : 1);
});

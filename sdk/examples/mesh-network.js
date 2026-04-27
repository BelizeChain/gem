/**
 * Mesh Network Integration Example
 * Demonstrates how to use GEM SDK with mesh network features
 */

const { GemSDK, MeshNetworkSDK, MeshNodeRole, MeshNodeType } = require('@belizechain/gem-sdk');

const NODE_URL = process.env.BLOCKCHAIN_WS_URL || 'ws://100.81.45.25:9944';

async function main() {
  // 1. Connect to BelizeChain
  const sdk = new GemSDK(NODE_URL);
  await sdk.connect();

  // 2. Initialize mesh network client
  const meshSDK = new MeshNetworkSDK(sdk.api);

  // 3. Get account (mesh node operator)
  const operator = sdk.getAccount('//Alice');
  console.log(`Operator account: ${operator.address}`);

  // 4. Register mesh node (Meshtastic T-Beam in Cayo District)
  console.log('\n📡 Registering mesh node...');
  try {
    await meshSDK.registerNode(
      operator,
      'TBeam-CAYO-001', // Node ID (from Meshtastic hardware)
      MeshNodeRole.ROUTER, // This node will relay messages
      MeshNodeType.T_BEAM, // T-Beam hardware
      {
        latitude: 17.1899, // San Ignacio, Cayo
        longitude: -89.0726,
        district: 'Cayo',
      }
    );
    console.log('✅ Mesh node registered successfully');
  } catch (error) {
    console.error(' Failed to register node:', error.message);
  }

  // 5. Get node status
  console.log('\n📊 Checking node status...');
  const nodeStatus = await meshSDK.getNodeStatus('TBeam-CAYO-001');
  if (nodeStatus) {
    console.log('Node Status:');
    console.log(`  - Owner: ${nodeStatus.owner}`);
    console.log(`  - Role: ${nodeStatus.role}`);
    console.log(`  - Type: ${nodeStatus.nodeType}`);
    console.log(`  - Location: ${nodeStatus.location.district}, BZ`);
    console.log(`  - Reputation: ${nodeStatus.reputation}`);
    console.log(`  - Relays: ${nodeStatus.relayCount}`);
    console.log(`  - Active: ${nodeStatus.isActive}`);
  } else {
    console.log('❌ Node not found');
  }

  // 6. Get all active nodes in Cayo District
  console.log('\n🗺️  Getting all nodes in Cayo District...');
  const cayoNodes = await meshSDK.getDistrictNodes('Cayo');
  console.log(`Found ${cayoNodes.length} active nodes:`);
  cayoNodes.forEach((node) => {
    console.log(
      `  - ${node.nodeId}: ${node.role} (${node.reputation} reputation, ${node.relayCount} relays)`
    );
  });

  // 7. Send heartbeat
  console.log('\n💓 Sending heartbeat...');
  try {
    await meshSDK.sendHeartbeat(operator, 'TBeam-CAYO-001');
    console.log('✅ Heartbeat sent');
  } catch (error) {
    console.error('❌ Failed to send heartbeat:', error.message);
  }

  // 8. Simulate relaying a transaction via LoRa mesh
  console.log('\n📡 Relaying transaction via mesh...');
  try {
    // Compress a transaction for LoRa (87 bytes)
    const tx = {
      nonce: 1,
      from: operator.address,
      to: '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty', // Bob
      amount: '1000000000000000', // 1000 DALLA
    };

    const compressedTx = MeshNetworkSDK.compressTransaction(tx);
    console.log(`Compressed tx size: ${compressedTx.length} bytes`);

    await meshSDK.relayTransaction(operator, compressedTx);
    console.log('✅ Transaction relayed successfully');
  } catch (error) {
    console.error('❌ Failed to relay transaction:', error.message);
  }

  // 9. Get relay statistics
  console.log('\n📈 Checking relay statistics...');
  const stats = await meshSDK.getRelayStats(operator.address);
  console.log('Relay Stats:');
  console.log(`  - Total Relays: ${stats.totalRelays}`);
  console.log(`  - Successful: ${stats.successfulRelays}`);
  console.log(`  - Total Rewards: ${stats.totalRewards} DALLA`);
  console.log(`  - Last Relay: Block #${stats.lastRelay}`);

  // 10. Claim relay mining rewards
  if (stats.totalRelays > 0) {
    console.log('\n💰 Claiming relay rewards...');
    try {
      await meshSDK.claimRewards(operator);
      console.log('✅ Rewards claimed successfully');
    } catch (error) {
      console.error('❌ Failed to claim rewards:', error.message);
    }
  }

  // 11. Monitor mesh network activity
  console.log('\n👀 Monitoring mesh network events...');
  sdk.api.query.system.events((events) => {
    events.forEach((record) => {
      const { event } = record;

      // Filter for mesh network events
      if (event.section === 'mesh') {
        console.log(`\n🔔 Mesh Event: ${event.method}`);
        console.log(`   Data: ${JSON.stringify(event.data.toHuman(), null, 2)}`);
      }
    });
  });

  // Keep running to monitor events
  console.log('\nPress Ctrl+C to exit...');
}

// Run the example
main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});

/**
 * Mesh Network SDK Extension for BelizeChain GEM
 * Provides access to pallet-belize-mesh functionality
 *
 * @module @belizechain/gem-sdk/mesh
 * @version 1.1.0
 */

/**
 * Mesh node roles
 */
const MeshNodeRole = {
  CLIENT: 'Client',
  ROUTER: 'Router',
  GATEWAY: 'Gateway',
  VALIDATOR_RELAY: 'ValidatorRelay',
  EMERGENCY_BEACON: 'EmergencyBeacon',
};

/**
 * Mesh node types (Meshtastic hardware)
 */
const MeshNodeType = {
  T_BEAM: 'TBeam',
  HELTEC_V3: 'HeltecV3',
  RAK_WISBLOCK: 'RAKWisBlock',
  STATION_G2: 'StationG2',
  LILYGO: 'LilyGo',
  CUSTOM: 'Custom',
};

/**
 * Mesh Network SDK extension
 */
class MeshNetworkSDK {
  constructor(api) {
    if (!api) {
      throw new Error('API instance required');
    }
    this.api = api;
  }

  /**
   * Register a Meshtastic mesh node
   * @param {KeyringPair} signer - Account signing the transaction
   * @param {string} nodeId - Unique node identifier (Meshtastic hardware ID)
   * @param {string} role - Node role (use MeshNodeRole constants)
   * @param {string} nodeType - Hardware type (use MeshNodeType constants)
   * @param {Object} location - Node location { latitude, longitude, district }
   * @returns {Promise<TransactionResult>}
   */
  async registerNode(signer, nodeId, role, nodeType, location) {
    const tx = this.api.tx.mesh.registerNode(
      nodeId,
      role,
      nodeType,
      location.latitude,
      location.longitude,
      location.district
    );

    return new Promise((resolve, reject) => {
      tx.signAndSend(signer, ({ status, events }) => {
        if (status.isInBlock || status.isFinalized) {
          console.log(`✅ Mesh node registered: ${nodeId}`);
          resolve({ status, events });
        }
      }).catch(reject);
    });
  }

  /**
   * Relay a compressed transaction via LoRa mesh
   * @param {KeyringPair} signer - Account signing the relay
   * @param {Uint8Array} compressedTx - Compressed transaction (87 bytes max)
   * @returns {Promise<TransactionResult>}
   */
  async relayTransaction(signer, compressedTx) {
    if (compressedTx.length > 87) {
      throw new Error(`Compressed tx too large: ${compressedTx.length} bytes (max 87)`);
    }

    const tx = this.api.tx.mesh.relayTransaction(compressedTx);

    return new Promise((resolve, reject) => {
      tx.signAndSend(signer, ({ status, events }) => {
        if (status.isInBlock || status.isFinalized) {
          console.log(`✅ Transaction relayed via mesh`);
          resolve({ status, events });
        }
      }).catch(reject);
    });
  }

  /**
   * Claim relay mining rewards
   * @param {KeyringPair} signer - Node operator account
   * @returns {Promise<TransactionResult>}
   */
  async claimRewards(signer) {
    const tx = this.api.tx.mesh.claimRelayRewards();

    return new Promise((resolve, reject) => {
      tx.signAndSend(signer, ({ status, events }) => {
        if (status.isInBlock || status.isFinalized) {
          // Find RewardsClaimed event
          const rewardEvent = events.find(
            ({ event }) => event.section === 'mesh' && event.method === 'RewardsClaimed'
          );

          if (rewardEvent) {
            const [, amount] = rewardEvent.event.data;
            console.log(`✅ Claimed ${amount.toString()} DALLA in relay rewards`);
          }

          resolve({ status, events });
        }
      }).catch(reject);
    });
  }

  /**
   * Get mesh node status
   * @param {string} nodeId - Node identifier
   * @returns {Promise<Object|null>} Node information or null if not found
   */
  async getNodeStatus(nodeId) {
    const nodeInfo = await this.api.query.mesh.meshNodes(nodeId);

    if (nodeInfo.isNone) {
      return null;
    }

    const node = nodeInfo.unwrap();
    return {
      nodeId: nodeId,
      owner: node.owner.toString(),
      role: node.role.toString(),
      nodeType: node.nodeType.toString(),
      location: {
        latitude: node.location.latitude.toNumber(),
        longitude: node.location.longitude.toNumber(),
        district: node.location.district.toString(),
      },
      reputation: node.reputation.toNumber(),
      relayCount: node.relayCount.toNumber(),
      lastHeartbeat: node.lastHeartbeat.toNumber(),
      isActive: node.isActive.valueOf(),
    };
  }

  /**
   * Get node relay statistics
   * @param {string} accountId - Node operator account
   * @returns {Promise<Object>} Relay statistics
   */
  async getRelayStats(accountId) {
    const stats = await this.api.query.mesh.relayStats(accountId);

    if (stats.isNone) {
      return {
        totalRelays: 0,
        successfulRelays: 0,
        totalRewards: '0',
        lastRelay: 0,
      };
    }

    const data = stats.unwrap();
    return {
      totalRelays: data.totalRelays.toNumber(),
      successfulRelays: data.successfulRelays.toNumber(),
      totalRewards: data.totalRewards.toString(),
      lastRelay: data.lastRelay.toNumber(),
    };
  }

  /**
   * Get all active mesh nodes in a district
   * @param {string} district - District name (e.g., 'Belize', 'Cayo')
   * @returns {Promise<Array>} List of active nodes
   */
  async getDistrictNodes(district) {
    const entries = await this.api.query.mesh.meshNodes.entries();
    const nodes = [];

    for (const [key, value] of entries) {
      if (value.isNone) continue;

      const node = value.unwrap();
      if (node.location.district.toString() === district && node.isActive.valueOf()) {
        const nodeId = key.args[0].toString();
        nodes.push({
          nodeId,
          role: node.role.toString(),
          reputation: node.reputation.toNumber(),
          relayCount: node.relayCount.toNumber(),
        });
      }
    }

    return nodes;
  }

  /**
   * Send heartbeat to update node status
   * @param {KeyringPair} signer - Node operator account
   * @param {string} nodeId - Node identifier
   * @returns {Promise<TransactionResult>}
   */
  async sendHeartbeat(signer, nodeId) {
    const tx = this.api.tx.mesh.heartbeat(nodeId);

    return new Promise((resolve, reject) => {
      tx.signAndSend(signer, ({ status, events }) => {
        if (status.isInBlock || status.isFinalized) {
          console.log(`✅ Heartbeat sent for node: ${nodeId}`);
          resolve({ status, events });
        }
      }).catch(reject);
    });
  }

  /**
   * Compress transaction for LoRa relay (87 bytes)
   * @param {Object} tx - Transaction data
   * @returns {Uint8Array} Compressed transaction
   */
  static compressTransaction(tx) {
    const buffer = new Uint8Array(87);

    // Format: [nonce(4)] + [from(32)] + [to(32)] + [amount(16)] + [signature(3)]
    // This is a simplified compression - production would use proper encoding

    // Nonce (4 bytes)
    const nonceBytes = new Uint8Array(new Uint32Array([tx.nonce]).buffer);
    buffer.set(nonceBytes, 0);

    // From address (32 bytes) - simplified, actual would decode SS58
    const fromBytes = new TextEncoder().encode(tx.from.slice(0, 32));
    buffer.set(fromBytes, 4);

    // To address (32 bytes)
    const toBytes = new TextEncoder().encode(tx.to.slice(0, 32));
    buffer.set(toBytes, 36);

    // Amount (16 bytes)
    const amountBytes = new Uint8Array(16);
    const amount = BigInt(tx.amount);
    for (let i = 0; i < 16; i++) {
      amountBytes[i] = Number((amount >> BigInt(i * 8)) & BigInt(0xff));
    }
    buffer.set(amountBytes, 68);

    // Signature prefix (3 bytes) - simplified
    buffer.set([0xff, 0xff, 0xff], 84);

    return buffer;
  }
}

module.exports = MeshNetworkSDK;
module.exports.MeshNodeRole = MeshNodeRole;
module.exports.MeshNodeType = MeshNodeType;

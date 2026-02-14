/**
 * TypeScript type definitions for Mesh Network SDK
 * @module @belizechain/gem-sdk/mesh
 */

import { ApiPromise } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';

/**
 * Mesh node roles
 */
export declare const MeshNodeRole: {
    CLIENT: string;
    ROUTER: string;
    GATEWAY: string;
    VALIDATOR_RELAY: string;
    EMERGENCY_BEACON: string;
};

/**
 * Mesh node types (Meshtastic hardware)
 */
export declare const MeshNodeType: {
    T_BEAM: string;
    HELTEC_V3: string;
    RAK_WISBLOCK: string;
    STATION_G2: string;
    LILYGO: string;
    CUSTOM: string;
};

/**
 * Node location information
 */
export interface MeshLocation {
    latitude: number;
    longitude: number;
    district: string;
}

/**
 * Mesh node status
 */
export interface MeshNodeStatus {
    nodeId: string;
    owner: string;
    role: string;
    nodeType: string;
    location: MeshLocation;
    reputation: number;
    relayCount: number;
    lastHeartbeat: number;
    isActive: boolean;
}

/**
 * Relay statistics
 */
export interface RelayStats {
    totalRelays: number;
    successfulRelays: number;
    totalRewards: string;
    lastRelay: number;
}

/**
 * Transaction result
 */
export interface TransactionResult {
    status: any;
    events: any[];
}

/**
 * Mesh Network SDK extension
 */
export declare class MeshNetworkSDK {
    /**
     * Polkadot API instance
     */
    api: ApiPromise;

    /**
     * Create a new MeshNetworkSDK instance
     * @param api - Connected Polkadot API instance
     */
    constructor(api: ApiPromise);

    /**
     * Register a Meshtastic mesh node
     * @param signer - Account signing the transaction
     * @param nodeId - Unique node identifier (Meshtastic hardware ID)
     * @param role - Node role (use MeshNodeRole constants)
     * @param nodeType - Hardware type (use MeshNodeType constants)
     * @param location - Node location
     */
    registerNode(
        signer: KeyringPair,
        nodeId: string,
        role: string,
        nodeType: string,
        location: MeshLocation
    ): Promise<TransactionResult>;

    /**
     * Relay a compressed transaction via LoRa mesh
     * @param signer - Account signing the relay
     * @param compressedTx - Compressed transaction (87 bytes max)
     */
    relayTransaction(
        signer: KeyringPair,
        compressedTx: Uint8Array
    ): Promise<TransactionResult>;

    /**
     * Claim relay mining rewards
     * @param signer - Node operator account
     */
    claimRewards(signer: KeyringPair): Promise<TransactionResult>;

    /**
     * Get mesh node status
     * @param nodeId - Node identifier
     */
    getNodeStatus(nodeId: string): Promise<MeshNodeStatus | null>;

    /**
     * Get node relay statistics
     * @param accountId - Node operator account
     */
    getRelayStats(accountId: string): Promise<RelayStats>;

    /**
     * Get all active mesh nodes in a district
     * @param district - District name (e.g., 'Belize', 'Cayo')
     */
    getDistrictNodes(district: string): Promise<Array<{
        nodeId: string;
        role: string;
        reputation: number;
        relayCount: number;
    }>>;

    /**
     * Send heartbeat to update node status
     * @param signer - Node operator account
     * @param nodeId - Node identifier
     */
    sendHeartbeat(signer: KeyringPair, nodeId: string): Promise<TransactionResult>;

    /**
     * Compress transaction for LoRa relay (87 bytes)
     * @param tx - Transaction data
     */
    static compressTransaction(tx: {
        nonce: number;
        from: string;
        to: string;
        amount: string;
    }): Uint8Array;
}

/**
 * TypeScript type definitions for BelizeChain Gem SDK
 * @module @belizechain/gem-sdk
 */

import { ApiPromise } from '@polkadot/api';
import { ContractPromise } from '@polkadot/api-contract';
import { KeyringPair } from '@polkadot/keyring/types';
import { Keyring } from '@polkadot/keyring';

/**
 * Balance information
 */
export interface Balance {
    free: string;
    reserved: string;
    frozen: string;
}

/**
 * Token metadata
 */
export interface TokenMetadata {
    name: string;
    symbol: string;
    decimals: number;
    totalSupply: string;
}

/**
 * DAO proposal
 */
export interface Proposal {
    proposer: string;
    description: string;
    votesFor: string;
    votesAgainst: string;
    endBlock: number;
    executed: boolean;
    status: 'Pending' | 'Active' | 'Passed' | 'Rejected' | 'Executed';
}

/**
 * Faucet statistics
 */
export interface FaucetStats {
    totalClaimed: string;
    claimCount: number;
    dripAmount: string;
    cooldown: number;
}

/**
 * Transaction result
 */
export interface TransactionResult {
    status: {
        isInBlock: boolean;
        isFinalized: boolean;
        asInBlock: any;
        asFinalized: any;
    };
    isError: boolean;
}

/**
 * Main GemSDK class for interacting with BelizeChain
 */
export declare class GemSDK {
    /**
     * WebSocket URL of BelizeChain node
     */
    nodeUrl: string;

    /**
     * Polkadot API instance
     */
    api: ApiPromise | null;

    /**
     * Keyring for account management
     */
    keyring: Keyring | null;

    /**
     * Loaded contract instances
     */
    contracts: Record<string, ContractPromise>;

    /**
     * Create a new GemSDK instance
     * @param nodeUrl - WebSocket URL of BelizeChain node (default: ws://localhost:9944)
     */
    constructor(nodeUrl?: string);

    /**
     * Connect to BelizeChain node
     */
    connect(): Promise<ApiPromise>;

    /**
     * Disconnect from node
     */
    disconnect(): Promise<void>;

    /**
     * Load a contract instance
     * @param contractType - Contract type: 'dalla', 'belinft', 'dao', 'faucet'
     * @param address - Contract address
     */
    loadContract(contractType: string, address: string): ContractPromise;

    /**
     * Get account from seed phrase or URI
     * @param uri - Account URI (e.g., '//Alice', seed phrase, or private key)
     */
    getAccount(uri: string): KeyringPair;

    /**
     * Get account balance
     * @param address - Account address
     */
    getBalance(address: string): Promise<Balance>;

    // ============================================
    // DALLA Token (PSP22) Helper Functions
    // ============================================

    /**
     * Transfer DALLA tokens
     * @param contractAddress - DALLA contract address
     * @param signer - Sender account
     * @param to - Recipient address
     * @param amount - Amount to transfer
     * @param gasLimit - Gas limit (default: 100000000000)
     */
    dallaTransfer(
        contractAddress: string,
        signer: KeyringPair,
        to: string,
        amount: string | number,
        gasLimit?: number
    ): Promise<TransactionResult>;

    /**
     * Get DALLA token balance
     * @param contractAddress - DALLA contract address
     * @param account - Account address
     */
    dallaBalanceOf(contractAddress: string, account: string): Promise<string>;

    /**
     * Get DALLA token metadata
     * @param contractAddress - DALLA contract address
     */
    dallaMetadata(contractAddress: string): Promise<TokenMetadata>;

    // ============================================
    // BeliNFT (PSP34) Helper Functions
    // ============================================

    /**
     * Mint NFT
     * @param contractAddress - BeliNFT contract address
     * @param signer - Minter account (must be owner)
     * @param to - Recipient address
     * @param uri - Token URI (IPFS or HTTP)
     */
    nftMint(
        contractAddress: string,
        signer: KeyringPair,
        to: string,
        uri: string
    ): Promise<TransactionResult>;

    /**
     * Get NFT owner
     * @param contractAddress - BeliNFT contract address
     * @param tokenId - Token ID
     */
    nftOwnerOf(contractAddress: string, tokenId: number): Promise<string>;

    /**
     * Get NFT metadata URI
     * @param contractAddress - BeliNFT contract address
     * @param tokenId - Token ID
     */
    nftTokenUri(contractAddress: string, tokenId: number): Promise<string>;

    // ============================================
    // DAO Helper Functions
    // ============================================

    /**
     * Create DAO proposal
     * @param contractAddress - DAO contract address
     * @param signer - Proposer account
     * @param description - Proposal description
     */
    daoCreateProposal(
        contractAddress: string,
        signer: KeyringPair,
        description: string
    ): Promise<number>;

    /**
     * Vote on DAO proposal
     * @param contractAddress - DAO contract address
     * @param signer - Voter account
     * @param proposalId - Proposal ID
     * @param support - true = for, false = against
     */
    daoVote(
        contractAddress: string,
        signer: KeyringPair,
        proposalId: number,
        support: boolean
    ): Promise<TransactionResult>;

    /**
     * Get DAO proposal
     * @param contractAddress - DAO contract address
     * @param proposalId - Proposal ID
     */
    daoGetProposal(contractAddress: string, proposalId: number): Promise<Proposal>;

    // ============================================
    // Faucet Helper Functions
    // ============================================

    /**
     * Claim tokens from faucet
     * @param contractAddress - Faucet contract address
     * @param signer - Claimer account
     */
    faucetClaim(
        contractAddress: string,
        signer: KeyringPair
    ): Promise<TransactionResult>;

    /**
     * Check if account can claim from faucet
     * @param contractAddress - Faucet contract address
     * @param account - Account address
     */
    faucetCanClaim(contractAddress: string, account: string): Promise<boolean>;

    /**
     * Get faucet statistics
     * @param contractAddress - Faucet contract address
     */
    faucetStats(contractAddress: string): Promise<FaucetStats>;
}

// ============================================
// Export extension modules (v1.1.0+)
// ============================================

export { MeshNetworkSDK, MeshNodeRole, MeshNodeType } from './meshNetwork';
export { PrivacySDK } from './privacy';
export { BelizeXSDK } from './belizex';

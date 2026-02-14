import { ApiPromise } from '@polkadot/api';
import { ContractPromise } from '@polkadot/api-contract';
import { KeyringPair } from '@polkadot/keyring/types';

export interface BelizeXAddresses {
    factory: string;
    router: string;
}

export interface BelizeXAbis {
    factory: any;
    pair: any;
    router: any;
}

export interface AddLiquidityParams {
    tokenA: string;
    tokenB: string;
    amountADesired: string;
    amountBDesired: string;
    amountAMin: string;
    amountBMin: string;
    to: string;
    deadline: number;
}

export interface RemoveLiquidityParams {
    tokenA: string;
    tokenB: string;
    liquidity: string;
    amountAMin: string;
    amountBMin: string;
    to: string;
    deadline: number;
}

export interface SwapParams {
    amountIn?: string;
    amountOut?: string;
    amountOutMin?: string;
    amountInMax?: string;
    path: string[];
    to: string;
    deadline: number;
}

export interface Reserves {
    reserve0: string;
    reserve1: string;
    blockTimestamp: number;
}

export interface LiquidityResult {
    amountA: string;
    amountB: string;
    liquidity?: string;
}

/**
 * BelizeX SDK for BelizeChain decentralized exchange
 */
export declare class BelizeXSDK {
    /**
     * Create BelizeX SDK instance
     * @param api - Polkadot JS API instance
     * @param addresses - DEX contract addresses
     */
    constructor(api: any, addresses?: DexAddresses);

    /**
     * Initialize DEX contracts
     * @param abis - Contract ABIs
     */
    init(abis: DexAbis): Promise<void>;

    // Factory Methods
    /**
     * Create a new trading pair
     * @param tokenA - First token address
     * @param tokenB - Second token address
     * @param signer - Account to sign transaction
     * @returns Pair address
     */
    createPair(tokenA: string, tokenB: string, signer: any): Promise<string>;

    /**
     * Get pair address for two tokens
     * @param tokenA - First token address
     * @param tokenB - Second token address
     * @returns Pair address or null
     */
    getPairAddress(tokenA: string, tokenB: string): Promise<string | null>;

    /**
     * Get total number of pairs
     * @returns Number of pairs
     */
    getAllPairsLength(): Promise<number>;

    /**
     * Get pair by index
     * @param index - Pair index
     * @returns Pair address
     */
    getPairByIndex(index: number): Promise<string>;

    // Pair Methods
    /**
     * Get pair reserves
     * @param pairAddress - Pair address
     * @returns Reserve data
     */
    getReserves(pairAddress: string): Promise<Reserves>;

    /**
     * Get LP token total supply
     * @param pairAddress - Pair address
     * @returns Total supply
     */
    getTotalSupply(pairAddress: string): Promise<string>;

    /**
     * Get LP token balance
     * @param pairAddress - Pair address
     * @param account - Account address
     * @returns LP token balance
     */
    getBalanceOf(pairAddress: string, account: string): Promise<string>;

    /**
     * Calculate output for given input
     * @param pairAddress - Pair address
     * @param amountIn - Input amount
     * @param reserveIn - Input reserve
     * @param reserveOut - Output reserve
     * @returns Output amount
     */
    getAmountOut(
        pairAddress: string,
        amountIn: string,
        reserveIn: string,
        reserveOut: string
    ): Promise<string>;

    // Router Methods
    /**
     * Add liquidity to a pair
     * @param params - Liquidity parameters
     * @param signer - Account to sign transaction
     * @returns Liquidity result
     */
    addLiquidity(params: AddLiquidityParams, signer: any): Promise<LiquidityResult>;

    /**
     * Remove liquidity from a pair
     * @param params - Removal parameters
     * @param signer - Account to sign transaction
     * @returns Token amounts returned
     */
    removeLiquidity(params: RemoveLiquidityParams, signer: any): Promise<LiquidityResult>;

    /**
     * Swap exact tokens for tokens
     * @param params - Swap parameters
     * @param signer - Account to sign transaction
     * @returns Amounts for each hop
     */
    swapExactTokensForTokens(params: SwapParams, signer: any): Promise<string[]>;

    /**
     * Swap tokens for exact tokens
     * @param params - Swap parameters
     * @param signer - Account to sign transaction
     * @returns Amounts for each hop
     */
    swapTokensForExactTokens(params: SwapParams, signer: any): Promise<string[]>;

    /**
     * Get amounts out for multi-hop swap (query only)
     * @param amountIn - Input amount
     * @param path - Token path
     * @returns Output amounts for each hop
     */
    getAmountsOut(amountIn: string, path: string[]): Promise<string[]>;

    /**
     * Get amounts in for multi-hop swap (query only)
     * @param amountOut - Desired output
     * @param path - Token path
     * @returns Input amounts for each hop
     */
    getAmountsIn(amountOut: string, path: string[]): Promise<string[]>;

    // Utility Methods
    /**
     * Calculate price impact
     * @param amountIn - Input amount
     * @param reserveIn - Input reserve
     * @param reserveOut - Output reserve
     * @returns Price impact percentage
     */
    calculatePriceImpact(amountIn: string, reserveIn: string, reserveOut: string): number;

    /**
     * Format token amount with decimals
     * @param amount - Raw amount
     * @param decimals - Token decimals (default 12)
     * @returns Formatted amount
     */
    formatAmount(amount: string, decimals?: number): string;

    /**
     * Parse token amount from decimal string
     * @param amount - Decimal amount
     * @param decimals - Token decimals (default 12)
     * @returns Raw amount
     */
    parseAmount(amount: string, decimals?: number): string;
}

/**
 * BelizeX SDK Module
 * Interact with BelizeChain DEX (Factory, Pair, Router)
 */

const { ApiPromise, WsProvider } = require('@polkadot/api');
const { ContractPromise } = require('@polkadot/api-contract');

// Contract ABIs (to be generated after deployment)
// const FACTORY_ABI = require('./contracts/dex_factory.json');
// const PAIR_ABI = require('./contracts/dex_pair.json');
// const ROUTER_ABI = require('./contracts/dex_router.json');

class BelizeXSDK {
    /**
     * Create BelizeX SDK instance
     * @param {ApiPromise} api - Polkadot JS API instance
     * @param {Object} addresses - Contract addresses
     * @param {string} addresses.factory - Factory contract address
     * @param {string} addresses.router - Router contract address
     */
    constructor(api, addresses = {}) {
        this.api = api;
        this.factoryAddress = addresses.factory;
        this.routerAddress = addresses.router;
        this.factory = null;
        this.router = null;
        this.pairs = new Map(); // Cache pair contracts
    }

    /**
     * Initialize DEX contracts
     * @param {Object} abis - Contract ABIs
     * @param {Object} abis.factory - Factory ABI
     * @param {Object} abis.pair - Pair ABI
     * @param {Object} abis.router - Router ABI
     */
    async init(abis) {
        if (!this.factoryAddress || !this.routerAddress) {
            throw new Error('BelizeX contract addresses not provided');
        }

        this.factoryABI = abis.factory;
        this.pairABI = abis.pair;
        this.routerABI = abis.router;

        this.factory = new ContractPromise(this.api, abis.factory, this.factoryAddress);
        this.router = new ContractPromise(this.api, abis.router, this.routerAddress);

        console.log('✅ BelizeX SDK initialized');
        console.log(`   Factory: ${this.factoryAddress}`);
        console.log(`   Router: ${this.routerAddress}`);
    }

    // ============================================================================
    // Factory Methods
    // ============================================================================

    /**
     * Create a new trading pair
     * @param {string} tokenA - First token address
     * @param {string} tokenB - Second token address
     * @param {Object} signer - Account to sign transaction
     * @returns {Promise<string>} Pair address
     */
    async createPair(tokenA, tokenB, signer) {
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 10000000000,
            proofSize: 131072,
        });

        const { result, output } = await this.factory.tx
            .createPair({ gasLimit }, tokenA, tokenB)
            .signAndSend(signer);

        if (result.isInBlock || result.isFinalized) {
            const pairAddress = output.toString(); // Extract from event
            console.log(`✅ Pair created: ${pairAddress}`);
            return pairAddress;
        }

        throw new Error('Failed to create pair');
    }

    /**
     * Get pair address for two tokens
     * @param {string} tokenA - First token address
     * @param {string} tokenB - Second token address
     * @returns {Promise<string|null>} Pair address or null if doesn't exist
     */
    async getPairAddress(tokenA, tokenB) {
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 3000000000,
            proofSize: 131072,
        });

        const { output } = await this.factory.query
            .getPairAddress(this.factoryAddress, { gasLimit }, tokenA, tokenB);

        return output.toHuman();
    }

    /**
     * Get total number of pairs
     * @returns {Promise<number>}
     */
    async getAllPairsLength() {
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 3000000000,
            proofSize: 131072,
        });

        const { output } = await this.factory.query
            .allPairsLength(this.factoryAddress, { gasLimit });

        return parseInt(output.toString());
    }

    /**
     * Get pair by index
     * @param {number} index - Pair index
     * @returns {Promise<string>} Pair address
     */
    async getPairByIndex(index) {
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 3000000000,
            proofSize: 131072,
        });

        const { output } = await this.factory.query
            .getPairByIndex(this.factoryAddress, { gasLimit }, index);

        return output.toString();
    }

    // ============================================================================
    // Pair Methods
    // ============================================================================

    /**
     * Get pair contract instance
     * @param {string} pairAddress - Pair contract address
     * @returns {ContractPromise}
     */
    getPairContract(pairAddress) {
        if (!this.pairs.has(pairAddress)) {
            const pair = new ContractPromise(this.api, this.pairABI, pairAddress);
            this.pairs.set(pairAddress, pair);
        }
        return this.pairs.get(pairAddress);
    }

    /**
     * Get pair reserves
     * @param {string} pairAddress - Pair address
     * @returns {Promise<{reserve0: string, reserve1: string, blockTimestamp: number}>}
     */
    async getReserves(pairAddress) {
        const pair = this.getPairContract(pairAddress);
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 3000000000,
            proofSize: 131072,
        });

        const { output } = await pair.query
            .getReserves(pairAddress, { gasLimit });

        const [reserve0, reserve1, blockTimestamp] = output.toHuman();
        
        return { reserve0, reserve1, blockTimestamp };
    }

    /**
     * Get LP token total supply
     * @param {string} pairAddress - Pair address
     * @returns {Promise<string>}
     */
    async getTotalSupply(pairAddress) {
        const pair = this.getPairContract(pairAddress);
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 3000000000,
            proofSize: 131072,
        });

        const { output } = await pair.query
            .totalSupply(pairAddress, { gasLimit });

        return output.toString();
    }

    /**
     * Get LP token balance
     * @param {string} pairAddress - Pair address
     * @param {string} account - Account address
     * @returns {Promise<string>}
     */
    async getBalanceOf(pairAddress, account) {
        const pair = this.getPairContract(pairAddress);
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 3000000000,
            proofSize: 131072,
        });

        const { output } = await pair.query
            .balanceOf(pairAddress, { gasLimit }, account);

        return output.toString();
    }

    /**
     * Calculate output amount for exact input
     * @param {string} pairAddress - Pair address
     * @param {string} amountIn - Input amount
     * @param {string} reserveIn - Input reserve
     * @param {string} reserveOut - Output reserve
     * @returns {Promise<string>} Output amount
     */
    async getAmountOut(pairAddress, amountIn, reserveIn, reserveOut) {
        const pair = this.getPairContract(pairAddress);
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 3000000000,
            proofSize: 131072,
        });

        const { output } = await pair.query
            .getAmountOut(pairAddress, { gasLimit }, amountIn, reserveIn, reserveOut);

        return output.toString();
    }

    // ============================================================================
    // Router Methods (User-Friendly)
    // ============================================================================

    /**
     * Add liquidity to a pair
     * @param {Object} params - Liquidity parameters
     * @param {string} params.tokenA - Token A address
     * @param {string} params.tokenB - Token B address
     * @param {string} params.amountADesired - Desired amount of token A
     * @param {string} params.amountBDesired - Desired amount of token B
     * @param {string} params.amountAMin - Minimum amount of token A (slippage)
     * @param {string} params.amountBMin - Minimum amount of token B (slippage)
     * @param {string} params.to - LP token recipient
     * @param {number} params.deadline - Deadline timestamp
     * @param {Object} signer - Account to sign transaction
     * @returns {Promise<{amountA: string, amountB: string, liquidity: string}>}
     */
    async addLiquidity(params, signer) {
        const { tokenA, tokenB, amountADesired, amountBDesired, amountAMin, amountBMin, to, deadline } = params;
        
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 10000000000,
            proofSize: 131072,
        });

        const { result, output } = await this.router.tx
            .addLiquidity(
                { gasLimit },
                tokenA,
                tokenB,
                amountADesired,
                amountBDesired,
                amountAMin,
                amountBMin,
                to,
                deadline
            )
            .signAndSend(signer);

        if (result.isInBlock || result.isFinalized) {
            const [amountA, amountB, liquidity] = output.toHuman();
            console.log(`✅ Liquidity added: ${amountA} + ${amountB} → ${liquidity} LP`);
            return { amountA, amountB, liquidity };
        }

        throw new Error('Failed to add liquidity');
    }

    /**
     * Remove liquidity from a pair
     * @param {Object} params - Removal parameters
     * @param {string} params.tokenA - Token A address
     * @param {string} params.tokenB - Token B address
     * @param {string} params.liquidity - LP tokens to burn
     * @param {string} params.amountAMin - Minimum amount of token A
     * @param {string} params.amountBMin - Minimum amount of token B
     * @param {string} params.to - Token recipient
     * @param {number} params.deadline - Deadline timestamp
     * @param {Object} signer - Account to sign transaction
     * @returns {Promise<{amountA: string, amountB: string}>}
     */
    async removeLiquidity(params, signer) {
        const { tokenA, tokenB, liquidity, amountAMin, amountBMin, to, deadline } = params;
        
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 10000000000,
            proofSize: 131072,
        });

        const { result, output } = await this.router.tx
            .removeLiquidity(
                { gasLimit },
                tokenA,
                tokenB,
                liquidity,
                amountAMin,
                amountBMin,
                to,
                deadline
            )
            .signAndSend(signer);

        if (result.isInBlock || result.isFinalized) {
            const [amountA, amountB] = output.toHuman();
            console.log(`✅ Liquidity removed: ${liquidity} LP → ${amountA} + ${amountB}`);
            return { amountA, amountB };
        }

        throw new Error('Failed to remove liquidity');
    }

    /**
     * Swap exact tokens for tokens
     * @param {Object} params - Swap parameters
     * @param {string} params.amountIn - Exact input amount
     * @param {string} params.amountOutMin - Minimum output amount (slippage)
     * @param {string[]} params.path - Token swap path [tokenIn, tokenOut] or multi-hop
     * @param {string} params.to - Output token recipient
     * @param {number} params.deadline - Deadline timestamp
     * @param {Object} signer - Account to sign transaction
     * @returns {Promise<string[]>} Amounts for each hop
     */
    async swapExactTokensForTokens(params, signer) {
        const { amountIn, amountOutMin, path, to, deadline } = params;
        
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 10000000000,
            proofSize: 131072,
        });

        const { result, output } = await this.router.tx
            .swapExactTokensForTokens(
                { gasLimit },
                amountIn,
                amountOutMin,
                path,
                to,
                deadline
            )
            .signAndSend(signer);

        if (result.isInBlock || result.isFinalized) {
            const amounts = output.toHuman();
            console.log(`✅ Swap executed: ${amounts.join(' → ')}`);
            return amounts;
        }

        throw new Error('Failed to swap tokens');
    }

    /**
     * Swap tokens for exact tokens
     * @param {Object} params - Swap parameters
     * @param {string} params.amountOut - Exact output amount desired
     * @param {string} params.amountInMax - Maximum input amount (slippage)
     * @param {string[]} params.path - Token swap path
     * @param {string} params.to - Output token recipient
     * @param {number} params.deadline - Deadline timestamp
     * @param {Object} signer - Account to sign transaction
     * @returns {Promise<string[]>} Amounts for each hop
     */
    async swapTokensForExactTokens(params, signer) {
        const { amountOut, amountInMax, path, to, deadline } = params;
        
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 10000000000,
            proofSize: 131072,
        });

        const { result, output } = await this.router.tx
            .swapTokensForExactTokens(
                { gasLimit },
                amountOut,
                amountInMax,
                path,
                to,
                deadline
            )
            .signAndSend(signer);

        if (result.isInBlock || result.isFinalized) {
            const amounts = output.toHuman();
            console.log(`✅ Swap executed: ${amounts.join(' → ')}`);
            return amounts;
        }

        throw new Error('Failed to swap tokens');
    }

    /**
     * Get amounts out for multi-hop swap
     * @param {string} amountIn - Input amount
     * @param {string[]} path - Token path
     * @returns {Promise<string[]>} Expected output amounts for each hop
     */
    async getAmountsOut(amountIn, path) {
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 3000000000,
            proofSize: 131072,
        });

        const { output } = await this.router.query
            .getAmountsOut(this.routerAddress, { gasLimit }, amountIn, path);

        return output.toHuman();
    }

    /**
     * Get amounts in for multi-hop swap
     * @param {string} amountOut - Desired output amount
     * @param {string[]} path - Token path
     * @returns {Promise<string[]>} Required input amounts for each hop
     */
    async getAmountsIn(amountOut, path) {
        const gasLimit = this.api.registry.createType('WeightV2', {
            refTime: 3000000000,
            proofSize: 131072,
        });

        const { output } = await this.router.query
            .getAmountsIn(this.routerAddress, { gasLimit }, amountOut, path);

        return output.toHuman();
    }

    // ============================================================================
    // Utility Methods
    // ============================================================================

    /**
     * Calculate price impact
     * @param {string} amountIn - Input amount
     * @param {string} reserveIn - Input reserve
     * @param {string} reserveOut - Output reserve
     * @returns {number} Price impact percentage
     */
    calculatePriceImpact(amountIn, reserveIn, reserveOut) {
        const spotPrice = parseFloat(reserveOut) / parseFloat(reserveIn);
        const amountOut = this._calculateAmountOut(amountIn, reserveIn, reserveOut);
        const executionPrice = parseFloat(amountOut) / parseFloat(amountIn);
        
        const impact = ((spotPrice - executionPrice) / spotPrice) * 100;
        return Math.abs(impact);
    }

    /**
     * Calculate amount out (client-side)
     * @param {string} amountIn - Input amount
     * @param {string} reserveIn - Input reserve
     * @param {string} reserveOut - Output reserve
     * @returns {string} Output amount
     */
    _calculateAmountOut(amountIn, reserveIn, reserveOut) {
        const amountInWithFee = BigInt(amountIn) * BigInt(997);
        const numerator = amountInWithFee * BigInt(reserveOut);
        const denominator = BigInt(reserveIn) * BigInt(1000) + amountInWithFee;
        return (numerator / denominator).toString();
    }

    /**
     * Format token amount with decimals
     * @param {string} amount - Raw amount
     * @param {number} decimals - Token decimals (default 12)
     * @returns {string} Formatted amount
     */
    formatAmount(amount, decimals = 12) {
        const divisor = BigInt(10) ** BigInt(decimals);
        const integerPart = BigInt(amount) / divisor;
        const fractionalPart = BigInt(amount) % divisor;
        
        return `${integerPart}.${fractionalPart.toString().padStart(decimals, '0')}`;
    }

    /**
     * Parse token amount from decimal string
     * @param {string} amount - Decimal amount (e.g., "100.5")
     * @param {number} decimals - Token decimals (default 12)
     * @returns {string} Raw amount
     */
    parseAmount(amount, decimals = 12) {
        const [integer, fractional = '0'] = amount.split('.');
        const paddedFractional = fractional.padEnd(decimals, '0').slice(0, decimals);
        return (BigInt(integer) * BigInt(10) ** BigInt(decimals) + BigInt(paddedFractional)).toString();
    }
}

module.exports = BelizeXSDK;

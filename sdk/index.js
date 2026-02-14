/**
 * BelizeChain Gem SDK
 * JavaScript/TypeScript SDK for interacting with Gem smart contracts
 *
 * @module @belizechain/gem-sdk
 * @version 1.0.0
 */

const { ApiPromise, WsProvider } = require('@polkadot/api');
const { ContractPromise } = require('@polkadot/api-contract');
const { Keyring } = require('@polkadot/keyring');

// Contract ABIs
const DALLA_ABI = require('./contracts/dalla.json');
const BELINFT_ABI = require('./contracts/belinft.json');
const DAO_ABI = require('./contracts/dao.json');
const FAUCET_ABI = require('./contracts/faucet.json');

/**
 * Main GemSDK class for interacting with BelizeChain
 */
class GemSDK {
  /**
   * Create a new GemSDK instance
   * @param {string} nodeUrl - WebSocket URL of BelizeChain node (default: ws://localhost:9944)
   */
  constructor(nodeUrl = 'ws://localhost:9944') {
    this.nodeUrl = nodeUrl;
    this.api = null;
    this.keyring = null;
    this.contracts = {};
  }

  /**
   * Connect to BelizeChain node
   * @returns {Promise<ApiPromise>}
   */
  async connect() {
    if (this.api) {
      return this.api;
    }

    const provider = new WsProvider(this.nodeUrl);
    this.api = await ApiPromise.create({ provider });
    this.keyring = new Keyring({ type: 'sr25519' });

    console.log(`✅ Connected to BelizeChain at ${this.nodeUrl}`);
    console.log(`   Chain: ${(await this.api.rpc.system.chain()).toString()}`);
    console.log(`   Node: ${(await this.api.rpc.system.name()).toString()}`);
    console.log(`   Version: ${(await this.api.rpc.system.version()).toString()}`);

    return this.api;
  }

  /**
   * Disconnect from node
   */
  async disconnect() {
    if (this.api) {
      await this.api.disconnect();
      this.api = null;
      console.log('✅ Disconnected from BelizeChain');
    }
  }

  /**
   * Load a contract instance
   * @param {string} contractType - Contract type: 'dalla', 'belinft', 'dao', 'faucet'
   * @param {string} address - Contract address
   * @returns {ContractPromise}
   */
  loadContract(contractType, address) {
    if (!this.api) {
      throw new Error('Not connected. Call connect() first.');
    }

    const abiMap = {
      dalla: DALLA_ABI,
      belinft: BELINFT_ABI,
      dao: DAO_ABI,
      faucet: FAUCET_ABI,
    };

    const abi = abiMap[contractType.toLowerCase()];
    if (!abi) {
      throw new Error(`Unknown contract type: ${contractType}`);
    }

    const contract = new ContractPromise(this.api, abi, address);
    this.contracts[contractType] = contract;

    console.log(`✅ Loaded ${contractType} contract at ${address}`);
    return contract;
  }

  /**
   * Get account from seed phrase or URI
   * @param {string} uri - Account URI (e.g., '//Alice', seed phrase, or private key)
   * @returns {KeyringPair}
   */
  getAccount(uri) {
    if (!this.keyring) {
      throw new Error('Not connected. Call connect() first.');
    }
    return this.keyring.addFromUri(uri);
  }

  /**
   * Get account balance
   * @param {string} address - Account address
   * @returns {Promise<Object>} Balance information
   */
  async getBalance(address) {
    if (!this.api) {
      throw new Error('Not connected. Call connect() first.');
    }

    const { data: balance } = await this.api.query.system.account(address);
    return {
      free: balance.free.toString(),
      reserved: balance.reserved.toString(),
      frozen: balance.frozen.toString(),
    };
  }

  // ============================================
  // DALLA Token (PSP22) Helper Functions
  // ============================================

  /**
   * Transfer DALLA tokens
   * @param {string} contractAddress - DALLA contract address
   * @param {KeyringPair} signer - Sender account
   * @param {string} to - Recipient address
   * @param {string|number} amount - Amount to transfer
   * @returns {Promise<Object>} Transaction result
   */
  async dallaTransfer(contractAddress, signer, to, amount) {
    const contract = this.contracts.dalla || this.loadContract('dalla', contractAddress);

    const { gasRequired, result } = await contract.query.transfer(
      signer.address,
      { gasLimit: -1 },
      to,
      amount
    );

    if (result.isErr) {
      throw new Error(`Transfer query failed: ${result.asErr.toString()}`);
    }

    const tx = await contract.tx.transfer({ gasLimit: gasRequired }, to, amount);

    return new Promise((resolve, reject) => {
      tx.signAndSend(signer, (result) => {
        if (result.status.isInBlock) {
          console.log(`✅ Transfer in block: ${result.status.asInBlock.toString()}`);
        }
        if (result.status.isFinalized) {
          console.log(`✅ Transfer finalized: ${result.status.asFinalized.toString()}`);
          resolve(result);
        }
        if (result.isError) {
          reject(new Error('Transaction failed'));
        }
      });
    });
  }

  /**
   * Get DALLA token balance
   * @param {string} contractAddress - DALLA contract address
   * @param {string} account - Account address
   * @returns {Promise<string>} Balance
   */
  async dallaBalanceOf(contractAddress, account) {
    const contract = this.contracts.dalla || this.loadContract('dalla', contractAddress);

    const { result, output } = await contract.query.balanceOf(account, { gasLimit: -1 }, account);

    if (result.isErr) {
      throw new Error(`Balance query failed: ${result.asErr.toString()}`);
    }

    return output.toString();
  }

  /**
   * Get DALLA token metadata
   * @param {string} contractAddress - DALLA contract address
   * @returns {Promise<Object>} Token metadata
   */
  async dallaMetadata(contractAddress) {
    const contract = this.contracts.dalla || this.loadContract('dalla', contractAddress);

    const [name, symbol, decimals, totalSupply] = await Promise.all([
      contract.query.tokenName(contract.address, { gasLimit: -1 }),
      contract.query.tokenSymbol(contract.address, { gasLimit: -1 }),
      contract.query.tokenDecimals(contract.address, { gasLimit: -1 }),
      contract.query.totalSupply(contract.address, { gasLimit: -1 }),
    ]);

    return {
      name: name.output.toString(),
      symbol: symbol.output.toString(),
      decimals: decimals.output.toNumber(),
      totalSupply: totalSupply.output.toString(),
    };
  }

  // ============================================
  // BeliNFT (PSP34) Helper Functions
  // ============================================

  /**
   * Mint NFT
   * @param {string} contractAddress - BeliNFT contract address
   * @param {KeyringPair} signer - Minter account (must be owner)
   * @param {string} to - Recipient address
   * @param {string} uri - Token URI (IPFS or HTTP)
   * @returns {Promise<Object>} Transaction result
   */
  async nftMint(contractAddress, signer, to, uri) {
    const contract = this.contracts.belinft || this.loadContract('belinft', contractAddress);

    const { gasRequired } = await contract.query.mint(signer.address, { gasLimit: -1 }, to, uri);

    const tx = await contract.tx.mint({ gasLimit: gasRequired }, to, uri);

    return new Promise((resolve, reject) => {
      tx.signAndSend(signer, (result) => {
        if (result.status.isFinalized) {
          console.log(`✅ NFT minted: ${result.status.asFinalized.toString()}`);
          resolve(result);
        }
        if (result.isError) {
          reject(new Error('Mint transaction failed'));
        }
      });
    });
  }

  /**
   * Get NFT owner
   * @param {string} contractAddress - BeliNFT contract address
   * @param {number} tokenId - Token ID
   * @returns {Promise<string>} Owner address
   */
  async nftOwnerOf(contractAddress, tokenId) {
    const contract = this.contracts.belinft || this.loadContract('belinft', contractAddress);

    const { output } = await contract.query.ownerOf(contract.address, { gasLimit: -1 }, tokenId);

    return output.toString();
  }

  /**
   * Get NFT metadata URI
   * @param {string} contractAddress - BeliNFT contract address
   * @param {number} tokenId - Token ID
   * @returns {Promise<string>} Token URI
   */
  async nftTokenUri(contractAddress, tokenId) {
    const contract = this.contracts.belinft || this.loadContract('belinft', contractAddress);

    const { output } = await contract.query.tokenUri(contract.address, { gasLimit: -1 }, tokenId);

    return output.toString();
  }

  // ============================================
  // DAO Helper Functions
  // ============================================

  /**
   * Create DAO proposal
   * @param {string} contractAddress - DAO contract address
   * @param {KeyringPair} signer - Proposer account
   * @param {string} description - Proposal description
   * @returns {Promise<number>} Proposal ID
   */
  async daoCreateProposal(contractAddress, signer, description) {
    const contract = this.contracts.dao || this.loadContract('dao', contractAddress);

    const { gasRequired, output } = await contract.query.createProposal(
      signer.address,
      { gasLimit: -1 },
      description
    );

    const tx = await contract.tx.createProposal({ gasLimit: gasRequired }, description);

    return new Promise((resolve, reject) => {
      tx.signAndSend(signer, (result) => {
        if (result.status.isFinalized) {
          console.log(`✅ Proposal created: ${result.status.asFinalized.toString()}`);
          resolve(output.toNumber());
        }
        if (result.isError) {
          reject(new Error('Create proposal failed'));
        }
      });
    });
  }

  /**
   * Vote on DAO proposal
   * @param {string} contractAddress - DAO contract address
   * @param {KeyringPair} signer - Voter account
   * @param {number} proposalId - Proposal ID
   * @param {boolean} support - true = for, false = against
   * @returns {Promise<Object>} Transaction result
   */
  async daoVote(contractAddress, signer, proposalId, support) {
    const contract = this.contracts.dao || this.loadContract('dao', contractAddress);

    const { gasRequired } = await contract.query.vote(
      signer.address,
      { gasLimit: -1 },
      proposalId,
      support
    );

    const tx = await contract.tx.vote({ gasLimit: gasRequired }, proposalId, support);

    return new Promise((resolve, reject) => {
      tx.signAndSend(signer, (result) => {
        if (result.status.isFinalized) {
          console.log(`✅ Vote cast: ${result.status.asFinalized.toString()}`);
          resolve(result);
        }
        if (result.isError) {
          reject(new Error('Vote failed'));
        }
      });
    });
  }

  /**
   * Get DAO proposal
   * @param {string} contractAddress - DAO contract address
   * @param {number} proposalId - Proposal ID
   * @returns {Promise<Object>} Proposal details
   */
  async daoGetProposal(contractAddress, proposalId) {
    const contract = this.contracts.dao || this.loadContract('dao', contractAddress);

    const { output } = await contract.query.getProposal(
      contract.address,
      { gasLimit: -1 },
      proposalId
    );

    const proposal = output.toHuman();
    return proposal;
  }

  // ============================================
  // Faucet Helper Functions
  // ============================================

  /**
   * Claim tokens from faucet
   * @param {string} contractAddress - Faucet contract address
   * @param {KeyringPair} signer - Claimer account
   * @returns {Promise<Object>} Transaction result
   */
  async faucetClaim(contractAddress, signer) {
    const contract = this.contracts.faucet || this.loadContract('faucet', contractAddress);

    const { gasRequired } = await contract.query.claim(signer.address, { gasLimit: -1 });

    const tx = await contract.tx.claim({ gasLimit: gasRequired });

    return new Promise((resolve, reject) => {
      tx.signAndSend(signer, (result) => {
        if (result.status.isFinalized) {
          console.log(`✅ Tokens claimed: ${result.status.asFinalized.toString()}`);
          resolve(result);
        }
        if (result.isError) {
          reject(new Error('Claim failed'));
        }
      });
    });
  }

  /**
   * Check if account can claim from faucet
   * @param {string} contractAddress - Faucet contract address
   * @param {string} account - Account address
   * @returns {Promise<boolean>} Can claim
   */
  async faucetCanClaim(contractAddress, account) {
    const contract = this.contracts.faucet || this.loadContract('faucet', contractAddress);

    const { output } = await contract.query.canClaim(account, { gasLimit: -1 }, account);

    return output.toHuman();
  }

  /**
   * Get faucet statistics
   * @param {string} contractAddress - Faucet contract address
   * @returns {Promise<Object>} Faucet stats
   */
  async faucetStats(contractAddress) {
    const contract = this.contracts.faucet || this.loadContract('faucet', contractAddress);

    const { output } = await contract.query.stats(contract.address, { gasLimit: -1 });

    return output.toHuman();
  }
}

// Export main SDK class
module.exports = { GemSDK };

// Export extension modules (v1.1.0+)
module.exports.MeshNetworkSDK = require('./meshNetwork');
module.exports.PrivacySDK = require('./privacy');
module.exports.BelizeXSDK = require('./belizex').BelizeXSDK;

// Re-export mesh constants for convenience
const { MeshNodeRole, MeshNodeType } = require('./meshNetwork');
module.exports.MeshNodeRole = MeshNodeRole;
module.exports.MeshNodeType = MeshNodeType;

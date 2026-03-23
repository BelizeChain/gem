#!/usr/bin/env node
/**
 * Smart Contract Deployment Script for BelizeChain GEM
 *
 * Deploys contracts to BelizeChain network using @polkadot/api-contract
 *
 * Usage:
 *   node scripts/deploy.js --contract=dalla --network=local
 *   node scripts/deploy.js --contract=all --network=testnet --account=//Alice
 *
 * Environment Variables:
 *   BELIZECHAIN_NODE_URL - WebSocket URL (default: ws://localhost:9944)
 *   DEPLOY_ACCOUNT - Account URI (default: //Alice for dev)
 *   DEPLOY_PASSWORD - Account password (optional)
 */

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { ContractPromise } = require('@polkadot/api-contract');
const { CodePromise } = require('@polkadot/api-contract');
const fs = require('fs');
const path = require('path');

// Detect if running inside Docker (artifacts at /app/artifacts/) vs local dev
const ARTIFACTS_DIR = fs.existsSync('/app/artifacts')
  ? '/app/artifacts'
  : path.join(__dirname, '..');

// Contract configurations
const CONTRACTS = {
  dalla: {
    name: 'DALLA Token (PSP22)',
    localPath: 'dalla_token/target/ink/dalla_token.contract',
    dockerPath: 'dalla_token/dalla_token.contract',
    constructor: 'new',
    args: [1000000000000000000] // 1M DALLA (1_000_000 * 10^12)
  },
  belinft: {
    name: 'BeliNFT Collection (PSP34)',
    localPath: 'beli_nft/target/ink/beli_nft.contract',
    dockerPath: 'beli_nft/beli_nft.contract',
    constructor: 'new',
    args: []
  },
  dao: {
    name: 'Simple DAO',
    localPath: 'simple_dao/target/ink/simple_dao.contract',
    dockerPath: 'simple_dao/simple_dao.contract',
    constructor: 'new',
    args: [] // Will be set to DALLA token address after deployment
  },
  faucet: {
    name: 'Testnet Faucet',
    localPath: 'faucet/target/ink/faucet.contract',
    dockerPath: 'faucet/faucet.contract',
    constructor: 'new',
    args: [100000000000000, 100] // 100 DALLA per claim (100 * 10^12), 100 blocks (~10 min) cooldown
  }
};

// Network configurations
const NETWORKS = {
  local: {
    url: process.env.BLOCKCHAIN_WS_URL || process.env.BELIZECHAIN_NODE_URL || 'ws://localhost:9944',
    account: process.env.DEPLOY_ACCOUNT || '//Alice'
  },
  testnet: {
    url: process.env.BLOCKCHAIN_WS_URL || process.env.BELIZECHAIN_TESTNET_URL || 'ws://belizechain-node.belizechain.svc.cluster.local:9944',
    account: process.env.DEPLOY_ACCOUNT || null
  },
  mainnet: {
    url: process.env.BLOCKCHAIN_WS_URL || process.env.BELIZECHAIN_MAINNET_URL || 'wss://rpc.belizechain.io',
    account: process.env.DEPLOY_ACCOUNT || null
  }
};

class Deployer {
  constructor(network, account) {
    this.network = NETWORKS[network];
    if (!this.network) {
      throw new Error(`Unknown network: ${network}. Use: local, testnet, or mainnet`);
    }
    this.accountUri = account || this.network.account;
    if (!this.accountUri) {
      throw new Error(`No account specified. Use --account=<uri> or set DEPLOY_ACCOUNT env var`);
    }
    this.api = null;
    this.signer = null;
    this.deployedAddresses = {};
  }

  async connect() {
    console.log(`🔌 Connecting to ${this.network.url}...`);

    try {
      const provider = new WsProvider(this.network.url, 5000); // 5 second timeout
      this.api = await ApiPromise.create({ provider });

      const [chain, nodeName, nodeVersion] = await Promise.all([
        this.api.rpc.system.chain(),
        this.api.rpc.system.name(),
        this.api.rpc.system.version()
      ]);

      console.log(`✅ Connected to ${chain} (${nodeName} v${nodeVersion})`);
      return true;
    } catch (error) {
      console.error(`❌ Connection failed: ${error.message}`);
      console.error(`\n💡 Make sure BelizeChain node is running at ${this.network.url}`);
      console.error(`   For local development, start a node with:`);
      console.error(`   substrate-contracts-node --dev --tmp`);
      return false;
    }
  }

  async setupAccount() {
    console.log(`🔑 Setting up account: ${this.accountUri}`);

    const keyring = new Keyring({ type: 'sr25519' });
    this.signer = keyring.addFromUri(this.accountUri, {}, 'sr25519');

    const { data: balance } = await this.api.query.system.account(this.signer.address);
    console.log(`   Address: ${this.signer.address}`);
    console.log(`   Balance: ${balance.free.toHuman()}`);

    // Check if account has sufficient balance
    const minBalance = 1000000000000; // 1 unit (adjust based on decimals)
    if (balance.free.toBigInt() < minBalance) {
      console.warn(`⚠️  Warning: Low balance! Deployment may fail.`);
    }
  }

  async deployContract(contractKey) {
    const config = CONTRACTS[contractKey];
    if (!config) {
      throw new Error(`Unknown contract: ${contractKey}`);
    }

    console.log(`\n📦 Deploying ${config.name}...`);

    // Resolve contract artifact path (Docker vs local)
    const isDocker = fs.existsSync('/app/artifacts');
    const relativePath = isDocker ? config.dockerPath : config.localPath;
    const contractPath = path.join(ARTIFACTS_DIR, relativePath);
    if (!fs.existsSync(contractPath)) {
      console.error(`❌ Contract file not found: ${contractPath}`);
      console.error(`   Build the contract first with: cargo contract build --release`);
      return null;
    }

    // Read contract file
    const contractData = JSON.parse(fs.readFileSync(contractPath, 'utf8'));
    const abi = contractData.source?.abi || contractData.abi || contractData;
    const wasm = contractData.source?.wasm || contractData.wasm;

    if (!wasm) {
      console.error(`❌ WASM not found in contract file`);
      return null;
    }

    try {
      // Upload code
      console.log(`   📤 Uploading contract code...`);
      const code = new CodePromise(this.api, abi, wasm);

      // Prepare constructor arguments
      let args = config.args;
      if (contractKey === 'dao' && this.deployedAddresses.dalla) {
        // DAO needs DALLA token address
        args = [this.deployedAddresses.dalla];
      }

      // Estimate gas
      const { gasRequired, storageDeposit } = await code.tx[config.constructor]({
        gasLimit: this.api.registry.createType('WeightV2', {
          refTime: -1,
          proofSize: -1
        }),
        storageDepositLimit: null
      }, ...args).dryRun(this.signer.address);

      console.log(`   ⛽ Gas required: ${gasRequired.refTime.toHuman()}`);
      console.log(`   💰 Storage deposit: ${storageDeposit.isCharge ? storageDeposit.asCharge.toHuman() : '0'}`);

      // Deploy
      console.log(`   🚀 Deploying contract...`);
      const tx = code.tx[config.constructor]({
        gasLimit: gasRequired,
        storageDepositLimit: storageDeposit.isCharge ? storageDeposit.asCharge : null
      }, ...args);

      return new Promise((resolve, reject) => {
        tx.signAndSend(this.signer, ({ status, contract, dispatchError }) => {
          if (dispatchError) {
            if (dispatchError.isModule) {
              const decoded = this.api.registry.findMetaError(dispatchError.asModule);
              const { docs, name, section } = decoded;
              console.error(`❌ Error: ${section}.${name}: ${docs.join(' ')}`);
            } else {
              console.error(`❌ Error: ${dispatchError.toString()}`);
            }
            reject(new Error('Deployment failed'));
          } else if (status.isInBlock) {
            console.log(`   ⏳ Included in block: ${status.asInBlock.toHex()}`);
          } else if (status.isFinalized) {
            if (contract) {
              console.log(`   ✅ Deployed at: ${contract.address.toString()}`);
              console.log(`   📋 Code hash: ${contract.codeHash.toHex()}`);

              this.deployedAddresses[contractKey] = contract.address.toString();

              resolve({
                address: contract.address.toString(),
                codeHash: contract.codeHash.toHex(),
                blockHash: status.asFinalized.toHex()
              });
            } else {
              reject(new Error('Contract deployed but address not available'));
            }
          }
        });
      });
    } catch (error) {
      console.error(`❌ Deployment error: ${error.message}`);
      return null;
    }
  }

  async deployAll() {
    const results = {};

    // Deploy in order (some contracts depend on others)
    const deployOrder = ['dalla', 'belinft', 'dao', 'faucet'];

    for (const contractKey of deployOrder) {
      const result = await this.deployContract(contractKey);
      if (result) {
        results[contractKey] = result;
      } else {
        console.warn(`⚠️  Skipping ${contractKey} deployment`);
      }
    }

    return results;
  }

  async saveDeploymentInfo(results) {
    const timestamp = new Date().toISOString();
    const deploymentInfo = {
      timestamp,
      network: this.network.url,
      deployer: this.signer.address,
      contracts: results
    };

    const outputPath = path.join(__dirname, '..', `deployment-${Date.now()}.json`);
    fs.writeFileSync(outputPath, JSON.stringify(deploymentInfo, null, 2));
    console.log(`\n💾 Deployment info saved to: ${outputPath}`);

    // Also update SDK contract addresses
    console.log(`\n📝 Add these addresses to your .env file:`);
    for (const [key, value] of Object.entries(results)) {
      console.log(`${key.toUpperCase()}_CONTRACT_ADDRESS=${value.address}`);
    }
  }

  async disconnect() {
    if (this.api) {
      await this.api.disconnect();
      console.log(`\n👋 Disconnected from BelizeChain`);
    }
  }
}

// Parse command line arguments
function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    contract: 'all',
    network: process.env.ENVIRONMENT || 'local',
    account: null
  };

  for (const arg of args) {
    if (arg.startsWith('--contract=')) {
      options.contract = arg.split('=')[1];
    } else if (arg.startsWith('--network=')) {
      options.network = arg.split('=')[1];
    } else if (arg.startsWith('--account=')) {
      options.account = arg.split('=')[1];
    } else if (arg === '--help' || arg === '-h') {
      console.log(`
BelizeChain Contract Deployment Script

Usage:
  node scripts/deploy.js [options]

Options:
  --contract=<name>   Contract to deploy: dalla, belinft, dao, faucet, all (default: all)
  --network=<name>    Network: local, testnet, mainnet (default: local)
  --account=<uri>     Account URI (default: //Alice for local)
  --help, -h          Show this help

Environment Variables:
  BLOCKCHAIN_WS_URL          WebSocket URL (preferred, set by K8s Job)
  BELIZECHAIN_NODE_URL       WebSocket URL for local node
  BELIZECHAIN_TESTNET_URL    WebSocket URL for testnet
  BELIZECHAIN_MAINNET_URL    WebSocket URL for mainnet
  DEPLOY_ACCOUNT             Account URI for deployment
  ENVIRONMENT                Network name: local, testnet, mainnet (default: local)

Examples:
  node scripts/deploy.js --contract=dalla --network=local
  node scripts/deploy.js --contract=all --network=testnet --account="//Bob"
  DEPLOY_ACCOUNT="//Alice" node scripts/deploy.js
      `);
      process.exit(0);
    }
  }

  return options;
}

// Main execution
async function main() {
  console.log('╔════════════════════════════════════════════════════════╗');
  console.log('║   BelizeChain GEM Contract Deployment                 ║');
  console.log('╚════════════════════════════════════════════════════════╝\n');

  const options = parseArgs();
  const deployer = new Deployer(options.network, options.account);

  try {
    // Connect to chain
    const connected = await deployer.connect();
    if (!connected) {
      process.exit(1);
    }

    // Setup deployer account
    await deployer.setupAccount();

    // Deploy contracts
    let results;
    if (options.contract === 'all') {
      results = await deployer.deployAll();
    } else {
      const result = await deployer.deployContract(options.contract);
      results = result ? { [options.contract]: result } : {};
    }

    // Save deployment info
    if (Object.keys(results).length > 0) {
      await deployer.saveDeploymentInfo(results);
      console.log('\n✅ Deployment completed successfully!');
    } else {
      console.log('\n❌ No contracts were deployed');
      process.exit(1);
    }
  } catch (error) {
    console.error(`\n❌ Deployment failed: ${error.message}`);
    console.error(error.stack);
    process.exit(1);
  } finally {
    await deployer.disconnect();
  }
}

// Run if called directly
if (require.main === module) {
  main().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}

module.exports = { Deployer, CONTRACTS, NETWORKS };

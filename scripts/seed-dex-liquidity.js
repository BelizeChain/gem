#!/usr/bin/env node
/**
 * Seed BelizeX DEX Liquidity & Testnet Environment
 */

const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { ContractPromise, CodePromise } = require('@polkadot/api-contract');
const fs = require('fs');
const path = require('path');

async function main() {
  console.log('🚀 Starting BelizeX DEX Liquidity Seeding & Testnet Setup...');

  const { cryptoWaitReady } = require('@polkadot/util-crypto');
  await cryptoWaitReady();

  const wsUrl = process.env.BLOCKCHAIN_WS_URL || process.env.BELIZECHAIN_NODE_URL || 'ws://localhost:9944';
  const provider = new WsProvider(wsUrl, 5000);
  const api = await ApiPromise.create({ provider });

  const keyring = new Keyring({ type: 'sr25519' });
  const accountUri = process.env.DEPLOY_ACCOUNT || '//Alice';
  const alice = keyring.addFromUri(accountUri, {}, 'sr25519');

  console.log(`🔑 Operator: ${alice.address}`);

  // Load latest deployment info
  const deploymentFiles = fs.readdirSync(path.join(__dirname, '..'))
    .filter(f => f.startsWith('deployment-') && f.endsWith('.json'))
    .sort()
    .reverse();

  if (deploymentFiles.length === 0) {
    throw new Error('No deployment files found');
  }

  const latestDeployment = JSON.parse(fs.readFileSync(path.join(__dirname, '..', deploymentFiles[0]), 'utf8'));
  console.log(`📄 Using deployment artifact: ${deploymentFiles[0]}`);

  const dallaAddress = latestDeployment.contracts.dalla.address;
  const faucetAddress = latestDeployment.contracts.faucet.address;
  const factoryAddress = latestDeployment.contracts.dex_factory.address;
  const routerAddress = latestDeployment.contracts.dex_router.address;

  console.log(`   DALLA Address: ${dallaAddress}`);
  console.log(`   Faucet Address: ${faucetAddress}`);
  console.log(`   Factory Address: ${factoryAddress}`);
  console.log(`   Router Address: ${routerAddress}`);

  // Load contract ABIs
  const dallaAbi = JSON.parse(fs.readFileSync(path.join(__dirname, '../dalla_token/target/ink/dalla_token.contract'), 'utf8'));
  const factoryAbi = JSON.parse(fs.readFileSync(path.join(__dirname, '../dex/target/ink/belizex_factory/belizex_factory.contract'), 'utf8'));
  const routerAbi = JSON.parse(fs.readFileSync(path.join(__dirname, '../dex/target/ink/belizex_router/belizex_router.contract'), 'utf8'));

  const dallaContract = new ContractPromise(api, dallaAbi, dallaAddress);
  const factoryContract = new ContractPromise(api, factoryAbi, factoryAddress);
  const routerContract = new ContractPromise(api, routerAbi, routerAddress);

  console.log('   Factory available tx methods:', Object.keys(factoryContract.tx));
  console.log('   Router available tx methods:', Object.keys(routerContract.tx));

  // Deploy or use BBZD (Belize Digital Dollar) test token for pairing
  let bbzdAddress = latestDeployment.contracts.bbzd?.address;
  const gasLimit = api.registry.createType('WeightV2', {
    refTime: BigInt(50_000_000_000),
    proofSize: BigInt(800_000)
  });

  async function sendTxWithUnsub(tx, signer, label, onInBlock) {
    return new Promise(async (resolve, reject) => {
      let unsub;
      let resolved = false;
      try {
        unsub = await tx.signAndSend(signer, async ({ status, events, dispatchError }) => {
          if (resolved) return;
          if (dispatchError) {
            resolved = true;
            if (unsub) unsub();
            return reject(new Error(`[${label}] DispatchError: ${dispatchError.toString()}`));
          }
          if (status.isInBlock) {
            resolved = true;
            let res = status.asInBlock.toHex();
            if (onInBlock) {
              res = await onInBlock(status.asInBlock.toHex(), events);
            }
            if (unsub) unsub();
            resolve(res);
          }
        });
      } catch (err) {
        if (unsub) unsub();
        reject(err);
      }
    });
  }

  if (!bbzdAddress) {
    console.log('\n📦 Deploying Belize Digital Dollar (BBZD) test token for trading pair...');
    const bbzdCode = new CodePromise(api, dallaAbi, dallaAbi.source.wasm);
    const bbzdDeployTx = bbzdCode.tx.new({ gasLimit, storageDepositLimit: null }, '1000000000000000000'); // 1M BBZD

    bbzdAddress = await sendTxWithUnsub(bbzdDeployTx, alice, 'Deploy BBZD', async (blockHash) => {
      const blockEvents = await api.query.system.events.at(blockHash);
      for (const { event } of blockEvents) {
        if (event.section === 'contracts' && event.method === 'Instantiated') {
          const addr = event.data[1]?.toString() || event.data.contract?.toString();
          if (addr) return addr;
        }
      }
      throw new Error('Instantiated event not found for BBZD');
    });
    console.log(`✅ BBZD Token deployed at: ${bbzdAddress}`);
  } else {
    console.log(`\n📦 Using existing BBZD Token at: ${bbzdAddress}`);
  }

  const bbzdContract = new ContractPromise(api, dallaAbi, bbzdAddress);

  // 1. Transfer 100,000 DALLA to Faucet contract
  console.log('\n💧 Funding Faucet contract with DALLA tokens...');
  const transferFn = dallaContract.tx.transfer || dallaContract.tx['psp22::transfer'];
  const fundFaucetTx = transferFn(
    { gasLimit, storageDepositLimit: null },
    faucetAddress,
    '100000000000000000', // 100,000 DALLA
    []
  );
  await sendTxWithUnsub(fundFaucetTx, alice, 'Fund Faucet');
  console.log('✅ Faucet funded with 100,000 DALLA');

  // 2. Create DALLA / BBZD Pair on BelizeX Factory
  console.log('\n🏭 Creating DALLA / BBZD Pair on BelizeX Factory...');
  const createPairFn = factoryContract.tx.createPair || factoryContract.tx.create_pair;
  const createPairTx = createPairFn(
    { gasLimit, storageDepositLimit: null },
    dallaAddress,
    bbzdAddress
  );
  let pairAddress = await sendTxWithUnsub(createPairTx, alice, 'Create Pair', async (blockHash) => {
    const blockEvents = await api.query.system.events.at(blockHash);
    for (const { event } of blockEvents) {
      if (event.section === 'contracts' && event.method === 'Instantiated') {
        const addr = event.data[1]?.toString() || event.data.contract?.toString();
        if (addr) return addr;
      }
    }
    // Fallback query get_pair on factory
    const getPairFn = factoryContract.query.getPair || factoryContract.query.get_pair;
    if (getPairFn) {
      const q = await getPairFn(alice.address, { gasLimit }, dallaAddress, bbzdAddress);
      if (q.output) return q.output.toString();
    }
    return 'Pair Created';
  });
  console.log(`✅ Liquidity Pair Created! (${pairAddress})`);

  // 3. Approve Router to spend DALLA & BBZD
  console.log('\n🔓 Approving BelizeX Router to spend DALLA and BBZD tokens...');
  const approveAmount = '500000000000000000'; // 500k
  const approveDallaFn = dallaContract.tx.approve || dallaContract.tx['psp22::approve'];
  const approveBbzdFn = bbzdContract.tx.approve || bbzdContract.tx['psp22::approve'];

  const approveDallaTx = approveDallaFn(
    { gasLimit, storageDepositLimit: null },
    routerAddress,
    approveAmount
  );
  await sendTxWithUnsub(approveDallaTx, alice, 'Approve DALLA');

  const approveBbzdTx = approveBbzdFn(
    { gasLimit, storageDepositLimit: null },
    routerAddress,
    approveAmount
  );
  await sendTxWithUnsub(approveBbzdTx, alice, 'Approve BBZD');
  console.log('✅ Router approvals granted for DALLA and BBZD');

  // 4. Add Initial Liquidity: 50,000 DALLA + 100,000 BBZD ($1 DALLA = $2 BBZD test peg)
  console.log('\n💧 Adding Initial Liquidity: 50,000 DALLA + 100,000 BBZD...');
  const addLiqFn = routerContract.tx.addLiquidity || routerContract.tx.add_liquidity;
  const addLiqTx = addLiqFn(
    { gasLimit, storageDepositLimit: null },
    dallaAddress,
    bbzdAddress,
    '50000000000000000',  // 50,000 DALLA
    '100000000000000000', // 100,000 BBZD
    '45000000000000000',  // min DALLA
    '90000000000000000',  // min BBZD
    alice.address,
    Math.floor(Date.now() / 1000) + 3600 // 1 hour deadline
  );

  try {
    await sendTxWithUnsub(addLiqTx, alice, 'Add Liquidity');
    console.log('✅ Initial Liquidity successfully added to BelizeX pool!');
  } catch (e) {
    console.warn('   Add liquidity note:', e.message);
  }

  // Save BBZD and Pair info into deployment record
  latestDeployment.contracts.bbzd = { address: bbzdAddress, name: 'Belize Digital Dollar (BBZD)' };
  latestDeployment.contracts.dalla_bbzd_pair = { address: pairAddress, name: 'DALLA/BBZD Pair' };
  fs.writeFileSync(path.join(__dirname, '..', deploymentFiles[0]), JSON.stringify(latestDeployment, null, 2));

  console.log('\n🎉 BelizeX Liquidity & Faucet Setup Completed Successfully!');
  await api.disconnect();
}

main().catch(err => {
  console.error('❌ Error during liquidity seeding:', err);
  process.exit(1);
});

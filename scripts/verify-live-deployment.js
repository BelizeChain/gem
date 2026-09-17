// One-shot: verify gem contract instantiations exist in live Ceiba chain state.
// Addresses are read from .env.testnet so the script stays valid across genesis resets.
const fs = require('fs');
const path = require('path');
const { WsProvider, ApiPromise } = require('@polkadot/api');
const { decodeAddress } = require('@polkadot/util-crypto');

const PROJECT_DIR = path.dirname(path.dirname(__filename));
const ENV_FILE = process.env.GEM_ENV_FILE || path.join(PROJECT_DIR, '.env.testnet');
const WS_URL = process.env.BLOCKCHAIN_WS_URL || 'ws://100.81.45.25:9944';

function loadEnvFile(filePath) {
  const values = {};
  for (const line of fs.readFileSync(filePath, 'utf8').split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const eq = trimmed.indexOf('=');
    if (eq === -1) continue;
    values[trimmed.slice(0, eq).trim()] = trimmed.slice(eq + 1).trim();
  }
  return values;
}

const ADDRESS_KEYS = [
  'DALLA_CONTRACT_ADDRESS',
  'BELINFT_CONTRACT_ADDRESS',
  'DAO_CONTRACT_ADDRESS',
  'FAUCET_CONTRACT_ADDRESS',
  'PSP37_CONTRACT_ADDRESS',
  'DEX_FACTORY_CONTRACT_ADDRESS',
  'DEX_ROUTER_CONTRACT_ADDRESS',
];

(async () => {
  const env = loadEnvFile(ENV_FILE);
  const api = await ApiPromise.create({ provider: new WsProvider(WS_URL) });
  const spec = await api.rpc.state.getRuntimeVersion();
  console.log('chain:', WS_URL, '| spec:', spec.specName.toString(), spec.specVersion.toNumber());
  console.log('contracts pallet in runtime:', !!api.query.contracts);

  let found = 0;
  for (const key of ADDRESS_KEYS) {
    const addr = env[key];
    if (!addr) {
      console.log(`${key}: not set in ${path.basename(ENV_FILE)} — skipped`);
      continue;
    }
    const hex = '0x' + Buffer.from(decodeAddress(addr)).toString('hex');
    const info = await api.query.contracts.contractInfoOf(hex);
    const exists = info.isSome;
    if (exists) found += 1;
    console.log(`${key}: ${addr} -> ${exists ? 'DEPLOYED' : 'MISSING — not in live state'}`);
  }
  console.log(`\n${found}/${ADDRESS_KEYS.length} contracts verified live.`);
  process.exit(found === ADDRESS_KEYS.length ? 0 : 2);
})().catch((e) => {
  console.error('ERR:', e.message);
  process.exit(1);
});

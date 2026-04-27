# GEM Ceiba Deploy Handoff

This handoff is for the current Ceiba contract deployment path for GEM.

## Current State

- Ceiba RPC is healthy at `ws://100.81.45.25:9944`.
- `pallet-contracts` is available on the current Ceiba node.
- GEM deployable artifacts build successfully from this repo.
- Standard dev accounts like `//Alice` are **not funded** on the active Ceiba chain.
- A live on-chain sudo key exists, but the signer material is not available in this repo.

Because of that, live deployment is blocked until Ceiba ops funds a dedicated deployer account or provides an existing funded signer through a secure out-of-band path.

## Inputs Ops Must Provide

Provide these outside git, never commit them:

- `DEPLOY_ACCOUNT`: funded signer URI, mnemonic, or other secret accepted by `@polkadot/keyring`
- `BLOCKCHAIN_WS_URL`: `ws://100.81.45.25:9944`

Optional:

- `BELIZECHAIN_TESTNET_URL`: same value as `BLOCKCHAIN_WS_URL` if you prefer the testnet env name

## Preflight Checks

Run these from the GEM repo root.

```bash
NODE_PATH="$PWD/sdk/node_modules" node scripts/check-node.js --url=ws://100.81.45.25:9944

bash scripts/build-all.sh
```

Expected outcome:

- node health check passes
- all contract artifacts exist under `*/target/ink/`

## Funding Check

Before deployment, confirm the deployer has native balance on Ceiba.

```bash
NODE_PATH="$PWD/sdk/node_modules" node - <<'NODE'
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');

(async () => {
  const accountUri = process.env.DEPLOY_ACCOUNT;
  if (!accountUri) {
    throw new Error('DEPLOY_ACCOUNT is required');
  }

  const api = await ApiPromise.create({ provider: new WsProvider(process.env.BLOCKCHAIN_WS_URL || 'ws://100.81.45.25:9944', 5000) });
  const keyring = new Keyring({ type: 'sr25519' });
  const pair = keyring.addFromUri(accountUri, {}, 'sr25519');
  const { data } = await api.query.system.account(pair.address);

  console.log(`ADDRESS ${pair.address}`);
  console.log(`FREE ${data.free.toString()}`);

  await api.disconnect();
})();
NODE
```

If `FREE` is `0`, stop and fund the deployer first.

## Exact Deployment Commands

Deploy all four primary contracts:

```bash
export BLOCKCHAIN_WS_URL=ws://100.81.45.25:9944
export DEPLOY_ACCOUNT='<FUNDED_SIGNER_URI>'

node scripts/deploy.js --contract=all --network=testnet --account="$DEPLOY_ACCOUNT"
```

Or use the wrapper:

```bash
export BLOCKCHAIN_WS_URL=ws://100.81.45.25:9944
export DEPLOY_ACCOUNT='<FUNDED_SIGNER_URI>'

bash scripts/deploy-testnet.sh
```

## Post-Deploy Recording

Take the deployed addresses from the script output and update `.env.testnet`:

```dotenv
BLOCKCHAIN_WS_URL=ws://100.81.45.25:9944

DALLA_CONTRACT_ADDRESS=<NEW_DALLA_ADDRESS>
BELINFT_CONTRACT_ADDRESS=<NEW_BELINFT_ADDRESS>
DAO_CONTRACT_ADDRESS=<NEW_DAO_ADDRESS>
FAUCET_CONTRACT_ADDRESS=<NEW_FAUCET_ADDRESS>
```

## On-Chain Verification

Verify the recorded addresses exist on the active Ceiba chain:

```bash
NODE_PATH="$PWD/sdk/node_modules" node - <<'NODE'
const { ApiPromise, WsProvider } = require('@polkadot/api');

const addresses = {
  DALLA: process.env.DALLA_CONTRACT_ADDRESS,
  BELINFT: process.env.BELINFT_CONTRACT_ADDRESS,
  DAO: process.env.DAO_CONTRACT_ADDRESS,
  FAUCET: process.env.FAUCET_CONTRACT_ADDRESS,
};

(async () => {
  const api = await ApiPromise.create({ provider: new WsProvider(process.env.BLOCKCHAIN_WS_URL || 'ws://100.81.45.25:9944', 5000) });

  for (const [name, address] of Object.entries(addresses)) {
    if (!address) {
      console.log(`${name} MISSING_ENV`);
      continue;
    }

    const info = await api.query.contracts.contractInfoOf(address);
    console.log(`${name} ${address} ${info.isSome ? 'PRESENT' : 'MISSING'}`);
  }

  await api.disconnect();
})();
NODE
```

## Minimal Ops Handoff

Send this to whoever controls Ceiba funding:

```text
Gem is build-ready and Ceiba RPC is healthy at ws://100.81.45.25:9944, but the standard dev deployers are unfunded on the live chain. Please fund a dedicated GEM deployer account with enough native balance for contract upload and instantiation, then share the funded signer with the GEM deployment operator out-of-band. After funding, run the commands in docs/guides/CEIBA_DEPLOY_HANDOFF.md from the gem repo and write the resulting addresses back into .env.testnet.
```
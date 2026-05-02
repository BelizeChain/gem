# GEM Ceiba Deploy Handoff

This handoff is for the current Ceiba contract deployment path for GEM.

## Current State

- Ceiba RPC is healthy at `ws://100.81.45.25:9944`.
- `pallet-contracts` is available on the current Ceiba node.
- GEM deployable artifacts build successfully from this repo.
- The current Ceiba reset chain has a funded testnet deployer available as `//treasury`.
- Primary GEM contracts were deployed and verified on 2026-05-02.

Current verified addresses:

```dotenv
DALLA_CONTRACT_ADDRESS=r1SAvDb2f5iFbafWL87rE1jP6QV3qCV5xtK8b1QXuqKpGknj5
BELINFT_CONTRACT_ADDRESS=r1Wywor1ittVCyZeYaA9hweBwuDiXLC2UdLNQm4oUB1ub8qyN
DAO_CONTRACT_ADDRESS=r1VnpeWtfLmtZ2W2UJhYXSLoHhwo7tAY48RZyVirRu5ucLi7i
FAUCET_CONTRACT_ADDRESS=r1TDXUdxgeLC5BAkFQeZnZNSAX67FwRAaavmG19TzPtc2Szcg
```

Redeploy only after a chain reset, artifact change, or explicit address rotation.

## Inputs Ops Must Provide

For a future redeploy, provide these outside git, never commit production secrets:

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
export DEPLOY_ACCOUNT='//treasury'

node scripts/deploy.js --contract=all --network=testnet --account="$DEPLOY_ACCOUNT"
```

Or use the wrapper:

```bash
export BLOCKCHAIN_WS_URL=ws://100.81.45.25:9944
export DEPLOY_ACCOUNT='//treasury'

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
Gem is deployed on the current Ceiba testnet and verified through contracts.contractInfoOf. If Ceiba is reset again or artifacts change, redeploy from the gem repo with a funded deployer, then write the resulting addresses back into .env.testnet and the UI/infra env wiring.
```
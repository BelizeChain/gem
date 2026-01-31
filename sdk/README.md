# BelizeChain Gem SDK 💎

**JavaScript/TypeScript SDK for BelizeChain smart contracts**

[![npm version](https://img.shields.io/npm/v/@belizechain/gem-sdk.svg)](https://www.npmjs.com/package/@belizechain/gem-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## Features

✅ **Easy-to-use API** - Simple methods for all contract interactions  
✅ **Full TypeScript support** - Complete type definitions included  
✅ **PSP22 & PSP34** - Token and NFT standard helpers  
✅ **DAO governance** - Proposal creation and voting  
✅ **Testnet faucet** - Automated token distribution  
✅ **Contract ABIs included** - No manual ABI management  

---

## Installation

```bash
npm install @belizechain/gem-sdk @polkadot/api @polkadot/api-contract
```

Or with Yarn:

```bash
yarn add @belizechain/gem-sdk @polkadot/api @polkadot/api-contract
```

---

## Quick Start

### 1. Connect to BelizeChain

```javascript
const { GemSDK } = require('@belizechain/gem-sdk');

// Create SDK instance
const sdk = new GemSDK('ws://localhost:9944');

// Connect to node
await sdk.connect();
```

### 2. Transfer DALLA Tokens (PSP22)

```javascript
// Get account from seed
const alice = sdk.getAccount('//Alice');

// Transfer 100 DALLA
await sdk.dallaTransfer(
    '5GD4w5...NVsNB',  // DALLA contract address
    alice,              // Sender
    '5FHneW...JM694ty', // Recipient
    100000000000000     // Amount (100 DALLA with 12 decimals)
);
```

### 3. Mint an NFT (PSP34)

```javascript
const alice = sdk.getAccount('//Alice');

// Mint NFT with IPFS URI
await sdk.nftMint(
    '5Ho6Ks...iFQL7',           // BeliNFT contract address
    alice,                       // Minter (owner)
    '5FHneW...JM694ty',          // Recipient
    'ipfs://QmYourImageHash'     // Token URI
);
```

### 4. Create a DAO Proposal

```javascript
const alice = sdk.getAccount('//Alice');

// Create proposal
const proposalId = await sdk.daoCreateProposal(
    '5ExampleDAOAddress',
    alice,
    'Increase treasury allocation for infrastructure'
);

console.log(`Proposal created: #${proposalId}`);
```

### 5. Claim from Faucet

```javascript
const bob = sdk.getAccount('//Bob');

// Check if can claim
const canClaim = await sdk.faucetCanClaim(
    '5FaucetAddress',
    bob.address
);

if (canClaim) {
    // Claim 1000 DALLA
    await sdk.faucetClaim('5FaucetAddress', bob);
}
```

---

## API Reference

### Constructor

```javascript
new GemSDK(nodeUrl?: string)
```

Create a new SDK instance.

**Parameters:**
- `nodeUrl` (optional): WebSocket URL (default: `ws://localhost:9944`)

---

### Connection Methods

#### `connect()`

Connect to BelizeChain node.

```javascript
await sdk.connect();
```

**Returns:** `Promise<ApiPromise>`

---

#### `disconnect()`

Disconnect from node.

```javascript
await sdk.disconnect();
```

---

### Account Management

#### `getAccount(uri)`

Get account from seed phrase or URI.

```javascript
const alice = sdk.getAccount('//Alice');
const bob = sdk.getAccount('//Bob');
```

**Parameters:**
- `uri`: Account URI, seed phrase, or private key

**Returns:** `KeyringPair`

---

#### `getBalance(address)`

Get account balance.

```javascript
const balance = await sdk.getBalance(alice.address);
console.log('Free balance:', balance.free);
```

**Returns:** `Promise<{ free: string, reserved: string, frozen: string }>`

---

### DALLA Token (PSP22) Methods

#### `dallaTransfer(contractAddress, signer, to, amount, gasLimit?)`

Transfer DALLA tokens.

```javascript
await sdk.dallaTransfer(
    '5GD4w5...NVsNB',
    alice,
    bob.address,
    1000000000000  // 1 DALLA
);
```

---

#### `dallaBalanceOf(contractAddress, account)`

Get token balance.

```javascript
const balance = await sdk.dallaBalanceOf(
    '5GD4w5...NVsNB',
    alice.address
);
```

---

#### `dallaMetadata(contractAddress)`

Get token metadata.

```javascript
const metadata = await sdk.dallaMetadata('5GD4w5...NVsNB');
console.log('Name:', metadata.name);
console.log('Symbol:', metadata.symbol);
console.log('Decimals:', metadata.decimals);
console.log('Total Supply:', metadata.totalSupply);
```

---

### BeliNFT (PSP34) Methods

#### `nftMint(contractAddress, signer, to, uri)`

Mint new NFT.

```javascript
await sdk.nftMint(
    '5Ho6Ks...iFQL7',
    alice,
    bob.address,
    'ipfs://Qm...'
);
```

---

#### `nftOwnerOf(contractAddress, tokenId)`

Get NFT owner.

```javascript
const owner = await sdk.nftOwnerOf('5Ho6Ks...iFQL7', 1);
```

---

#### `nftTokenUri(contractAddress, tokenId)`

Get NFT metadata URI.

```javascript
const uri = await sdk.nftTokenUri('5Ho6Ks...iFQL7', 1);
console.log('IPFS URI:', uri);
```

---

### DAO Methods

#### `daoCreateProposal(contractAddress, signer, description)`

Create DAO proposal.

```javascript
const proposalId = await sdk.daoCreateProposal(
    '5DAOAddress',
    alice,
    'Proposal description'
);
```

---

#### `daoVote(contractAddress, signer, proposalId, support)`

Vote on proposal.

```javascript
await sdk.daoVote(
    '5DAOAddress',
    alice,
    0,      // Proposal ID
    true    // true = for, false = against
);
```

---

#### `daoGetProposal(contractAddress, proposalId)`

Get proposal details.

```javascript
const proposal = await sdk.daoGetProposal('5DAOAddress', 0);
console.log('Description:', proposal.description);
console.log('Votes For:', proposal.votesFor);
console.log('Votes Against:', proposal.votesAgainst);
console.log('Status:', proposal.status);
```

---

### Faucet Methods

#### `faucetClaim(contractAddress, signer)`

Claim tokens from faucet.

```javascript
await sdk.faucetClaim('5FaucetAddress', bob);
```

---

#### `faucetCanClaim(contractAddress, account)`

Check if account can claim.

```javascript
const canClaim = await sdk.faucetCanClaim(
    '5FaucetAddress',
    bob.address
);
```

---

#### `faucetStats(contractAddress)`

Get faucet statistics.

```javascript
const stats = await sdk.faucetStats('5FaucetAddress');
console.log('Total Claimed:', stats.totalClaimed);
console.log('Claim Count:', stats.claimCount);
console.log('Drip Amount:', stats.dripAmount);
console.log('Cooldown:', stats.cooldown);
```

---

## Complete Example

```javascript
const { GemSDK } = require('@belizechain/gem-sdk');

async function main() {
    // 1. Connect
    const sdk = new GemSDK('ws://localhost:9944');
    await sdk.connect();

    // 2. Get accounts
    const alice = sdk.getAccount('//Alice');
    const bob = sdk.getAccount('//Bob');

    // 3. Check balances
    const aliceBalance = await sdk.getBalance(alice.address);
    console.log('Alice balance:', aliceBalance.free);

    // 4. Transfer DALLA tokens
    await sdk.dallaTransfer(
        '5GD4w5...NVsNB',
        alice,
        bob.address,
        1000000000000000  // 1000 DALLA
    );

    // 5. Mint NFT
    await sdk.nftMint(
        '5Ho6Ks...iFQL7',
        alice,
        bob.address,
        'ipfs://QmYourHash'
    );

    // 6. Create DAO proposal
    const proposalId = await sdk.daoCreateProposal(
        '5DAOAddress',
        alice,
        'Increase developer funding'
    );

    // 7. Vote on proposal
    await sdk.daoVote(
        '5DAOAddress',
        alice,
        proposalId,
        true  // Vote for
    );

    // 8. Disconnect
    await sdk.disconnect();
}

main().catch(console.error);
```

---

## TypeScript Support

The SDK includes full TypeScript definitions:

```typescript
import { GemSDK } from '@belizechain/gem-sdk';

const sdk: GemSDK = new GemSDK('ws://localhost:9944');
await sdk.connect();

const balance: string = await sdk.dallaBalanceOf(
    '5GD4w5...NVsNB',
    alice.address
);
```

---

## Contract Addresses

**BelizeChain Testnet:**

| Contract | Address |
|----------|---------|
| DALLA Token | `5GD4w5DP6VUBtFt7F9LB9EDzGcpbzFwvR9CWVNVsNB` |
| BeliNFT | `5Ho6KsLW7F8vBUWg3uz97sTu6iwxr665ucLtFzn2i8riFQL7` |
| Simple DAO | *Awaiting deployment* |
| Faucet | *Awaiting deployment* |

---

## Examples

See the `examples/` directory for complete working examples:

- `transfer.js` - Transfer DALLA tokens
- `mint-nft.js` - Mint and transfer NFTs
- `dao-vote.js` - Create proposals and vote
- `faucet.js` - Claim testnet tokens
- `test-connection.js` - Test node connection

---

## Development

### Run Examples

```bash
# Test connection
node examples/test-connection.js

# Transfer tokens
node examples/transfer.js

# Mint NFT
node examples/mint-nft.js
```

### Add Contract ABIs

Contract ABIs are automatically included from compiled contracts:

```bash
cp gem/dalla_token/target/ink/dalla.json sdk/contracts/
cp gem/beli_nft/target/ink/belinft.json sdk/contracts/
cp gem/simple_dao/target/ink/dao.json sdk/contracts/
cp gem/faucet/target/ink/faucet.json sdk/contracts/
```

---

## Support

- 📖 [Full Documentation](https://github.com/BelizeChain/gem)
- 💬 [Discord Community](https://discord.gg/belizechain)
- 🐛 [Report Issues](https://github.com/BelizeChain/gem/issues)
- 📧 Email: dev@belizechain.io

---

## License

MIT © BelizeChain Team

---

**Built with ❤️ in Belize 🇧🇿**

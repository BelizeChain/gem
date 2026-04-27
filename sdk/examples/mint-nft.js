/**
 * Mint and transfer NFTs
 *
 * Usage: node examples/mint-nft.js
 */

const { GemSDK } = require('../index');

// CONFIGURATION - Update these values
const BELINFT_CONTRACT = process.env.BELINFT_CONTRACT_ADDRESS || 'REPLACE_WITH_BELINFT_CONTRACT_ADDRESS';
const NODE_URL = process.env.BLOCKCHAIN_WS_URL || 'ws://100.81.45.25:9944';

async function main() {
  console.log('🎨 BeliNFT Minting Example\n');

  const sdk = new GemSDK(NODE_URL);
  await sdk.connect();

  // Get accounts
  const alice = sdk.getAccount('//Alice');
  const bob = sdk.getAccount('//Bob');

  console.log(`Minter (Owner): ${alice.address}`);
  console.log(`Recipient: ${bob.address}\n`);

  try {
    // Mint NFT #1
    console.log('🎨 Minting NFT #1...');
    await sdk.nftMint(
      BELINFT_CONTRACT,
      alice,
      bob.address,
      'ipfs://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG'
    );

    console.log('✅ NFT #1 minted to Bob\n');

    // Mint NFT #2
    console.log('🎨 Minting NFT #2...');
    await sdk.nftMint(
      BELINFT_CONTRACT,
      alice,
      alice.address,
      'ipfs://QmRSXYd3Zxm1RZQfjh3H2P7fUdSKQYZfZ3sVVV6TgJCxZM'
    );

    console.log('✅ NFT #2 minted to Alice\n');

    // Query NFT owners
    console.log('📊 Checking NFT ownership:');

    const owner1 = await sdk.nftOwnerOf(BELINFT_CONTRACT, 1);
    const uri1 = await sdk.nftTokenUri(BELINFT_CONTRACT, 1);
    console.log(`   NFT #1: ${owner1}`);
    console.log(`   URI: ${uri1}`);

    const owner2 = await sdk.nftOwnerOf(BELINFT_CONTRACT, 2);
    const uri2 = await sdk.nftTokenUri(BELINFT_CONTRACT, 2);
    console.log(`   NFT #2: ${owner2}`);
    console.log(`   URI: ${uri2}\n`);

    console.log('✅ NFT minting complete!\n');
  } catch (error) {
    console.error('\n❌ Minting failed:', error.message);
  }

  await sdk.disconnect();
}

main().catch(console.error);

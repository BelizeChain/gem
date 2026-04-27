/**
 * Create DAO proposals and vote
 *
 * Usage: node examples/dao-vote.js
 */

const { GemSDK } = require('../index');

// CONFIGURATION - Update these values
const DAO_CONTRACT = process.env.DAO_CONTRACT_ADDRESS || 'REPLACE_WITH_DAO_CONTRACT_ADDRESS';
const NODE_URL = process.env.BLOCKCHAIN_WS_URL || 'ws://100.81.45.25:9944';

async function main() {
  console.log('🗳️  DAO Governance Example\n');

  const sdk = new GemSDK(NODE_URL);
  await sdk.connect();

  // Get accounts
  const alice = sdk.getAccount('//Alice');
  const bob = sdk.getAccount('//Bob');
  const charlie = sdk.getAccount('//Charlie');

  console.log(`Proposer: ${alice.address}`);
  console.log(`Voter 1: ${bob.address}`);
  console.log(`Voter 2: ${charlie.address}\n`);

  try {
    // Create proposal
    console.log('📝 Creating proposal...');
    const proposalId = await sdk.daoCreateProposal(
      DAO_CONTRACT,
      alice,
      'Increase developer treasury allocation by 10%'
    );

    console.log(`✅ Proposal #${proposalId} created\n`);

    // Get proposal details
    console.log('📊 Proposal Details:');
    const proposal = await sdk.daoGetProposal(DAO_CONTRACT, proposalId);
    console.log(`   ID: ${proposalId}`);
    console.log(`   Proposer: ${proposal.proposer}`);
    console.log(`   Description: ${proposal.description}`);
    console.log(`   Status: ${proposal.status}\n`);

    // Vote FOR (Alice)
    console.log('🗳️  Alice voting FOR...');
    await sdk.daoVote(DAO_CONTRACT, alice, proposalId, true);
    console.log('✅ Vote cast\n');

    // Vote FOR (Bob)
    console.log('🗳️  Bob voting FOR...');
    await sdk.daoVote(DAO_CONTRACT, bob, proposalId, true);
    console.log('✅ Vote cast\n');

    // Vote AGAINST (Charlie)
    console.log('🗳️  Charlie voting AGAINST...');
    await sdk.daoVote(DAO_CONTRACT, charlie, proposalId, false);
    console.log('✅ Vote cast\n');

    // Get final results
    console.log('📊 Final Vote Count:');
    const finalProposal = await sdk.daoGetProposal(DAO_CONTRACT, proposalId);
    console.log(`   Votes For: ${finalProposal.votesFor}`);
    console.log(`   Votes Against: ${finalProposal.votesAgainst}`);
    console.log(`   Status: ${finalProposal.status}\n`);

    console.log('✅ Voting complete!\n');
  } catch (error) {
    console.error('\n❌ Operation failed:', error.message);
    console.error('\nMake sure DAO contract is deployed and address is correct.');
  }

  await sdk.disconnect();
}

main().catch(console.error);

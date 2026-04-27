/**
 * Privacy-Preserving Payroll Example
 * Demonstrates how to use GEM SDK with privacy commitments
 */

const { GemSDK, PrivacySDK } = require('@belizechain/gem-sdk');

const NODE_URL = process.env.BLOCKCHAIN_WS_URL || 'ws://100.81.45.25:9944';

async function main() {
  // 1. Connect to BelizeChain
  const sdk = new GemSDK(NODE_URL);
  await sdk.connect();

  // 2. Initialize privacy SDK
  const privacySDK = new PrivacySDK(sdk.api);

  // 3. Get accounts
  const employer = sdk.getAccount('//Alice'); // Government employer
  const employee = sdk.getAccount('//Bob'); // Employee

  console.log(`Employer: ${employer.address}`);
  console.log(`Employee: ${employee.address}`);

  // 4. Compute salary commitment (keeps salary private)
  console.log('\n🔐 Computing salary commitment...');
  const salaryAmount = '5000000000000000'; // 5000 DALLA (BZD 5,000/month)
  const salaryCommitment = privacySDK.computeSalaryCommitment(
    salaryAmount,
    employer.address,
    employee.address
  );
  console.log(`Salary Commitment: ${salaryCommitment}`);
  console.log('✅ Salary is now private (hidden via commitment hash)');

  // 5. Validate commitment
  console.log('\n✔️  Validating commitment...');
  const validation = privacySDK.validateCommitment(salaryCommitment);
  console.log(`Valid: ${validation.valid}`);
  console.log(`Reason: ${validation.reason}`);

  // 6. Get payroll record (privacy-preserving)
  console.log('\n📋 Fetching payroll record...');
  const payrollRecord = await privacySDK.getPayrollRecord(employee.address);

  if (payrollRecord) {
    console.log('Payroll Record (Privacy-Preserving):');
    console.log(`  - Employee ID: ${payrollRecord.employeeId}`);
    console.log(`  - Employer: ${payrollRecord.employer}`);
    console.log(`  - Department: ${payrollRecord.department}`);
    console.log(`  - Worker Type: ${payrollRecord.workerType}`);
    console.log(`  - Salary: **HIDDEN** (commitment: ${payrollRecord.salaryCommitment})`);
    console.log(`  - Joined: ${new Date(payrollRecord.joinedAt * 1000).toLocaleDateString()}`);
    console.log(`  - Active: ${payrollRecord.isActive}`);
    console.log(`  - Payments: ${payrollRecord.paymentCount}`);
  } else {
    console.log('❌ No payroll record found');
  }

  // 7. Verify salary commitment (only employee/employer know actual amount)
  console.log('\n🔍 Verifying salary commitment...');
  const isValid = privacySDK.verifySalaryCommitment(
    salaryCommitment,
    salaryAmount,
    employer.address,
    employee.address
  );
  console.log(`Commitment matches: ${isValid ? '✅ YES' : '❌ NO'}`);

  // 8. Compute payment commitment (hides individual payment amounts)
  console.log('\n💰 Computing payment commitment...');
  const grossAmount = salaryAmount;
  const deductions = [
    { type: 'IncomeTax', amount: '1250000000000000' }, // 25% tax
    { type: 'SocialSecurity', amount: '250000000000000' }, // 5% SS
  ];
  const netAmount = '3500000000000000'; // Net: 3500 DALLA

  const paymentCommitment = privacySDK.computePaymentCommitment(
    grossAmount,
    deductions,
    netAmount,
    employee.address,
    Date.now()
  );
  console.log(`Payment Commitment: ${paymentCommitment}`);
  console.log('✅ Payment details are private (amounts hidden)');

  // 9. Get payment history (privacy-preserving)
  console.log('\n📜 Fetching payment history...');
  const paymentHistory = await privacySDK.getPaymentHistory(employee.address);

  console.log(`Found ${paymentHistory.length} payments:`);
  paymentHistory.forEach((payment, index) => {
    console.log(`\nPayment ${index + 1}:`);
    console.log(`  - Payment ID: ${payment.paymentId}`);
    console.log(`  - Amounts: **HIDDEN** (commitment: ${payment.paymentCommitment})`);
    console.log(`  - Timestamp: ${new Date(payment.timestamp * 1000).toLocaleString()}`);
    console.log(`  - Status: ${payment.status}`);
  });

  // 10. Demonstrate ZK-style vote commitment (future DAO privacy)
  console.log('\n🗳️  Computing ZK-style vote commitment (roadmap 2028)...');
  const salt = PrivacySDK.generateSalt();
  const voteCommitment = privacySDK.computeVoteCommitment(
    42, // Proposal ID
    true, // Vote: Aye
    salt
  );
  console.log(`Vote Commitment: ${voteCommitment}`);
  console.log(`Salt (keep secret): ${salt}`);
  console.log('✅ Vote is private until reveal phase');

  // 11. Compute computation commitment (for staking rewards)
  console.log('\n🧮 Computing computation commitment (PoUW)...');
  const modelDelta = new Uint8Array(32).fill(42); // Simulated encrypted delta
  const computationCommitment = privacySDK.computeComputationCommitment(
    modelDelta,
    employee.address,
    Date.now()
  );
  console.log(`Computation Commitment: ${computationCommitment}`);
  console.log('✅ Model delta is private (computation verified via commitment)');

  // 12. Demonstrate batch payment commitment
  console.log('\n📦 Computing batch payment commitment...');
  const paymentCommitments = [paymentCommitment, paymentCommitment, paymentCommitment];
  const batchCommitment = privacySDK.computeBatchCommitment(
    paymentCommitments,
    '10500000000000000', // Total: 10,500 DALLA (3 employees)
    Date.now()
  );
  console.log(`Batch Commitment: ${batchCommitment}`);
  console.log('✅ Batch total is public, individual amounts remain private');

  // 13. Monitor privacy-aware events
  console.log('\n👀 Monitoring privacy-aware payroll events...');
  sdk.api.query.system.events((events) => {
    events.forEach((record) => {
      const { event } = record;
      if (event.section === 'payroll') {
        console.log(`\n🔔 Payroll Event: ${event.method}`);
        const eventData = event.data.toHuman();

        // Show commitments instead of plaintext amounts
        if (eventData.salaryCommitment) {
          console.log(`   Salary: **HIDDEN** (commitment: ${eventData.salaryCommitment})`);
        }
        if (eventData.paymentCommitment) {
          console.log(`   Payment: **HIDDEN** (commitment: ${eventData.paymentCommitment})`);
        }
      }
    });
  });

  console.log('\nPress Ctrl+C to exit...');
}

// Run the example
main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});

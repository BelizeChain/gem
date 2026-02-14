/**
 * Privacy SDK Extension for BelizeChain GEM
 * Provides privacy-preserving utilities for working with commitment hashes
 *
 * @module @belizechain/gem-sdk/privacy
 * @version 1.1.0
 */

const { blake2AsHex } = require('@polkadot/util-crypto');

/**
 * Privacy SDK extension
 */
class PrivacySDK {
  constructor(api) {
    if (!api) {
      throw new Error('API instance required');
    }
    this.api = api;
  }

  /**
   * Compute salary commitment hash (as used in pallet-belize-payroll)
   * @param {string} salary - Salary amount (in base units)
   * @param {string} employerId - Employer account ID
   * @param {string} employeeId - Employee account ID
   * @returns {string} Commitment hash (32 bytes hex)
   */
  computeSalaryCommitment(salary, employerId, employeeId) {
    const data = `${salary}:${employerId}:${employeeId}`;
    return blake2AsHex(data, 256);
  }

  /**
   * Compute payment commitment hash
   * @param {string} grossAmount - Gross payment amount
   * @param {Array<Object>} deductions - Array of { type, amount }
   * @param {string} netAmount - Net payment amount
   * @param {string} employeeId - Employee account ID
   * @param {number} timestamp - Payment timestamp
   * @returns {string} Commitment hash (32 bytes hex)
   */
  computePaymentCommitment(grossAmount, deductions, netAmount, employeeId, timestamp) {
    const deductionsStr = deductions.map((d) => `${d.type}:${d.amount}`).join(',');

    const data = `${grossAmount}:${deductionsStr}:${netAmount}:${employeeId}:${timestamp}`;
    return blake2AsHex(data, 256);
  }

  /**
   * Compute batch payment commitment
   * @param {Array<string>} paymentCommitments - Individual payment commitments
   * @param {string} totalAmount - Total batch amount
   * @param {number} timestamp - Batch timestamp
   * @returns {string} Batch commitment hash
   */
  computeBatchCommitment(paymentCommitments, totalAmount, timestamp) {
    const commitmentsStr = paymentCommitments.join(',');
    const data = `${commitmentsStr}:${totalAmount}:${timestamp}`;
    return blake2AsHex(data, 256);
  }

  /**
   * Get payroll record by commitment (privacy-preserving)
   * @param {string} employeeId - Employee account ID
   * @returns {Promise<Object|null>} Payroll record with commitments
   */
  async getPayrollRecord(employeeId) {
    const record = await this.api.query.payroll.employees(employeeId);

    if (record.isNone) {
      return null;
    }

    const employee = record.unwrap();
    return {
      employeeId,
      employer: employee.employer.toString(),
      department: employee.department.toString(),
      workerType: employee.workerType.toString(),
      salaryCommitment: employee.salaryCommitment.toHex(), // Commitment instead of plaintext
      joinedAt: employee.joinedAt.toNumber(),
      isActive: employee.isActive.valueOf(),
      paymentCount: employee.paymentCount.toNumber(),
    };
  }

  /**
   * Get payment history with commitments (privacy-preserving)
   * @param {string} employeeId - Employee account ID
   * @returns {Promise<Array>} Payment records with commitments
   */
  async getPaymentHistory(employeeId) {
    const payments = await this.api.query.payroll.paymentHistory(employeeId);

    if (!payments || payments.length === 0) {
      return [];
    }

    return payments.map((payment, index) => ({
      paymentId: index,
      paymentCommitment: payment.paymentCommitment.toHex(), // Commitment instead of amounts
      timestamp: payment.timestamp.toNumber(),
      status: payment.status.toString(),
    }));
  }

  /**
   * Verify salary commitment locally
   * @param {string} commitment - Commitment hash from chain
   * @param {string} salary - Claimed salary amount
   * @param {string} employerId - Employer account ID
   * @param {string} employeeId - Employee account ID
   * @returns {boolean} True if commitment matches
   */
  verifySalaryCommitment(commitment, salary, employerId, employeeId) {
    const computed = this.computeSalaryCommitment(salary, employerId, employeeId);
    return commitment === computed;
  }

  /**
   * Create ZK-style vote commitment for future DAO privacy
   * (Roadmap 2028 - commit-reveal voting)
   * @param {number} proposalId - Proposal ID
   * @param {boolean} vote - True for Aye, False for Nay
   * @param {string} salt - Random salt (32 bytes hex)
   * @returns {string} Vote commitment
   */
  computeVoteCommitment(proposalId, vote, salt) {
    const voteValue = vote ? '1' : '0';
    const data = `${proposalId}:${voteValue}:${salt}`;
    return blake2AsHex(data, 256);
  }

  /**
   * Generate random salt for commitments
   * @returns {string} Random 32-byte hex string
   */
  static generateSalt() {
    const array = new Uint8Array(32);
    crypto.getRandomValues(array);
    return (
      '0x' +
      Array.from(array)
        .map((b) => b.toString(16).padStart(2, '0'))
        .join('')
    );
  }

  /**
   * Compute computation commitment (as used in pallet-belize-staking)
   * @param {Uint8Array} encryptedDelta - Encrypted model delta
   * @param {string} validatorId - Validator account ID
   * @param {number} timestamp - Submission timestamp
   * @returns {string} Computation commitment hash
   */
  computeComputationCommitment(encryptedDelta, validatorId, timestamp) {
    const deltaHex = Array.from(encryptedDelta)
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('');

    const data = `${deltaHex}:${validatorId}:${timestamp}`;
    return blake2AsHex(data, 256);
  }

  /**
   * Validate commitment hash properties (non-zero, non-uniform)
   * @param {string} commitment - Commitment hash to validate
   * @returns {Object} Validation result { valid, reason }
   */
  validateCommitment(commitment) {
    // Remove 0x prefix if present
    const hex = commitment.startsWith('0x') ? commitment.slice(2) : commitment;

    // Check length (should be 64 hex chars = 32 bytes)
    if (hex.length !== 64) {
      return { valid: false, reason: `Invalid length: ${hex.length} (expected 64)` };
    }

    // Check if all zeros
    if (hex === '0'.repeat(64)) {
      return { valid: false, reason: 'Commitment is all zeros' };
    }

    // Check if all same character (uniform)
    const firstChar = hex[0];
    if (hex.split('').every((c) => c === firstChar)) {
      return { valid: false, reason: 'Commitment is uniform (all same character)' };
    }

    // Check entropy (at least 10 different hex characters)
    const uniqueChars = new Set(hex.split('')).size;
    if (uniqueChars < 10) {
      return { valid: false, reason: `Low entropy: only ${uniqueChars} unique characters` };
    }

    return { valid: true, reason: 'Valid commitment' };
  }

  /**
   * Get deduction commitment
   * @param {string} deductionType - Type of deduction
   * @param {string} amount - Deduction amount
   * @param {string} employeeId - Employee account ID
   * @returns {string} Deduction commitment
   */
  computeDeductionCommitment(deductionType, amount, employeeId) {
    const data = `${deductionType}:${amount}:${employeeId}`;
    return blake2AsHex(data, 256);
  }

  /**
   * Get bonus/amount commitment
   * @param {string} amount - Bonus amount
   * @param {string} employeeId - Employee account ID
   * @param {string} bonusType - Type of bonus
   * @returns {string} Bonus commitment
   */
  computeAmountCommitment(amount, employeeId, bonusType) {
    const data = `${amount}:${employeeId}:${bonusType}`;
    return blake2AsHex(data, 256);
  }
}

module.exports = PrivacySDK;

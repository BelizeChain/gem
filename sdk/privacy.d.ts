/**
 * TypeScript type definitions for Privacy SDK
 * @module @belizechain/gem-sdk/privacy
 */

import { ApiPromise } from '@polkadot/api';

/**
 * Deduction information
 */
export interface Deduction {
    type: string;
    amount: string;
}

/**
 * Payroll record with privacy commitments
 */
export interface PayrollRecord {
    employeeId: string;
    employer: string;
    department: string;
    workerType: string;
    salaryCommitment: string; // Hash instead of plaintext amount
    joinedAt: number;
    isActive: boolean;
    paymentCount: number;
}

/**
 * Payment record with privacy commitments
 */
export interface PaymentRecord {
    paymentId: number;
    paymentCommitment: string; // Hash instead of plaintext amounts
    timestamp: number;
    status: string;
}

/**
 * Commitment validation result
 */
export interface CommitmentValidation {
    valid: boolean;
    reason: string;
}

/**
 * Privacy SDK extension
 */
export declare class PrivacySDK {
    /**
     * Polkadot API instance
     */
    api: ApiPromise;

    /**
     * Create a new PrivacySDK instance
     * @param api - Connected Polkadot API instance
     */
    constructor(api: ApiPromise);

    /**
     * Compute salary commitment hash (as used in pallet-belize-payroll)
     * @param salary - Salary amount (in base units)
     * @param employerId - Employer account ID
     * @param employeeId - Employee account ID
     */
    computeSalaryCommitment(
        salary: string,
        employerId: string,
        employeeId: string
    ): string;

    /**
     * Compute payment commitment hash
     * @param grossAmount - Gross payment amount
     * @param deductions - Array of deductions
     * @param netAmount - Net payment amount
     * @param employeeId - Employee account ID
     * @param timestamp - Payment timestamp
     */
    computePaymentCommitment(
        grossAmount: string,
        deductions: Deduction[],
        netAmount: string,
        employeeId: string,
        timestamp: number
    ): string;

    /**
     * Compute batch payment commitment
     * @param paymentCommitments - Individual payment commitments
     * @param totalAmount - Total batch amount
     * @param timestamp - Batch timestamp
     */
    computeBatchCommitment(
        paymentCommitments: string[],
        totalAmount: string,
        timestamp: number
    ): string;

    /**
     * Get payroll record by commitment (privacy-preserving)
     * @param employeeId - Employee account ID
     */
    getPayrollRecord(employeeId: string): Promise<PayrollRecord | null>;

    /**
     * Get payment history with commitments (privacy-preserving)
     * @param employeeId - Employee account ID
     */
    getPaymentHistory(employeeId: string): Promise<PaymentRecord[]>;

    /**
     * Verify salary commitment locally
     * @param commitment - Commitment hash from chain
     * @param salary - Claimed salary amount
     * @param employerId - Employer account ID
     * @param employeeId - Employee account ID
     */
    verifySalaryCommitment(
        commitment: string,
        salary: string,
        employerId: string,
        employeeId: string
    ): boolean;

    /**
     * Create ZK-style vote commitment for future DAO privacy
     * (Roadmap 2028 - commit-reveal voting)
     * @param proposalId - Proposal ID
     * @param vote - True for Aye, False for Nay
     * @param salt - Random salt (32 bytes hex)
     */
    computeVoteCommitment(
        proposalId: number,
        vote: boolean,
        salt: string
    ): string;

    /**
     * Generate random salt for commitments
     */
    static generateSalt(): string;

    /**
     * Compute computation commitment (as used in pallet-belize-staking)
     * @param encryptedDelta - Encrypted model delta
     * @param validatorId - Validator account ID
     * @param timestamp - Submission timestamp
     */
    computeComputationCommitment(
        encryptedDelta: Uint8Array,
        validatorId: string,
        timestamp: number
    ): string;

    /**
     * Validate commitment hash properties (non-zero, non-uniform)
     * @param commitment - Commitment hash to validate
     */
    validateCommitment(commitment: string): CommitmentValidation;

    /**
     * Get deduction commitment
     * @param deductionType - Type of deduction
     * @param amount - Deduction amount
     * @param employeeId - Employee account ID
     */
    computeDeductionCommitment(
        deductionType: string,
        amount: string,
        employeeId: string
    ): string;

    /**
     * Get bonus/amount commitment
     * @param amount - Bonus amount
     * @param employeeId - Employee account ID
     * @param bonusType - Type of bonus
     */
    computeAmountCommitment(
        amount: string,
        employeeId: string,
        bonusType: string
    ): string;
}

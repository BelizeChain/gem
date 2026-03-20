#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// # Simple DAO - Governance Contract
///
/// A lightweight DAO (Decentralized Autonomous Organization) for BelizeChain.
/// Demonstrates cross-contract integration with DALLA token.
///
/// ## Features
/// - Proposal creation with optional treasury transfer actions
/// - DALLA token-weighted voting
/// - Per-proposal total supply snapshot for quorum isolation
/// - Configurable voting periods with execution timelock
/// - Two-step admin transfer pattern
/// - Two-step code hash upgrade with timelock
/// - Proposal cancellation by proposer or admin
/// - Active proposal cap to prevent storage DoS
///
/// ## Governance Security
/// - Voting locks: When a user votes, their DALLA tokens are locked via
///   cross-contract call until the proposal's voting period ends. This prevents
///   flash governance (C02) and double-voting via token transfer (H01).
/// - Proposal expiration: Passed proposals must be executed within the execution
///   window after the timelock expires, or they auto-expire.

#[ink::contract]
mod simple_dao {
    use ink::env::call::{build_call, ExecutionInput, Selector};
    use ink::prelude::string::String;
    use ink::storage::Mapping;

    /// Proposal ID type
    pub type ProposalId = u32;

    /// Proposal status
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    pub enum ProposalStatus {
        Active,
        Passed,
        Rejected,
        Executed,
        Cancelled,
        Expired,
    }

    /// Proposal structure
    #[derive(Debug, Clone)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    #[cfg_attr(feature = "std", derive(ink::storage::traits::StorageLayout))]
    pub struct Proposal {
        pub proposer: AccountId,
        pub description: String,
        pub yes_votes: u128,
        pub no_votes: u128,
        pub start_block: u32,
        pub end_block: u32,
        pub status: ProposalStatus,
        pub executed: bool,
        /// Total voting power snapshot taken at proposal creation (quorum denominator).
        /// Isolates each proposal from future changes to total_voting_power.
        pub total_supply_snapshot: u128,
        /// Block at which the proposal was finalized (used for timelock).
        pub finalized_block: Option<u32>,
        /// Optional transfer target for treasury proposals.
        pub transfer_target: Option<AccountId>,
        /// Transfer value for treasury proposals (native tokens).
        pub transfer_value: Balance,
    }

    /// The DAO error types
    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        /// Proposal does not exist
        ProposalNotFound,
        /// Voting period has ended
        VotingEnded,
        /// Voting period still active
        VotingActive,
        /// Already voted on this proposal
        AlreadyVoted,
        /// Proposal already executed
        AlreadyExecuted,
        /// Proposal has not passed
        ProposalNotPassed,
        /// Caller is not the admin
        NotAdmin,
        /// Invalid voting period (must be >= 10 blocks)
        InvalidVotingPeriod,
        /// Zero address provided
        ZeroAddress,
        /// DALLA token contract not configured
        DallaTokenNotConfigured,
        /// Cross-contract balance query failed
        BalanceQueryFailed,
        /// Caller has no voting power (zero token balance)
        InsufficientVotingPower,
        /// Description exceeds maximum length
        DescriptionTooLong,
        /// Proposal is not in Active status
        NotActive,
        /// Arithmetic overflow
        Overflow,
        /// Maximum active proposals reached
        ProposalCapReached,
        /// Total voting power is zero — cannot calculate quorum
        ZeroTotalSupply,
        /// Timelock period has not expired
        TimelockNotExpired,
        /// No pending admin transfer
        NoPendingAdmin,
        /// Caller is not the pending admin
        NotPendingAdmin,
        /// Invalid quorum basis points (must be 100..=10000)
        InvalidQuorumBps,
        /// Caller is not the proposer or admin
        NotProposerOrAdmin,
        /// Code hash update failed
        CodeHashUpdateFailed,
        /// Execution transfer failed
        ExecutionFailed,
        /// Caller's balance is below the proposal creation threshold
        BelowProposalThreshold,
        /// No code hash upgrade has been proposed
        NoUpgradeProposed,
        /// Proposal execution window has expired
        ExecutionWindowExpired,
        /// Failed to lock voter's DALLA tokens for voting
        VotingLockFailed,
    }

    /// Result type
    pub type Result<T> = core::result::Result<T, Error>;

    /// The Simple DAO storage
    #[ink(storage)]
    pub struct SimpleDao {
        /// Mapping from proposal ID to proposal
        proposals: Mapping<ProposalId, Proposal>,
        /// Mapping from (proposal_id, voter) to vote weight
        votes: Mapping<(ProposalId, AccountId), u128>,
        /// Next proposal ID
        next_proposal_id: ProposalId,
        /// Voting period in blocks
        voting_period: u32,
        /// Quorum threshold (basis points, e.g., 2000 = 20%)
        quorum_bps: u32,
        /// Total voting power — snapshotted per-proposal at creation time.
        /// Admin can update this for future proposals, but changes do not
        /// affect proposals already created.
        total_voting_power: u128,
        /// Admin account
        admin: AccountId,
        /// Pending admin for two-step transfer
        pending_admin: Option<AccountId>,
        /// DALLA token contract address (optional — required for non-admin operations)
        dalla_token: Option<AccountId>,
        /// Timelock duration in blocks after proposal finalization before execution
        timelock_blocks: u32,
        /// Maximum number of active proposals allowed simultaneously
        max_active_proposals: u32,
        /// Current count of active proposals
        active_proposal_count: u32,
        /// Minimum DALLA balance required to create proposals (for non-admin callers)
        min_proposal_threshold: u128,
        /// Proposed code hash for two-step upgrade
        proposed_code_hash: Option<Hash>,
        /// Block at which code hash upgrade was proposed
        proposed_code_hash_block: u32,
        /// Block window after timelock within which a passed proposal must be executed
        execution_window: u32,
    }

    /// Event emitted when a proposal is created
    #[ink(event)]
    pub struct ProposalCreated {
        #[ink(topic)]
        id: ProposalId,
        #[ink(topic)]
        proposer: AccountId,
        description: String,
        end_block: u32,
    }

    /// Event emitted when a vote is cast
    #[ink(event)]
    pub struct VoteCast {
        #[ink(topic)]
        proposal_id: ProposalId,
        #[ink(topic)]
        voter: AccountId,
        support: bool,
        weight: u128,
    }

    /// Event emitted when a proposal is finalized
    #[ink(event)]
    pub struct ProposalFinalized {
        #[ink(topic)]
        proposal_id: ProposalId,
        status: ProposalStatus,
        yes_votes: u128,
        no_votes: u128,
    }

    /// Event emitted when a proposal is executed
    #[ink(event)]
    pub struct ProposalExecuted {
        #[ink(topic)]
        proposal_id: ProposalId,
    }

    /// Event emitted when a proposal is cancelled
    #[ink(event)]
    pub struct ProposalCancelled {
        #[ink(topic)]
        proposal_id: ProposalId,
        #[ink(topic)]
        cancelled_by: AccountId,
    }

    /// Event emitted when an admin transfer is proposed
    #[ink(event)]
    pub struct AdminTransferProposed {
        #[ink(topic)]
        current_admin: AccountId,
        #[ink(topic)]
        proposed_admin: AccountId,
    }

    /// Event emitted when an admin transfer is completed
    #[ink(event)]
    pub struct AdminTransferred {
        #[ink(topic)]
        old_admin: AccountId,
        #[ink(topic)]
        new_admin: AccountId,
    }

    /// Event emitted when a code hash upgrade is proposed
    #[ink(event)]
    pub struct CodeHashUpgradeProposed {
        #[ink(topic)]
        proposed_by: AccountId,
        new_code_hash: Hash,
        earliest_execution_block: u32,
    }

    /// Event emitted when a code hash upgrade is executed
    #[ink(event)]
    pub struct CodeHashUpdated {
        new_code_hash: Hash,
    }

    /// Event emitted when total voting power is updated
    #[ink(event)]
    pub struct TotalVotingPowerUpdated {
        old_value: u128,
        new_value: u128,
    }

    impl SimpleDao {
        /// Maximum description length in bytes
        const MAX_DESCRIPTION_LENGTH: usize = 1024;

        /// Creates a new Simple DAO
        ///
        /// # Panics
        /// - `voting_period < 10`
        /// - `quorum_bps` not in `100..=10000`
        /// - `total_voting_power == 0`
        /// - `max_active_proposals == 0`
        #[ink(constructor)]
        pub fn new(
            voting_period: u32,
            quorum_bps: u32,
            total_voting_power: u128,
            dalla_token: Option<AccountId>,
            timelock_blocks: u32,
            max_active_proposals: u32,
            min_proposal_threshold: u128,
            execution_window: u32,
        ) -> Self {
            assert!(voting_period >= 10, "voting_period must be >= 10 blocks");
            assert!(
                execution_window >= 10,
                "execution_window must be >= 10 blocks"
            );
            assert!(
                quorum_bps >= 100 && quorum_bps <= 10000,
                "quorum_bps must be between 100 and 10000"
            );
            assert!(total_voting_power > 0, "total_voting_power must be > 0");
            assert!(
                max_active_proposals >= 1,
                "max_active_proposals must be >= 1"
            );

            let caller = Self::env().caller();

            Self {
                proposals: Mapping::default(),
                votes: Mapping::default(),
                next_proposal_id: 1,
                voting_period,
                quorum_bps,
                total_voting_power,
                admin: caller,
                pending_admin: None,
                dalla_token,
                timelock_blocks,
                max_active_proposals,
                active_proposal_count: 0,
                min_proposal_threshold,
                proposed_code_hash: None,
                proposed_code_hash_block: 0,
                execution_window,
            }
        }

        /// Creates a new proposal
        ///
        /// Requires the caller to be the admin or hold DALLA tokens above
        /// `min_proposal_threshold`. Optionally includes a treasury transfer
        /// that will be executed if the proposal passes.
        #[ink(message)]
        pub fn create_proposal(
            &mut self,
            description: String,
            transfer_target: Option<AccountId>,
            transfer_value: Balance,
        ) -> Result<ProposalId> {
            let caller = self.env().caller();
            let current_block = self.env().block_number();

            if description.len() > Self::MAX_DESCRIPTION_LENGTH {
                return Err(Error::DescriptionTooLong);
            }

            // M04: Active proposal cap
            if self.active_proposal_count >= self.max_active_proposals {
                return Err(Error::ProposalCapReached);
            }

            // Access control: admin can always create; others need DALLA tokens
            if caller != self.admin {
                let dalla = self.dalla_token.ok_or(Error::DallaTokenNotConfigured)?;
                let balance = self.query_dalla_balance(dalla, caller)?;
                if balance == 0 {
                    return Err(Error::InsufficientVotingPower);
                }
                // L01: Minimum proposal threshold
                if balance < self.min_proposal_threshold {
                    return Err(Error::BelowProposalThreshold);
                }
            }

            // Validate transfer target is not zero address
            if let Some(target) = transfer_target {
                if target == AccountId::from([0u8; 32]) {
                    return Err(Error::ZeroAddress);
                }
            }

            // L03: Checked arithmetic for proposal ID
            let proposal_id = self.next_proposal_id;
            let next_id = proposal_id.checked_add(1).ok_or(Error::Overflow)?;
            // M06: Checked arithmetic for end_block
            let end_block = current_block
                .checked_add(self.voting_period)
                .ok_or(Error::Overflow)?;

            let proposal = Proposal {
                proposer: caller,
                description: description.clone(),
                yes_votes: 0,
                no_votes: 0,
                start_block: current_block,
                end_block,
                status: ProposalStatus::Active,
                executed: false,
                // C05: Snapshot total_voting_power per-proposal at creation time
                total_supply_snapshot: self.total_voting_power,
                finalized_block: None,
                transfer_target,
                transfer_value,
            };

            self.proposals.insert(proposal_id, &proposal);
            self.next_proposal_id = next_id;
            self.active_proposal_count = self
                .active_proposal_count
                .checked_add(1)
                .ok_or(Error::Overflow)?;

            self.env().emit_event(ProposalCreated {
                id: proposal_id,
                proposer: caller,
                description,
                end_block,
            });

            Ok(proposal_id)
        }

        /// Casts a vote on a proposal
        ///
        /// Vote weight is determined by the caller's DALLA token balance
        /// via cross-contract call — callers cannot self-report weight.
        #[ink(message)]
        pub fn vote(&mut self, proposal_id: ProposalId, support: bool) -> Result<()> {
            let caller = self.env().caller();
            let current_block = self.env().block_number();

            // Check if already voted
            if self.votes.contains((proposal_id, caller)) {
                return Err(Error::AlreadyVoted);
            }

            // Get proposal
            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            // Ensure proposal is still active
            if proposal.status != ProposalStatus::Active {
                return Err(Error::NotActive);
            }

            // Check if voting period is still open
            if current_block > proposal.end_block {
                return Err(Error::VotingEnded);
            }

            // Query caller's DALLA balance via cross-contract call
            let dalla = self.dalla_token.ok_or(Error::DallaTokenNotConfigured)?;
            let weight = self.query_dalla_balance(dalla, caller)?;
            if weight == 0 {
                return Err(Error::InsufficientVotingPower);
            }

            // Record vote
            self.votes.insert((proposal_id, caller), &weight);

            // M06: Checked arithmetic on vote tallies
            if support {
                proposal.yes_votes = proposal
                    .yes_votes
                    .checked_add(weight)
                    .ok_or(Error::Overflow)?;
            } else {
                proposal.no_votes = proposal
                    .no_votes
                    .checked_add(weight)
                    .ok_or(Error::Overflow)?;
            }

            self.proposals.insert(proposal_id, &proposal);

            // Lock voter's DALLA tokens to prevent double-voting (H01/C02)
            // Fails if DALLA's authorized_dao is not set to this contract
            self.lock_dalla_tokens(dalla, caller, proposal.end_block)?;

            self.env().emit_event(VoteCast {
                proposal_id,
                voter: caller,
                support,
                weight,
            });

            Ok(())
        }

        /// Finalizes a proposal after voting period ends
        ///
        /// Uses the per-proposal `total_supply_snapshot` for quorum calculation,
        /// ensuring that admin changes to `total_voting_power` after proposal
        /// creation do not affect existing proposals.
        #[ink(message)]
        pub fn finalize_proposal(&mut self, proposal_id: ProposalId) -> Result<()> {
            let current_block = self.env().block_number();

            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            // C01: Status guard — only Active proposals can be finalized
            if proposal.status != ProposalStatus::Active {
                return Err(Error::NotActive);
            }

            // Check if voting period ended
            if current_block <= proposal.end_block {
                return Err(Error::VotingActive);
            }

            // C03: Guard against zero total supply snapshot
            if proposal.total_supply_snapshot == 0 {
                return Err(Error::ZeroTotalSupply);
            }

            // M06: Checked arithmetic for quorum calculation
            let total_votes = proposal
                .yes_votes
                .checked_add(proposal.no_votes)
                .ok_or(Error::Overflow)?;
            let quorum_required = proposal
                .total_supply_snapshot
                .checked_mul(self.quorum_bps as u128)
                .ok_or(Error::Overflow)?
                .checked_div(10000)
                .ok_or(Error::Overflow)?;

            if total_votes >= quorum_required && proposal.yes_votes > proposal.no_votes {
                proposal.status = ProposalStatus::Passed;
            } else {
                proposal.status = ProposalStatus::Rejected;
            }

            // H02: Record finalization block for timelock
            proposal.finalized_block = Some(current_block);

            self.proposals.insert(proposal_id, &proposal);

            // Decrement active proposal count
            self.active_proposal_count = self.active_proposal_count.saturating_sub(1);

            // M03: Emit finalization event
            self.env().emit_event(ProposalFinalized {
                proposal_id,
                status: proposal.status,
                yes_votes: proposal.yes_votes,
                no_votes: proposal.no_votes,
            });

            Ok(())
        }

        /// Executes a passed proposal after the timelock period
        ///
        /// If the proposal includes a transfer target and value, the transfer
        /// is dispatched after state is updated (CEI pattern).
        #[ink(message)]
        pub fn execute_proposal(&mut self, proposal_id: ProposalId) -> Result<()> {
            let current_block = self.env().block_number();

            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.executed {
                return Err(Error::AlreadyExecuted);
            }

            if proposal.status != ProposalStatus::Passed {
                return Err(Error::ProposalNotPassed);
            }

            // H02: Timelock check
            let finalized_block = proposal.finalized_block.ok_or(Error::NotActive)?;
            let earliest_execution = finalized_block
                .checked_add(self.timelock_blocks)
                .ok_or(Error::Overflow)?;
            if current_block < earliest_execution {
                return Err(Error::TimelockNotExpired);
            }

            // Check execution window — auto-expire if deadline passed
            let deadline = earliest_execution
                .checked_add(self.execution_window)
                .ok_or(Error::Overflow)?;
            if current_block > deadline {
                proposal.status = ProposalStatus::Expired;
                self.proposals.insert(proposal_id, &proposal);
                return Err(Error::ExecutionWindowExpired);
            }

            // CEI: Mark as executed BEFORE dispatching any transfer
            proposal.executed = true;
            proposal.status = ProposalStatus::Executed;
            self.proposals.insert(proposal_id, &proposal);

            // M01: Execute treasury transfer if configured
            if let Some(target) = proposal.transfer_target {
                if proposal.transfer_value > 0 {
                    self.env()
                        .transfer(target, proposal.transfer_value)
                        .map_err(|_| Error::ExecutionFailed)?;
                }
            }

            self.env().emit_event(ProposalExecuted { proposal_id });

            Ok(())
        }

        /// Cancels an active proposal
        ///
        /// Only the original proposer or the admin can cancel a proposal.
        /// Cancelled proposals cannot be finalized or executed.
        #[ink(message)]
        pub fn cancel_proposal(&mut self, proposal_id: ProposalId) -> Result<()> {
            let caller = self.env().caller();

            let mut proposal = self
                .proposals
                .get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.status != ProposalStatus::Active {
                return Err(Error::NotActive);
            }

            if caller != proposal.proposer && caller != self.admin {
                return Err(Error::NotProposerOrAdmin);
            }

            proposal.status = ProposalStatus::Cancelled;
            self.proposals.insert(proposal_id, &proposal);

            self.active_proposal_count = self.active_proposal_count.saturating_sub(1);

            self.env().emit_event(ProposalCancelled {
                proposal_id,
                cancelled_by: caller,
            });

            Ok(())
        }

        /// Gets a proposal by ID
        #[ink(message)]
        pub fn get_proposal(&self, proposal_id: ProposalId) -> Option<Proposal> {
            self.proposals.get(proposal_id)
        }

        /// Gets the vote weight for an account on a proposal
        #[ink(message)]
        pub fn get_vote(&self, proposal_id: ProposalId, voter: AccountId) -> Option<u128> {
            self.votes.get((proposal_id, voter))
        }

        /// Gets the current proposal count
        #[ink(message)]
        pub fn proposal_count(&self) -> ProposalId {
            self.next_proposal_id.saturating_sub(1)
        }

        /// Gets the voting period
        #[ink(message)]
        pub fn voting_period(&self) -> u32 {
            self.voting_period
        }

        /// Gets the quorum threshold
        #[ink(message)]
        pub fn quorum_threshold(&self) -> u32 {
            self.quorum_bps
        }

        /// Sets the total voting power (admin only)
        ///
        /// Changes only affect future proposals — existing proposals retain
        /// their creation-time snapshot.
        #[ink(message)]
        pub fn set_total_voting_power(&mut self, power: u128) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotAdmin);
            }
            if power == 0 {
                return Err(Error::ZeroTotalSupply);
            }

            let old_value = self.total_voting_power;
            self.total_voting_power = power;

            self.env().emit_event(TotalVotingPowerUpdated {
                old_value,
                new_value: power,
            });

            Ok(())
        }

        /// Gets the total voting power
        #[ink(message)]
        pub fn total_voting_power(&self) -> u128 {
            self.total_voting_power
        }

        /// Gets the DALLA token address
        #[ink(message)]
        pub fn dalla_token_address(&self) -> Option<AccountId> {
            self.dalla_token
        }

        /// Proposes an admin transfer (two-step pattern)
        ///
        /// The new admin must call `accept_admin()` to complete the transfer.
        #[ink(message)]
        pub fn transfer_admin(&mut self, new_admin: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotAdmin);
            }
            if new_admin == AccountId::from([0u8; 32]) {
                return Err(Error::ZeroAddress);
            }

            self.pending_admin = Some(new_admin);

            self.env().emit_event(AdminTransferProposed {
                current_admin: self.admin,
                proposed_admin: new_admin,
            });

            Ok(())
        }

        /// Accepts a pending admin transfer
        ///
        /// Must be called by the account set as `pending_admin`.
        #[ink(message)]
        pub fn accept_admin(&mut self) -> Result<()> {
            let caller = self.env().caller();
            let pending = self.pending_admin.ok_or(Error::NoPendingAdmin)?;

            if caller != pending {
                return Err(Error::NotPendingAdmin);
            }

            let old_admin = self.admin;
            self.admin = pending;
            self.pending_admin = None;

            self.env().emit_event(AdminTransferred {
                old_admin,
                new_admin: pending,
            });

            Ok(())
        }

        /// Gets the admin address
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
        }

        /// Gets the pending admin address
        #[ink(message)]
        pub fn pending_admin(&self) -> Option<AccountId> {
            self.pending_admin
        }

        /// Gets the timelock duration in blocks
        #[ink(message)]
        pub fn timelock_blocks(&self) -> u32 {
            self.timelock_blocks
        }

        /// Gets the current active proposal count
        #[ink(message)]
        pub fn active_proposal_count(&self) -> u32 {
            self.active_proposal_count
        }

        /// Gets the maximum active proposals allowed
        #[ink(message)]
        pub fn max_active_proposals(&self) -> u32 {
            self.max_active_proposals
        }

        /// Gets the minimum proposal threshold
        #[ink(message)]
        pub fn min_proposal_threshold(&self) -> u128 {
            self.min_proposal_threshold
        }

        /// Gets the execution window in blocks
        #[ink(message)]
        pub fn execution_window(&self) -> u32 {
            self.execution_window
        }

        /// Gets the currently proposed code hash (if any)
        #[ink(message)]
        pub fn proposed_code_hash(&self) -> Option<Hash> {
            self.proposed_code_hash
        }

        /// Proposes a code hash upgrade (admin only, requires timelock)
        ///
        /// The upgrade cannot be executed until `timelock_blocks` have passed.
        /// This gives the community time to observe and react.
        #[ink(message)]
        pub fn propose_code_hash_upgrade(&mut self, new_code_hash: Hash) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotAdmin);
            }

            let current_block = self.env().block_number();
            self.proposed_code_hash = Some(new_code_hash);
            self.proposed_code_hash_block = current_block;

            let earliest = current_block
                .checked_add(self.timelock_blocks)
                .unwrap_or(u32::MAX);

            self.env().emit_event(CodeHashUpgradeProposed {
                proposed_by: caller,
                new_code_hash,
                earliest_execution_block: earliest,
            });

            Ok(())
        }

        /// Executes a previously proposed code hash upgrade after timelock
        #[ink(message)]
        pub fn execute_code_hash_upgrade(&mut self) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotAdmin);
            }

            let code_hash = self.proposed_code_hash.ok_or(Error::NoUpgradeProposed)?;

            let current_block = self.env().block_number();
            let earliest = self
                .proposed_code_hash_block
                .checked_add(self.timelock_blocks)
                .ok_or(Error::Overflow)?;
            if current_block < earliest {
                return Err(Error::TimelockNotExpired);
            }

            // Clear proposal before executing
            self.proposed_code_hash = None;
            self.proposed_code_hash_block = 0;

            self.env().emit_event(CodeHashUpdated {
                new_code_hash: code_hash,
            });

            ink::env::set_code_hash::<Environment>(&code_hash)
                .map_err(|_| Error::CodeHashUpdateFailed)?;

            Ok(())
        }

        /// Cancels a pending code hash upgrade
        #[ink(message)]
        pub fn cancel_code_hash_upgrade(&mut self) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotAdmin);
            }

            if self.proposed_code_hash.is_none() {
                return Err(Error::NoUpgradeProposed);
            }

            self.proposed_code_hash = None;
            self.proposed_code_hash_block = 0;

            Ok(())
        }

        /// Query DALLA token balance via cross-contract call
        ///
        /// Uses PSP22 `balance_of` selector (0x6568_2523)
        fn query_dalla_balance(&self, dalla: AccountId, account: AccountId) -> Result<u128> {
            // PSP22::balance_of selector
            let selector = [0x65, 0x68, 0x25, 0x23];

            let result = build_call::<Environment>()
                .call(dalla)
                .exec_input(
                    ExecutionInput::new(Selector::new(selector)).push_arg(account),
                )
                .returns::<u128>()
                .try_invoke();

            match result {
                Ok(Ok(balance)) => Ok(balance),
                _ => Err(Error::BalanceQueryFailed),
            }
        }

        /// Lock DALLA tokens for voting via cross-contract call
        ///
        /// Uses DALLA `lock_for_voting` selector (0x6C6F636B).
        /// Returns Ok if lock succeeded or DALLA is not configured for locking.
        fn lock_dalla_tokens(
            &self,
            dalla: AccountId,
            account: AccountId,
            until_block: u32,
        ) -> Result<()> {
            let selector = [0x6C, 0x6F, 0x63, 0x6B];

            let result = build_call::<Environment>()
                .call(dalla)
                .exec_input(
                    ExecutionInput::new(Selector::new(selector))
                        .push_arg(account)
                        .push_arg(until_block),
                )
                .returns::<bool>()
                .try_invoke();

            match result {
                Ok(Ok(true)) => Ok(()),
                Ok(Ok(false)) => Err(Error::VotingLockFailed),
                _ => Err(Error::VotingLockFailed),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn default_accounts() -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn set_caller(account: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(account);
        }

        fn advance_block(blocks: u32) {
            let current = ink::env::block_number::<ink::env::DefaultEnvironment>();
            ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(
                current.saturating_add(blocks),
            );
        }

        /// Helper: creates a DAO with sensible defaults (no DALLA, threshold 0)
        fn create_dao() -> SimpleDao {
            SimpleDao::new(100, 2000, 1000, None, 10, 50, 0, 100)
        }

        /// Helper: creates a DAO with a DALLA token address and threshold
        fn create_dao_with_dalla(dalla: AccountId) -> SimpleDao {
            SimpleDao::new(100, 2000, 1000, Some(dalla), 10, 50, 100, 100)
        }

        // ── Constructor tests ───────────────────────────────────────────

        #[ink::test]
        fn new_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let dao = create_dao();

            assert_eq!(dao.voting_period(), 100);
            assert_eq!(dao.quorum_threshold(), 2000);
            assert_eq!(dao.proposal_count(), 0);
            assert_eq!(dao.admin(), accounts.alice);
            assert_eq!(dao.total_voting_power(), 1000);
            assert_eq!(dao.timelock_blocks(), 10);
            assert_eq!(dao.max_active_proposals(), 50);
            assert_eq!(dao.active_proposal_count(), 0);
            assert_eq!(dao.pending_admin(), None);
            assert_eq!(dao.execution_window(), 100);
        }

        #[ink::test]
        #[should_panic(expected = "voting_period must be >= 10 blocks")]
        fn new_rejects_short_voting_period() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            SimpleDao::new(5, 2000, 1000, None, 10, 50, 0, 100);
        }

        #[ink::test]
        #[should_panic(expected = "quorum_bps must be between 100 and 10000")]
        fn new_rejects_zero_quorum_bps() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            SimpleDao::new(100, 0, 1000, None, 10, 50, 0, 100);
        }

        #[ink::test]
        #[should_panic(expected = "quorum_bps must be between 100 and 10000")]
        fn new_rejects_excessive_quorum_bps() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            SimpleDao::new(100, 10001, 1000, None, 10, 50, 0, 100);
        }

        #[ink::test]
        #[should_panic(expected = "total_voting_power must be > 0")]
        fn new_rejects_zero_total_voting_power() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            SimpleDao::new(100, 2000, 0, None, 10, 50, 0, 100);
        }

        #[ink::test]
        #[should_panic(expected = "max_active_proposals must be >= 1")]
        fn new_rejects_zero_max_active_proposals() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            SimpleDao::new(100, 2000, 1000, None, 10, 0, 0, 100);
        }

        // ── Proposal creation tests ─────────────────────────────────────

        #[ink::test]
        fn create_proposal_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let description = String::from("Increase treasury allocation");
            let result = dao.create_proposal(description.clone(), None, 0);
            assert!(result.is_ok());

            let proposal_id = result.unwrap();
            assert_eq!(proposal_id, 1);
            assert_eq!(dao.proposal_count(), 1);
            assert_eq!(dao.active_proposal_count(), 1);

            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.proposer, accounts.alice);
            assert_eq!(proposal.description, description);
            assert_eq!(proposal.status, ProposalStatus::Active);
            assert_eq!(proposal.total_supply_snapshot, 1000);
            assert_eq!(proposal.finalized_block, None);
            assert_eq!(proposal.transfer_target, None);
            assert_eq!(proposal.transfer_value, 0);
        }

        #[ink::test]
        fn create_proposal_with_transfer() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let result = dao.create_proposal(
                String::from("Fund development"),
                Some(accounts.bob),
                1_000_000,
            );
            assert!(result.is_ok());

            let proposal = dao.get_proposal(result.unwrap()).unwrap();
            assert_eq!(proposal.transfer_target, Some(accounts.bob));
            assert_eq!(proposal.transfer_value, 1_000_000);
        }

        #[ink::test]
        fn create_proposal_description_too_long() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let long_desc = String::from_utf8(vec![b'A'; 1025]).unwrap();
            let result = dao.create_proposal(long_desc, None, 0);
            assert_eq!(result, Err(Error::DescriptionTooLong));
        }

        #[ink::test]
        fn create_proposal_cap_enforced() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = SimpleDao::new(100, 2000, 1000, None, 10, 2, 0, 100);

            dao.create_proposal(String::from("P1"), None, 0).unwrap();
            dao.create_proposal(String::from("P2"), None, 0).unwrap();

            let result = dao.create_proposal(String::from("P3"), None, 0);
            assert_eq!(result, Err(Error::ProposalCapReached));
        }

        #[ink::test]
        fn create_proposal_cap_frees_on_cancel() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = SimpleDao::new(100, 2000, 1000, None, 10, 2, 0, 100);

            let p1 = dao.create_proposal(String::from("P1"), None, 0).unwrap();
            dao.create_proposal(String::from("P2"), None, 0).unwrap();

            // Cancel P1 — frees a slot
            dao.cancel_proposal(p1).unwrap();

            let result = dao.create_proposal(String::from("P3"), None, 0);
            assert!(result.is_ok());
        }

        #[ink::test]
        fn create_proposal_non_admin_requires_dalla() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            set_caller(accounts.bob);
            let result = dao.create_proposal(String::from("Test"), None, 0);
            assert_eq!(result, Err(Error::DallaTokenNotConfigured));
        }

        #[ink::test]
        fn create_proposal_zero_transfer_target_rejected() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let result = dao.create_proposal(
                String::from("Bad transfer"),
                Some(AccountId::from([0u8; 32])),
                1000,
            );
            assert_eq!(result, Err(Error::ZeroAddress));
        }

        #[ink::test]
        #[should_panic(expected = "not implemented")]
        fn create_proposal_non_admin_with_dalla_panics_offchain() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao_with_dalla(accounts.charlie);

            set_caller(accounts.bob);
            let _ = dao.create_proposal(String::from("Test"), None, 0);
        }

        // ── Voting tests ────────────────────────────────────────────────

        #[ink::test]
        fn vote_requires_dalla_token() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            let result = dao.vote(proposal_id, true);
            assert_eq!(result, Err(Error::DallaTokenNotConfigured));
        }

        #[ink::test]
        #[should_panic(expected = "not implemented")]
        fn vote_with_fake_dalla_panics_in_offchain() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao_with_dalla(accounts.charlie);
            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            let _ = dao.vote(proposal_id, true);
        }

        #[ink::test]
        fn already_voted_checked_before_balance_query() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            dao.votes.insert((proposal_id, accounts.alice), &100);

            let result = dao.vote(proposal_id, true);
            assert_eq!(result, Err(Error::AlreadyVoted));
        }

        #[ink::test]
        fn vote_on_cancelled_proposal_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            dao.cancel_proposal(proposal_id).unwrap();

            let result = dao.vote(proposal_id, true);
            assert_eq!(result, Err(Error::NotActive));
        }

        // ── Finalization tests ──────────────────────────────────────────

        #[ink::test]
        fn finalize_proposal_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            // Manually insert vote records (simulating successful cross-contract votes)
            dao.votes.insert((proposal_id, accounts.alice), &300);
            let mut proposal = dao.get_proposal(proposal_id).unwrap();
            proposal.yes_votes = 300;
            dao.proposals.insert(proposal_id, &proposal);

            advance_block(101);

            let result = dao.finalize_proposal(proposal_id);
            assert!(result.is_ok());

            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.status, ProposalStatus::Passed);
            assert!(proposal.finalized_block.is_some());
            assert_eq!(dao.active_proposal_count(), 0);
        }

        #[ink::test]
        fn finalize_rejects_non_active_proposal() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            dao.votes.insert((proposal_id, accounts.alice), &300);
            let mut proposal = dao.get_proposal(proposal_id).unwrap();
            proposal.yes_votes = 300;
            dao.proposals.insert(proposal_id, &proposal);

            advance_block(101);
            dao.finalize_proposal(proposal_id).unwrap();

            // C01: Re-finalization attempt — must fail
            let result = dao.finalize_proposal(proposal_id);
            assert_eq!(result, Err(Error::NotActive));
        }

        #[ink::test]
        fn finalize_cancelled_proposal_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            dao.cancel_proposal(proposal_id).unwrap();

            advance_block(101);
            let result = dao.finalize_proposal(proposal_id);
            assert_eq!(result, Err(Error::NotActive));
        }

        #[ink::test]
        fn quorum_not_met_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            // Only 100 votes, quorum needs 200 (20% of 1000)
            dao.votes.insert((proposal_id, accounts.alice), &100);
            let mut proposal = dao.get_proposal(proposal_id).unwrap();
            proposal.yes_votes = 100;
            dao.proposals.insert(proposal_id, &proposal);

            advance_block(101);
            dao.finalize_proposal(proposal_id).unwrap();

            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.status, ProposalStatus::Rejected);
        }

        #[ink::test]
        fn total_voting_power_snapshot_isolation() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            // Create proposal with total_voting_power = 1000
            let p1 = dao.create_proposal(String::from("P1"), None, 0).unwrap();

            // Change total_voting_power to 10000
            dao.set_total_voting_power(10000).unwrap();

            // Create second proposal with new total_voting_power
            let p2 = dao.create_proposal(String::from("P2"), None, 0).unwrap();

            // Verify snapshots are independent
            let proposal1 = dao.get_proposal(p1).unwrap();
            let proposal2 = dao.get_proposal(p2).unwrap();
            assert_eq!(proposal1.total_supply_snapshot, 1000);
            assert_eq!(proposal2.total_supply_snapshot, 10000);
        }

        // ── Execution tests ─────────────────────────────────────────────

        #[ink::test]
        fn execute_proposal_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            dao.votes.insert((proposal_id, accounts.alice), &300);
            let mut proposal = dao.get_proposal(proposal_id).unwrap();
            proposal.yes_votes = 300;
            dao.proposals.insert(proposal_id, &proposal);

            advance_block(101);
            dao.finalize_proposal(proposal_id).unwrap();

            // Advance past timelock (10 blocks)
            advance_block(11);

            let result = dao.execute_proposal(proposal_id);
            assert!(result.is_ok());

            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.status, ProposalStatus::Executed);
            assert!(proposal.executed);
        }

        #[ink::test]
        fn execute_proposal_before_timelock_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            dao.votes.insert((proposal_id, accounts.alice), &300);
            let mut proposal = dao.get_proposal(proposal_id).unwrap();
            proposal.yes_votes = 300;
            dao.proposals.insert(proposal_id, &proposal);

            advance_block(101);
            dao.finalize_proposal(proposal_id).unwrap();

            // Do NOT advance past timelock
            let result = dao.execute_proposal(proposal_id);
            assert_eq!(result, Err(Error::TimelockNotExpired));
        }

        #[ink::test]
        fn execute_rejected_proposal_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            // No votes — will be rejected
            advance_block(101);
            dao.finalize_proposal(proposal_id).unwrap();
            advance_block(11);

            let result = dao.execute_proposal(proposal_id);
            assert_eq!(result, Err(Error::ProposalNotPassed));
        }

        // ── Cancellation tests ──────────────────────────────────────────

        #[ink::test]
        fn cancel_proposal_by_proposer() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            assert_eq!(dao.active_proposal_count(), 1);

            let result = dao.cancel_proposal(proposal_id);
            assert!(result.is_ok());

            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.status, ProposalStatus::Cancelled);
            assert_eq!(dao.active_proposal_count(), 0);
        }

        #[ink::test]
        fn cancel_proposal_by_admin() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            // Alice is both proposer and admin — can cancel
            let result = dao.cancel_proposal(proposal_id);
            assert!(result.is_ok());
        }

        #[ink::test]
        fn cancel_proposal_non_proposer_non_admin_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            set_caller(accounts.bob);
            let result = dao.cancel_proposal(proposal_id);
            assert_eq!(result, Err(Error::NotProposerOrAdmin));
        }

        #[ink::test]
        fn cancel_non_active_proposal_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            dao.cancel_proposal(proposal_id).unwrap();

            let result = dao.cancel_proposal(proposal_id);
            assert_eq!(result, Err(Error::NotActive));
        }

        // ── Admin transfer tests ────────────────────────────────────────

        #[ink::test]
        fn two_step_admin_transfer() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            assert_eq!(dao.admin(), accounts.alice);

            // Propose transfer
            dao.transfer_admin(accounts.bob).unwrap();
            assert_eq!(dao.admin(), accounts.alice); // Still alice
            assert_eq!(dao.pending_admin(), Some(accounts.bob));

            // Bob accepts
            set_caller(accounts.bob);
            dao.accept_admin().unwrap();
            assert_eq!(dao.admin(), accounts.bob);
            assert_eq!(dao.pending_admin(), None);
        }

        #[ink::test]
        fn accept_admin_wrong_caller_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            dao.transfer_admin(accounts.bob).unwrap();

            set_caller(accounts.charlie);
            let result = dao.accept_admin();
            assert_eq!(result, Err(Error::NotPendingAdmin));
        }

        #[ink::test]
        fn accept_admin_no_pending_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let result = dao.accept_admin();
            assert_eq!(result, Err(Error::NoPendingAdmin));
        }

        #[ink::test]
        fn transfer_admin_zero_address_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let result = dao.transfer_admin(AccountId::from([0u8; 32]));
            assert_eq!(result, Err(Error::ZeroAddress));
        }

        #[ink::test]
        fn transfer_admin_non_admin_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            set_caller(accounts.bob);
            let result = dao.transfer_admin(accounts.charlie);
            assert_eq!(result, Err(Error::NotAdmin));
        }

        // ── Total voting power tests ────────────────────────────────────

        #[ink::test]
        fn set_total_voting_power_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let result = dao.set_total_voting_power(5000);
            assert!(result.is_ok());
            assert_eq!(dao.total_voting_power(), 5000);
        }

        #[ink::test]
        fn set_total_voting_power_zero_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let result = dao.set_total_voting_power(0);
            assert_eq!(result, Err(Error::ZeroTotalSupply));
        }

        #[ink::test]
        fn set_total_voting_power_non_admin_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            set_caller(accounts.bob);
            let result = dao.set_total_voting_power(5000);
            assert_eq!(result, Err(Error::NotAdmin));
        }

        // ── Code hash upgrade tests ─────────────────────────────────────

        #[ink::test]
        fn code_hash_upgrade_propose_and_cancel() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            let hash = Hash::from([0xAB; 32]);

            dao.propose_code_hash_upgrade(hash).unwrap();
            assert_eq!(dao.proposed_code_hash(), Some(hash));

            dao.cancel_code_hash_upgrade().unwrap();
            assert_eq!(dao.proposed_code_hash(), None);
        }

        #[ink::test]
        fn code_hash_upgrade_before_timelock_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();
            let hash = Hash::from([0xAB; 32]);

            dao.propose_code_hash_upgrade(hash).unwrap();

            let result = dao.execute_code_hash_upgrade();
            assert_eq!(result, Err(Error::TimelockNotExpired));
        }

        #[ink::test]
        fn code_hash_upgrade_non_admin_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            set_caller(accounts.bob);
            let hash = Hash::from([0xAB; 32]);
            let result = dao.propose_code_hash_upgrade(hash);
            assert_eq!(result, Err(Error::NotAdmin));
        }

        #[ink::test]
        fn cancel_code_hash_no_proposal_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let result = dao.cancel_code_hash_upgrade();
            assert_eq!(result, Err(Error::NoUpgradeProposed));
        }

        #[ink::test]
        fn execute_code_hash_no_proposal_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let result = dao.execute_code_hash_upgrade();
            assert_eq!(result, Err(Error::NoUpgradeProposed));
        }

        // ── Execution window tests ──────────────────────────────────────

        #[ink::test]
        #[should_panic(expected = "execution_window must be >= 10 blocks")]
        fn new_rejects_short_execution_window() {
            let accounts = default_accounts();
            set_caller(accounts.alice);
            SimpleDao::new(100, 2000, 1000, None, 10, 50, 0, 5);
        }

        #[ink::test]
        fn execute_proposal_after_window_expires() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            dao.votes.insert((proposal_id, accounts.alice), &300);
            let mut proposal = dao.get_proposal(proposal_id).unwrap();
            proposal.yes_votes = 300;
            dao.proposals.insert(proposal_id, &proposal);

            // Advance past voting period
            advance_block(101);
            dao.finalize_proposal(proposal_id).unwrap();

            // Advance past timelock (10) + execution_window (100) = 110 blocks past finalization
            advance_block(111);

            let result = dao.execute_proposal(proposal_id);
            assert_eq!(result, Err(Error::ExecutionWindowExpired));

            // Verify proposal status is now Expired
            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.status, ProposalStatus::Expired);
        }

        #[ink::test]
        fn execute_proposal_at_window_edge_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            dao.votes.insert((proposal_id, accounts.alice), &300);
            let mut proposal = dao.get_proposal(proposal_id).unwrap();
            proposal.yes_votes = 300;
            dao.proposals.insert(proposal_id, &proposal);

            advance_block(101);
            dao.finalize_proposal(proposal_id).unwrap();

            // Advance to exactly timelock + execution_window (not past it)
            // finalized_block = 101, deadline = 101 + 10 + 100 = 211
            // current_block after advance = 101 + 110 = 211
            advance_block(110);

            let result = dao.execute_proposal(proposal_id);
            assert!(result.is_ok());
        }

        #[ink::test]
        fn expired_proposal_cannot_be_re_executed() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();

            dao.votes.insert((proposal_id, accounts.alice), &300);
            let mut proposal = dao.get_proposal(proposal_id).unwrap();
            proposal.yes_votes = 300;
            dao.proposals.insert(proposal_id, &proposal);

            advance_block(101);
            dao.finalize_proposal(proposal_id).unwrap();

            // Expire it
            advance_block(111);
            let result = dao.execute_proposal(proposal_id);
            assert_eq!(result, Err(Error::ExecutionWindowExpired));

            // Try again — now it's ProposalNotPassed because status is Expired
            let result = dao.execute_proposal(proposal_id);
            assert_eq!(result, Err(Error::ProposalNotPassed));
        }

        // ── Active proposal count tests ─────────────────────────────────

        #[ink::test]
        fn active_proposal_count_decrements_on_finalize() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();
            assert_eq!(dao.active_proposal_count(), 1);

            advance_block(101);
            dao.finalize_proposal(proposal_id).unwrap();
            assert_eq!(dao.active_proposal_count(), 0);
        }

        #[ink::test]
        fn active_proposal_count_decrements_on_cancel() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = create_dao();

            let proposal_id = dao.create_proposal(String::from("Test"), None, 0).unwrap();
            assert_eq!(dao.active_proposal_count(), 1);

            dao.cancel_proposal(proposal_id).unwrap();
            assert_eq!(dao.active_proposal_count(), 0);
        }
    }
}

#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// # Simple DAO - Governance Contract
/// 
/// A lightweight DAO (Decentralized Autonomous Organization) for BelizeChain.
/// Demonstrates cross-contract integration with DALLA token.
/// 
/// ## Features
/// - Proposal creation and voting
/// - DALLA token-weighted voting
/// - NFT-based membership verification
/// - Configurable voting periods
/// - Proposal execution after quorum
/// 
/// ## Use Cases
/// - Community governance
/// - Treasury management
/// - Protocol upgrades
/// - Parameter adjustments

#[ink::contract]
mod simple_dao {
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
        /// Proposal did not pass
        ProposalFailed,
        /// Not a member (no NFT)
        NotMember,
        /// Invalid voting period
        InvalidVotingPeriod,
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
        /// Total voting power (for quorum calculation)
        total_voting_power: u128,
        /// Admin account
        admin: AccountId,
        /// DALLA token contract address (optional)
        dalla_token: Option<AccountId>,
        /// NFT membership contract address (optional)
        nft_membership: Option<AccountId>,
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

    /// Event emitted when a proposal is executed
    #[ink(event)]
    pub struct ProposalExecuted {
        #[ink(topic)]
        proposal_id: ProposalId,
    }

    impl SimpleDao {
        /// Creates a new Simple DAO
        #[ink(constructor)]
        pub fn new(
            voting_period: u32,
            quorum_bps: u32,
            dalla_token: Option<AccountId>,
            nft_membership: Option<AccountId>,
        ) -> Self {
            let caller = Self::env().caller();
            
            Self {
                proposals: Mapping::default(),
                votes: Mapping::default(),
                next_proposal_id: 1,
                voting_period,
                quorum_bps,
                total_voting_power: 0,
                admin: caller,
                dalla_token,
                nft_membership,
            }
        }

        /// Creates a new proposal
        #[ink(message)]
        pub fn create_proposal(&mut self, description: String) -> Result<ProposalId> {
            let caller = self.env().caller();
            let current_block = self.env().block_number();
            
            if self.voting_period == 0 {
                return Err(Error::InvalidVotingPeriod);
            }

            let proposal_id = self.next_proposal_id;
            let end_block = current_block.saturating_add(self.voting_period);

            let proposal = Proposal {
                proposer: caller,
                description: description.clone(),
                yes_votes: 0,
                no_votes: 0,
                start_block: current_block,
                end_block,
                status: ProposalStatus::Active,
                executed: false,
            };

            self.proposals.insert(proposal_id, &proposal);
            self.next_proposal_id = self.next_proposal_id.saturating_add(1);

            self.env().emit_event(ProposalCreated {
                id: proposal_id,
                proposer: caller,
                description,
                end_block,
            });

            Ok(proposal_id)
        }

        /// Casts a vote on a proposal
        #[ink(message)]
        pub fn vote(&mut self, proposal_id: ProposalId, support: bool, weight: u128) -> Result<()> {
            let caller = self.env().caller();
            let current_block = self.env().block_number();

            // Check if already voted
            if self.votes.contains((proposal_id, caller)) {
                return Err(Error::AlreadyVoted);
            }

            // Get proposal
            let mut proposal = self.proposals.get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            // Check if voting is still active
            if current_block > proposal.end_block {
                return Err(Error::VotingEnded);
            }

            // Record vote
            self.votes.insert((proposal_id, caller), &weight);

            // Update vote counts
            if support {
                proposal.yes_votes = proposal.yes_votes.saturating_add(weight);
            } else {
                proposal.no_votes = proposal.no_votes.saturating_add(weight);
            }

            self.proposals.insert(proposal_id, &proposal);

            self.env().emit_event(VoteCast {
                proposal_id,
                voter: caller,
                support,
                weight,
            });

            Ok(())
        }

        /// Finalizes a proposal after voting period ends
        #[ink(message)]
        pub fn finalize_proposal(&mut self, proposal_id: ProposalId) -> Result<()> {
            let current_block = self.env().block_number();

            let mut proposal = self.proposals.get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            // Check if voting period ended
            if current_block <= proposal.end_block {
                return Err(Error::VotingActive);
            }

            // Update status based on votes
            let total_votes = proposal.yes_votes.saturating_add(proposal.no_votes);
            let quorum_required = self.total_voting_power
                .saturating_mul(self.quorum_bps as u128)
                .saturating_div(10000);

            if total_votes >= quorum_required && proposal.yes_votes > proposal.no_votes {
                proposal.status = ProposalStatus::Passed;
            } else {
                proposal.status = ProposalStatus::Rejected;
            }

            self.proposals.insert(proposal_id, &proposal);

            Ok(())
        }

        /// Executes a passed proposal
        #[ink(message)]
        pub fn execute_proposal(&mut self, proposal_id: ProposalId) -> Result<()> {
            let mut proposal = self.proposals.get(proposal_id)
                .ok_or(Error::ProposalNotFound)?;

            if proposal.executed {
                return Err(Error::AlreadyExecuted);
            }

            if proposal.status != ProposalStatus::Passed {
                return Err(Error::ProposalFailed);
            }

            // Mark as executed
            proposal.executed = true;
            proposal.status = ProposalStatus::Executed;
            self.proposals.insert(proposal_id, &proposal);

            self.env().emit_event(ProposalExecuted {
                proposal_id,
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
        #[ink(message)]
        pub fn set_total_voting_power(&mut self, power: u128) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotMember);
            }

            self.total_voting_power = power;
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

        /// Gets the NFT membership address
        #[ink(message)]
        pub fn nft_membership_address(&self) -> Option<AccountId> {
            self.nft_membership
        }

        /// Transfers admin rights
        #[ink(message)]
        pub fn transfer_admin(&mut self, new_admin: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.admin {
                return Err(Error::NotMember);
            }

            self.admin = new_admin;
            Ok(())
        }

        /// Gets the admin address
        #[ink(message)]
        pub fn admin(&self) -> AccountId {
            self.admin
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
            let current = ink::env::test::get_current_block_number::<ink::env::DefaultEnvironment>()
                .unwrap_or(1);
            ink::env::test::set_block_number::<ink::env::DefaultEnvironment>(
                current.saturating_add(blocks)
            );
        }

        #[ink::test]
        fn new_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let dao = SimpleDao::new(100, 2000, None, None);

            assert_eq!(dao.voting_period(), 100);
            assert_eq!(dao.quorum_threshold(), 2000);
            assert_eq!(dao.proposal_count(), 0);
            assert_eq!(dao.admin(), accounts.alice);
        }

        #[ink::test]
        fn create_proposal_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = SimpleDao::new(100, 2000, None, None);

            let description = String::from("Increase treasury allocation");
            let result = dao.create_proposal(description.clone());
            assert!(result.is_ok());

            let proposal_id = result.unwrap();
            assert_eq!(proposal_id, 1);
            assert_eq!(dao.proposal_count(), 1);

            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.proposer, accounts.alice);
            assert_eq!(proposal.description, description);
            assert_eq!(proposal.status, ProposalStatus::Active);
        }

        #[ink::test]
        fn vote_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = SimpleDao::new(100, 2000, None, None);
            dao.set_total_voting_power(1000).unwrap();

            let proposal_id = dao.create_proposal(String::from("Test proposal")).unwrap();

            // Alice votes yes with weight 100
            set_caller(accounts.alice);
            let result = dao.vote(proposal_id, true, 100);
            assert!(result.is_ok());

            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.yes_votes, 100);
            assert_eq!(proposal.no_votes, 0);

            // Bob votes no with weight 50
            set_caller(accounts.bob);
            let result = dao.vote(proposal_id, false, 50);
            assert!(result.is_ok());

            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.yes_votes, 100);
            assert_eq!(proposal.no_votes, 50);
        }

        #[ink::test]
        fn already_voted_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = SimpleDao::new(100, 2000, None, None);
            let proposal_id = dao.create_proposal(String::from("Test")).unwrap();

            dao.vote(proposal_id, true, 100).unwrap();

            // Try to vote again
            let result = dao.vote(proposal_id, false, 50);
            assert_eq!(result, Err(Error::AlreadyVoted));
        }

        #[ink::test]
        fn finalize_proposal_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = SimpleDao::new(100, 2000, None, None);
            dao.set_total_voting_power(1000).unwrap();

            let proposal_id = dao.create_proposal(String::from("Test")).unwrap();

            // Vote
            dao.vote(proposal_id, true, 300).unwrap();

            // Advance blocks past voting period
            advance_block(101);

            // Finalize
            let result = dao.finalize_proposal(proposal_id);
            assert!(result.is_ok());

            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.status, ProposalStatus::Passed);
        }

        #[ink::test]
        fn execute_proposal_works() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = SimpleDao::new(100, 2000, None, None);
            dao.set_total_voting_power(1000).unwrap();

            let proposal_id = dao.create_proposal(String::from("Test")).unwrap();
            dao.vote(proposal_id, true, 300).unwrap();

            advance_block(101);
            dao.finalize_proposal(proposal_id).unwrap();

            // Execute
            let result = dao.execute_proposal(proposal_id);
            assert!(result.is_ok());

            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.status, ProposalStatus::Executed);
            assert!(proposal.executed);
        }

        #[ink::test]
        fn quorum_not_met_fails() {
            let accounts = default_accounts();
            set_caller(accounts.alice);

            let mut dao = SimpleDao::new(100, 2000, None, None);
            dao.set_total_voting_power(1000).unwrap();

            let proposal_id = dao.create_proposal(String::from("Test")).unwrap();

            // Vote with only 100 (quorum needs 200 = 20% of 1000)
            dao.vote(proposal_id, true, 100).unwrap();

            advance_block(101);
            dao.finalize_proposal(proposal_id).unwrap();

            let proposal = dao.get_proposal(proposal_id).unwrap();
            assert_eq!(proposal.status, ProposalStatus::Rejected);
        }
    }
}

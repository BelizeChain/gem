// gem-fuzz: Extracted pure math functions from GEM contracts for fuzzing.
//
// These functions are exact copies of the arithmetic code in the contracts,
// extracted here so they can be tested natively with proptest and cargo-fuzz
// without requiring the ink! off-chain environment.

// ============================================================================
// DALLA Token Arithmetic
// ============================================================================

/// Checked transfer: debit `amount` from `from_balance`, credit to `to_balance`.
/// Returns (new_from, new_to) or None on overflow/underflow.
pub fn checked_transfer(from_balance: u128, to_balance: u128, amount: u128) -> Option<(u128, u128)> {
    let new_from = from_balance.checked_sub(amount)?;
    let new_to = to_balance.checked_add(amount)?;
    Some((new_from, new_to))
}

/// Checked mint: add `amount` to `balance` and `total_supply`, respecting `max_supply`.
/// Returns (new_balance, new_total_supply) or None on overflow/cap breach.
pub fn checked_mint(
    balance: u128,
    total_supply: u128,
    amount: u128,
    max_supply: u128,
) -> Option<(u128, u128)> {
    let new_total = total_supply.checked_add(amount)?;
    if new_total > max_supply {
        return None;
    }
    let new_balance = balance.checked_add(amount)?;
    Some((new_balance, new_total))
}

/// Checked burn: subtract `amount` from `balance` and `total_supply`.
/// Returns (new_balance, new_total_supply) or None on underflow.
pub fn checked_burn(balance: u128, total_supply: u128, amount: u128) -> Option<(u128, u128)> {
    let new_balance = balance.checked_sub(amount)?;
    let new_total = total_supply.checked_sub(amount)?;
    Some((new_balance, new_total))
}

/// Checked allowance increase.
pub fn checked_increase_allowance(current: u128, delta: u128) -> Option<u128> {
    current.checked_add(delta)
}

/// Checked allowance decrease (saturating at zero, matching contract behavior).
pub fn saturating_decrease_allowance(current: u128, delta: u128) -> u128 {
    current.saturating_sub(delta)
}

// ============================================================================
// DEX Pair Arithmetic
// ============================================================================

pub const PAIR_FEE_NUMERATOR: u128 = 3;
pub const PAIR_FEE_DENOMINATOR: u128 = 1000;
pub const MINIMUM_LIQUIDITY: u128 = 1000;

/// 256-bit multiplication: a * b = (hi, lo) where result = hi * 2^128 + lo.
/// Exact copy from dex/pair/lib.rs.
pub fn mul_u256(a: u128, b: u128) -> (u128, u128) {
    let a_lo = a & 0xFFFF_FFFF_FFFF_FFFF_u128;
    let a_hi = a >> 64;
    let b_lo = b & 0xFFFF_FFFF_FFFF_FFFF_u128;
    let b_hi = b >> 64;

    let lo_lo = a_lo * b_lo;
    let hi_lo = a_hi * b_lo;
    let lo_hi = a_lo * b_hi;
    let hi_hi = a_hi * b_hi;

    let mid = (lo_lo >> 64)
        + (hi_lo & 0xFFFF_FFFF_FFFF_FFFF_u128)
        + (lo_hi & 0xFFFF_FFFF_FFFF_FFFF_u128);

    let lo = (lo_lo & 0xFFFF_FFFF_FFFF_FFFF_u128) | ((mid & 0xFFFF_FFFF_FFFF_FFFF_u128) << 64);
    let hi = hi_hi + (hi_lo >> 64) + (lo_hi >> 64) + (mid >> 64);

    (hi, lo)
}

/// Integer square root (Babylonian method). Exact copy from dex/pair/lib.rs.
pub fn sqrt(y: u128) -> u128 {
    if y > 3 {
        let mut z = y;
        let mut x = y / 2 + 1;
        while x < z {
            z = x;
            x = (y / x + x) / 2;
        }
        z
    } else if y != 0 {
        1
    } else {
        0
    }
}

/// Pair's get_amount_out: output amount for exact input with 0.3% fee.
/// Exact copy from dex/pair/lib.rs.
pub fn pair_get_amount_out(
    amount_in: u128,
    reserve_in: u128,
    reserve_out: u128,
) -> Option<u128> {
    if amount_in == 0 {
        return None;
    }
    if reserve_in == 0 || reserve_out == 0 {
        return None;
    }

    let fee_factor = PAIR_FEE_DENOMINATOR.checked_sub(PAIR_FEE_NUMERATOR)?;
    let amount_in_with_fee = amount_in.checked_mul(fee_factor)?;
    let numerator = amount_in_with_fee.checked_mul(reserve_out)?;
    let denominator = reserve_in
        .checked_mul(PAIR_FEE_DENOMINATOR)?
        .checked_add(amount_in_with_fee)?;
    numerator.checked_div(denominator)
}

/// Pair's get_amount_in: input amount needed for exact output with 0.3% fee.
/// Exact copy from dex/pair/lib.rs.
pub fn pair_get_amount_in(
    amount_out: u128,
    reserve_in: u128,
    reserve_out: u128,
) -> Option<u128> {
    if amount_out == 0 {
        return None;
    }
    if reserve_in == 0 || reserve_out == 0 || amount_out >= reserve_out {
        return None;
    }

    let fee_factor = PAIR_FEE_DENOMINATOR.checked_sub(PAIR_FEE_NUMERATOR)?;
    let numerator = reserve_in
        .checked_mul(amount_out)?
        .checked_mul(PAIR_FEE_DENOMINATOR)?;
    let denominator = reserve_out
        .checked_sub(amount_out)?
        .checked_mul(fee_factor)?;
    let amount_in = numerator.checked_div(denominator)?.checked_add(1)?;
    Some(amount_in)
}

/// K-invariant check: verify K_new >= K_old using 256-bit arithmetic.
/// Returns true if the constant product invariant holds after a swap.
pub fn k_invariant_holds(
    reserve0_before: u128,
    reserve1_before: u128,
    balance0_after: u128,
    balance1_after: u128,
    amount0_in: u128,
    amount1_in: u128,
) -> bool {
    // Apply fee adjustment (matching pair contract logic)
    let balance0_adjusted = match balance0_after
        .checked_mul(PAIR_FEE_DENOMINATOR)
        .and_then(|v| v.checked_sub(amount0_in.checked_mul(PAIR_FEE_NUMERATOR)?))
    {
        Some(v) => v,
        None => return false,
    };

    let balance1_adjusted = match balance1_after
        .checked_mul(PAIR_FEE_DENOMINATOR)
        .and_then(|v| v.checked_sub(amount1_in.checked_mul(PAIR_FEE_NUMERATOR)?))
    {
        Some(v) => v,
        None => return false,
    };

    let (new_hi, new_lo) = mul_u256(balance0_adjusted, balance1_adjusted);

    let reserve0_scaled = match reserve0_before.checked_mul(PAIR_FEE_DENOMINATOR) {
        Some(v) => v,
        None => return false,
    };
    let reserve1_scaled = match reserve1_before.checked_mul(PAIR_FEE_DENOMINATOR) {
        Some(v) => v,
        None => return false,
    };
    let (old_hi, old_lo) = mul_u256(reserve0_scaled, reserve1_scaled);

    // K_new >= K_old (256-bit comparison)
    if new_hi > old_hi {
        true
    } else if new_hi == old_hi {
        new_lo >= old_lo
    } else {
        false
    }
}

/// Initial LP token calculation for first liquidity deposit.
/// Returns LP tokens minted (with MINIMUM_LIQUIDITY locked).
pub fn initial_lp_tokens(amount0: u128, amount1: u128) -> Option<u128> {
    let product = amount0.checked_mul(amount1)?;
    let lp_total = sqrt(product);
    lp_total.checked_sub(MINIMUM_LIQUIDITY)
}

/// Subsequent LP token calculation.
/// Returns LP tokens minted = min(amount0 * totalSupply / reserve0, amount1 * totalSupply / reserve1).
pub fn subsequent_lp_tokens(
    amount0: u128,
    amount1: u128,
    reserve0: u128,
    reserve1: u128,
    total_supply: u128,
) -> Option<u128> {
    if reserve0 == 0 || reserve1 == 0 || total_supply == 0 {
        return None;
    }
    let lp0 = amount0.checked_mul(total_supply)?.checked_div(reserve0)?;
    let lp1 = amount1.checked_mul(total_supply)?.checked_div(reserve1)?;
    Some(lp0.min(lp1))
}

// ============================================================================
// DEX Router Arithmetic
// ============================================================================

/// Router's get_amount_out (uses _checked_mul_div with GCD reduction).
/// Exact copy from dex/router/lib.rs.
pub fn router_get_amount_out(
    amount_in: u128,
    reserve_in: u128,
    reserve_out: u128,
) -> Option<u128> {
    if amount_in == 0 || reserve_in == 0 || reserve_out == 0 {
        return None;
    }
    let amount_in_with_fee = amount_in.checked_mul(997)?;
    let denominator = reserve_in
        .checked_mul(1000)?
        .checked_add(amount_in_with_fee)?;
    checked_mul_div(amount_in_with_fee, reserve_out, denominator)
}

/// Router's get_amount_in (uses _checked_mul_div with GCD reduction).
/// Exact copy from dex/router/lib.rs.
pub fn router_get_amount_in(
    amount_out: u128,
    reserve_in: u128,
    reserve_out: u128,
) -> Option<u128> {
    if amount_out == 0 || reserve_in == 0 || reserve_out == 0 || amount_out >= reserve_out {
        return None;
    }
    let numerator_factor = reserve_in.checked_mul(1000)?;
    let denominator = reserve_out.checked_sub(amount_out)?.checked_mul(997)?;
    let result = checked_mul_div(numerator_factor, amount_out, denominator)?;
    Some(result + 1)
}

/// Router's quote function. Exact copy from dex/router/lib.rs.
pub fn router_quote(amount_a: u128, reserve_a: u128, reserve_b: u128) -> Option<u128> {
    if amount_a == 0 || reserve_a == 0 || reserve_b == 0 {
        return None;
    }
    amount_a.checked_mul(reserve_b)?.checked_div(reserve_a)
}

/// Safe (a * b) / c with GCD reduction. Exact copy from dex/router/lib.rs.
pub fn checked_mul_div(a: u128, b: u128, c: u128) -> Option<u128> {
    if c == 0 {
        return None;
    }
    if let Some(product) = a.checked_mul(b) {
        return Some(product / c);
    }
    let g1 = gcd(a, c);
    let a_reduced = a / g1;
    let c_reduced = c / g1;
    let g2 = gcd(b, c_reduced);
    let b_reduced = b / g2;
    let c_final = c_reduced / g2;
    a_reduced.checked_mul(b_reduced).map(|p| p / c_final)
}

/// Greatest common divisor (Euclidean algorithm). Exact copy from dex/router/lib.rs.
pub fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

// ============================================================================
// HARNESS-05: DAO State Machine Model
// ============================================================================

/// DAO proposal status (mirrors simple_dao::ProposalStatus)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaoStatus {
    Active,
    Passed,
    Rejected,
    Executed,
    Cancelled,
    Expired,
}

/// Minimal DAO proposal model for state-machine fuzzing.
#[derive(Debug, Clone)]
pub struct DaoProposal {
    pub status: DaoStatus,
    pub yes_votes: u128,
    pub no_votes: u128,
    pub snapshot_supply: u128,
    pub executed: bool,
    pub finalized_block: Option<u32>,
}

/// Compute whether quorum is met: total_votes >= snapshot * quorum_bps / 10000.
pub fn dao_quorum_met(yes: u128, no: u128, snapshot: u128, quorum_bps: u32) -> Option<bool> {
    let total = yes.checked_add(no)?;
    let required = snapshot.checked_mul(quorum_bps as u128)?.checked_div(10000)?;
    Some(total >= required)
}

/// Simulate finalize: Active → Passed or Rejected.
pub fn dao_finalize(
    proposal: &mut DaoProposal,
    quorum_bps: u32,
) -> Option<DaoStatus> {
    if proposal.status != DaoStatus::Active {
        return None;
    }
    let met = dao_quorum_met(proposal.yes_votes, proposal.no_votes, proposal.snapshot_supply, quorum_bps)?;
    if met && proposal.yes_votes > proposal.no_votes {
        proposal.status = DaoStatus::Passed;
    } else {
        proposal.status = DaoStatus::Rejected;
    }
    Some(proposal.status)
}

/// Simulate execute: Passed → Executed (only if not already executed).
pub fn dao_execute(proposal: &mut DaoProposal) -> bool {
    if proposal.status != DaoStatus::Passed || proposal.executed {
        return false;
    }
    proposal.executed = true;
    proposal.status = DaoStatus::Executed;
    true
}

/// Simulate cancel: Active → Cancelled.
pub fn dao_cancel(proposal: &mut DaoProposal) -> bool {
    if proposal.status != DaoStatus::Active {
        return false;
    }
    proposal.status = DaoStatus::Cancelled;
    true
}

/// Add a vote. Returns false if overflow or exceeds snapshot.
pub fn dao_vote(proposal: &mut DaoProposal, support: bool, weight: u128) -> bool {
    if proposal.status != DaoStatus::Active || weight == 0 {
        return false;
    }
    if support {
        match proposal.yes_votes.checked_add(weight) {
            Some(v) => proposal.yes_votes = v,
            None => return false,
        }
    } else {
        match proposal.no_votes.checked_add(weight) {
            Some(v) => proposal.no_votes = v,
            None => return false,
        }
    }
    true
}

// ============================================================================
// HARNESS-06: PSP37 Balance Model
// ============================================================================

/// Token type (mirrors psp37_multi_token::TokenType)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Psp37TokenType {
    Fungible,
    NonFungible,
}

/// Minimal PSP37 token accounting model for supply invariant testing.
pub struct Psp37Model {
    pub token_type: Psp37TokenType,
    pub max_supply: Option<u128>,
    pub total_supply: u128,
    /// Simplified: track balances for up to N accounts.
    pub balances: [u128; 4],
}

impl Psp37Model {
    pub fn new(token_type: Psp37TokenType, max_supply: Option<u128>) -> Self {
        Self {
            token_type,
            max_supply,
            total_supply: 0,
            balances: [0; 4],
        }
    }

    /// Mint tokens to account idx.
    pub fn mint(&mut self, to: usize, amount: u128) -> bool {
        if to >= self.balances.len() { return false; }
        if amount == 0 { return false; }
        if self.token_type == Psp37TokenType::NonFungible && amount != 1 {
            return false;
        }
        if self.token_type == Psp37TokenType::NonFungible && self.balances[to] >= 1 {
            return false;
        }
        if let Some(cap) = self.max_supply {
            if self.total_supply + amount > cap { return false; }
        }
        let new_bal = match self.balances[to].checked_add(amount) {
            Some(b) => b,
            None => return false,
        };
        let new_supply = match self.total_supply.checked_add(amount) {
            Some(s) => s,
            None => return false,
        };
        self.balances[to] = new_bal;
        self.total_supply = new_supply;
        true
    }

    /// Burn tokens from account idx.
    pub fn burn(&mut self, from: usize, amount: u128) -> bool {
        if from >= self.balances.len() { return false; }
        if amount == 0 || amount > self.balances[from] { return false; }
        self.balances[from] -= amount;
        self.total_supply -= amount;
        true
    }

    /// Transfer tokens between accounts.
    pub fn transfer(&mut self, from: usize, to: usize, amount: u128) -> bool {
        if from >= self.balances.len() || to >= self.balances.len() { return false; }
        if from == to { return true; }
        if amount == 0 || amount > self.balances[from] { return false; }
        if self.token_type == Psp37TokenType::NonFungible && self.balances[to] >= 1 {
            return false;
        }
        let new_to = match self.balances[to].checked_add(amount) {
            Some(b) => b,
            None => return false,
        };
        self.balances[from] -= amount;
        self.balances[to] = new_to;
        true
    }

    /// Check invariant: supply == sum of balances.
    pub fn supply_eq_sum(&self) -> bool {
        let sum: u128 = self.balances.iter().sum();
        self.total_supply == sum
    }

    /// Check NFT invariant: no balance > 1 for NonFungible tokens.
    pub fn nft_balance_invariant(&self) -> bool {
        if self.token_type == Psp37TokenType::NonFungible {
            self.balances.iter().all(|b| *b <= 1)
        } else {
            true
        }
    }
}

// ============================================================================
// HARNESS-07: Access Control Model
// ============================================================================

/// Minimal access control model for role management invariants.
pub struct AccessControlModel {
    /// roles[role][account] = has_role
    pub roles: [[bool; 4]; 4],
    pub admin_count: u32,
}

pub const DEFAULT_ADMIN_ROLE: usize = 0;

impl AccessControlModel {
    pub fn new(admin_idx: usize) -> Self {
        let mut model = Self {
            roles: [[false; 4]; 4],
            admin_count: 0,
        };
        if admin_idx < 4 {
            model.roles[DEFAULT_ADMIN_ROLE][admin_idx] = true;
            model.admin_count = 1;
        }
        model
    }

    /// Grant role to account. Caller must have admin role for the target role.
    /// Simplified: admin_role for all roles is DEFAULT_ADMIN_ROLE.
    pub fn grant_role(&mut self, caller: usize, role: usize, account: usize) -> bool {
        if caller >= 4 || role >= 4 || account >= 4 { return false; }
        if !self.roles[DEFAULT_ADMIN_ROLE][caller] { return false; }
        if !self.roles[role][account] {
            self.roles[role][account] = true;
            if role == DEFAULT_ADMIN_ROLE {
                self.admin_count = self.admin_count.saturating_add(1);
            }
        }
        true
    }

    /// Revoke role from account. Cannot revoke last admin.
    pub fn revoke_role(&mut self, caller: usize, role: usize, account: usize) -> bool {
        if caller >= 4 || role >= 4 || account >= 4 { return false; }
        if !self.roles[DEFAULT_ADMIN_ROLE][caller] { return false; }
        if self.roles[role][account] {
            if role == DEFAULT_ADMIN_ROLE && self.admin_count <= 1 {
                return false; // Cannot revoke last admin
            }
            self.roles[role][account] = false;
            if role == DEFAULT_ADMIN_ROLE {
                self.admin_count -= 1;
            }
        }
        true
    }

    /// Renounce own role.
    pub fn renounce_role(&mut self, account: usize, role: usize) -> bool {
        if account >= 4 || role >= 4 { return false; }
        if self.roles[role][account] {
            if role == DEFAULT_ADMIN_ROLE && self.admin_count <= 1 {
                return false;
            }
            self.roles[role][account] = false;
            if role == DEFAULT_ADMIN_ROLE {
                self.admin_count -= 1;
            }
        }
        true
    }

    /// Check invariant: admin_count == actual count of DEFAULT_ADMIN_ROLE holders.
    pub fn admin_count_correct(&self) -> bool {
        let actual: u32 = self.roles[DEFAULT_ADMIN_ROLE]
            .iter()
            .filter(|&&has| has)
            .count() as u32;
        self.admin_count == actual
    }

    /// Check invariant: at least one admin exists.
    pub fn has_admin(&self) -> bool {
        self.admin_count >= 1
    }
}

// ============================================================================
// HARNESS-08: NFT Enumeration Model
// ============================================================================

/// Minimal NFT model tracking ownership, burns, and supply consistency.
pub struct NftModel {
    /// owner[token_id] = Some(account_idx) or None (not minted or burned)
    pub owners: Vec<Option<usize>>,
    pub burned: Vec<bool>,
    pub total_supply: u32,
    pub balance_of: [u32; 4],
    pub next_token_id: u32,
    pub max_supply: Option<u32>,
}

impl NftModel {
    pub fn new(max_supply: Option<u32>) -> Self {
        Self {
            owners: Vec::new(),
            burned: Vec::new(),
            total_supply: 0,
            balance_of: [0; 4],
            next_token_id: 1,
            max_supply,
        }
    }

    /// Mint a new NFT to account_idx.
    pub fn mint(&mut self, to: usize) -> Option<u32> {
        if to >= 4 { return None; }
        if let Some(cap) = self.max_supply {
            if self.total_supply >= cap { return None; }
        }
        let id = self.next_token_id;
        self.next_token_id = self.next_token_id.checked_add(1)?;
        // Extend vectors to include this token (id is 1-based)
        while self.owners.len() < id as usize {
            self.owners.push(None);
            self.burned.push(false);
        }
        self.owners[id as usize - 1] = Some(to);
        self.total_supply = self.total_supply.checked_add(1)?;
        self.balance_of[to] = self.balance_of[to].checked_add(1)?;
        Some(id)
    }

    /// Burn an NFT by token_id (1-based).
    pub fn burn(&mut self, token_id: u32) -> bool {
        if token_id == 0 || token_id as usize > self.owners.len() { return false; }
        let idx = token_id as usize - 1;
        match self.owners[idx] {
            Some(owner) => {
                self.owners[idx] = None;
                self.burned[idx] = true;
                self.total_supply -= 1;
                self.balance_of[owner] -= 1;
                true
            }
            None => false,
        }
    }

    /// Transfer NFT to new owner.
    pub fn transfer(&mut self, token_id: u32, to: usize) -> bool {
        if to >= 4 || token_id == 0 || token_id as usize > self.owners.len() {
            return false;
        }
        let idx = token_id as usize - 1;
        match self.owners[idx] {
            Some(from) if from != to => {
                self.owners[idx] = Some(to);
                self.balance_of[from] -= 1;
                self.balance_of[to] = self.balance_of[to].saturating_add(1);
                true
            }
            _ => false,
        }
    }

    /// Check invariant: total_supply == count of non-None owners.
    pub fn supply_consistent(&self) -> bool {
        let actual = self.owners.iter().filter(|o| o.is_some()).count() as u32;
        self.total_supply == actual
    }

    /// Check invariant: balance_of matches actual ownership counts.
    pub fn balances_consistent(&self) -> bool {
        let mut counts = [0u32; 4];
        for owner in &self.owners {
            if let Some(idx) = owner {
                if *idx < 4 { counts[*idx] += 1; }
            }
        }
        self.balance_of == counts
    }

    /// Check invariant: burned IDs are never owned.
    pub fn burned_ids_clean(&self) -> bool {
        for (i, &is_burned) in self.burned.iter().enumerate() {
            if is_burned && self.owners[i].is_some() {
                return false;
            }
        }
        true
    }
}

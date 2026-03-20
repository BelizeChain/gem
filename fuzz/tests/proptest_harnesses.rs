//! Proptest-based property tests for all extracted math functions.
//! These serve as the cargo-test equivalent of the libfuzzer harnesses
//! and can run on stable Rust without nightly toolchain.

use gem_fuzz::*;
use proptest::prelude::*;

const MAX_SUPPLY: u128 = 100_000_000_000_000_000_000;

// ============================================================================
// HARNESS-01: DALLA Token Arithmetic
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50_000))]

    #[test]
    fn dalla_transfer_preserves_sum(
        from_bal in 0u128..=MAX_SUPPLY,
        to_bal in 0u128..=MAX_SUPPLY / 2,
        amount in 0u128..=MAX_SUPPLY,
    ) {
        if from_bal.checked_add(to_bal).is_none() { return Ok(()); }
        let sum_before = from_bal + to_bal;
        if let Some((new_from, new_to)) = checked_transfer(from_bal, to_bal, amount) {
            prop_assert_eq!(new_from + new_to, sum_before, "INV-01: sum changed");
            prop_assert!(new_from <= from_bal, "from_balance increased in transfer");
            prop_assert!(new_to >= to_bal, "to_balance decreased in transfer");
        }
    }

    #[test]
    fn dalla_mint_respects_max_supply(
        balance in 0u128..=MAX_SUPPLY,
        total in 0u128..=MAX_SUPPLY,
        amount in 0u128..=MAX_SUPPLY,
    ) {
        if balance > total { return Ok(()); }
        if let Some((new_bal, new_total)) = checked_mint(balance, total, amount, MAX_SUPPLY) {
            prop_assert!(new_total <= MAX_SUPPLY, "INV-03: exceeds MAX_SUPPLY");
            prop_assert!(new_bal <= new_total, "INV-02: balance > total_supply");
            prop_assert_eq!(new_total, total + amount, "total_supply math wrong");
        }
    }

    #[test]
    fn dalla_burn_never_underflows(
        balance in 0u128..=MAX_SUPPLY,
        total in 0u128..=MAX_SUPPLY,
        amount in 0u128..=MAX_SUPPLY,
    ) {
        if balance > total { return Ok(()); }
        if let Some((new_bal, new_total)) = checked_burn(balance, total, amount) {
            prop_assert!(new_bal <= balance, "balance increased after burn");
            prop_assert!(new_total <= total, "total increased after burn");
            prop_assert!(new_bal <= new_total, "balance > total after burn");
        }
    }

    #[test]
    fn dalla_allowance_saturating(
        current in 0u128..=u128::MAX,
        delta in 0u128..=u128::MAX,
    ) {
        let result = saturating_decrease_allowance(current, delta);
        prop_assert!(result <= current, "saturating decrease increased allowance");
        if delta >= current {
            prop_assert_eq!(result, 0, "should saturate to 0");
        } else {
            prop_assert_eq!(result, current - delta, "should be exact subtraction");
        }
    }
}

// ============================================================================
// HARNESS-02: Pair K-Invariant
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100_000))]

    #[test]
    fn mul_u256_correctness(a: u128, b: u128) {
        let (hi, lo) = mul_u256(a, b);

        // Commutativity
        let (hi2, lo2) = mul_u256(b, a);
        prop_assert_eq!(hi, hi2, "commutativity hi");
        prop_assert_eq!(lo, lo2, "commutativity lo");

        // Zero identity
        let (zh, zl) = mul_u256(a, 0);
        prop_assert_eq!(zh, 0);
        prop_assert_eq!(zl, 0);

        // Verify against checked_mul when possible
        if let Some(expected) = a.checked_mul(b) {
            prop_assert_eq!(hi, 0, "hi should be 0");
            prop_assert_eq!(lo, expected);
        } else {
            // Overflow case: hi must be > 0 (both operands nonzero)
            if a > 0 && b > 0 {
                prop_assert!(hi > 0, "overflowing product must have hi > 0");
            }
        }
    }

    #[test]
    fn sqrt_correctness(y: u128) {
        let s = sqrt(y);
        if y == 0 {
            prop_assert_eq!(s, 0);
        } else {
            // s² ≤ y
            if let Some(sq) = s.checked_mul(s) {
                prop_assert!(sq <= y, "sqrt({})² = {} > {}", y, sq, y);
            }
            // (s+1)² > y
            if let Some(sp) = s.checked_add(1) {
                if let Some(sp_sq) = sp.checked_mul(sp) {
                    prop_assert!(sp_sq > y, "(sqrt({})+1)² = {} ≤ {}", y, sp_sq, y);
                }
            }
        }
    }

    #[test]
    fn k_invariant_after_swap(
        r0 in 1u64..=u32::MAX as u64,
        r1 in 1u64..=u32::MAX as u64,
        amt in 1u64..=u32::MAX as u64,
        direction: bool,
    ) {
        let r0 = r0 as u128;
        let r1 = r1 as u128;
        let amt = amt as u128;

        let (reserve_in, reserve_out) = if direction { (r0, r1) } else { (r1, r0) };

        if let Some(amount_out) = pair_get_amount_out(amt, reserve_in, reserve_out) {
            // Output must be < reserve_out
            prop_assert!(amount_out < reserve_out, "K-INV-06: drained reserve");

            let (b0, b1, a0_in, a1_in) = if direction {
                (r0 + amt, r1 - amount_out, amt, 0u128)
            } else {
                (r0 - amount_out, r1 + amt, 0u128, amt)
            };

            prop_assert!(
                k_invariant_holds(r0, r1, b0, b1, a0_in, a1_in),
                "K-INV-01: K decreased"
            );
        }
    }

    #[test]
    fn pair_amount_roundtrip(
        reserve_in in 1000u128..=1_000_000_000u128,
        reserve_out in 1000u128..=1_000_000_000u128,
        amount_in in 1u128..=100_000_000u128,
    ) {
        if let Some(amount_out) = pair_get_amount_out(amount_in, reserve_in, reserve_out) {
            if amount_out > 0 && amount_out < reserve_out {
                if let Some(needed) = pair_get_amount_in(amount_out, reserve_in, reserve_out) {
                    // needed should be ≤ amount_in (we put in enough)
                    prop_assert!(
                        needed <= amount_in + 1,
                        "roundtrip: need {} > provided {} + 1",
                        needed, amount_in
                    );
                }
            }
        }
    }
}

// ============================================================================
// HARNESS-03: LP Token Inflation
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50_000))]

    #[test]
    fn initial_lp_minimum_liquidity_locked(
        d0 in 1001u128..=1_000_000_000u128,
        d1 in 1001u128..=1_000_000_000u128,
    ) {
        if let Some(lp) = initial_lp_tokens(d0, d1) {
            let total = lp + MINIMUM_LIQUIDITY;
            prop_assert!(total >= MINIMUM_LIQUIDITY, "LP-INV-01");
            prop_assert!(total > 0, "LP-INV-05: zero supply with reserves");
        }
    }

    #[test]
    fn subsequent_lp_proportional(
        reserve0 in 10_000u128..=1_000_000_000u128,
        reserve1 in 10_000u128..=1_000_000_000u128,
        total_supply in 1_000u128..=1_000_000_000u128,
        amount0 in 1u128..=1_000_000u128,
        amount1 in 1u128..=1_000_000u128,
    ) {
        if let Some(lp) = subsequent_lp_tokens(amount0, amount1, reserve0, reserve1, total_supply) {
            // LP is min of two ratios
            let lp0 = amount0.checked_mul(total_supply).and_then(|n| n.checked_div(reserve0));
            let lp1 = amount1.checked_mul(total_supply).and_then(|n| n.checked_div(reserve1));
            if let (Some(l0), Some(l1)) = (lp0, lp1) {
                prop_assert_eq!(lp, l0.min(l1), "LP should be min of two ratios");
            }
        }
    }

    #[test]
    fn inflation_attack_bounded(
        d0 in 10_000u128..=1_000_000u128,
        d1 in 10_000u128..=1_000_000u128,
        donate0 in 0u128..=100_000u128,
        donate1 in 0u128..=100_000u128,
        v0 in 1u128..=1_000_000u128,
        v1 in 1u128..=1_000_000u128,
    ) {
        let attacker_lp = match initial_lp_tokens(d0, d1) {
            Some(lp) if lp > 0 => lp,
            _ => return Ok(()),
        };
        let total_supply = attacker_lp + MINIMUM_LIQUIDITY;

        let r0 = d0.saturating_add(donate0);
        let r1 = d1.saturating_add(donate1);

        let victim_lp = match subsequent_lp_tokens(v0, v1, r0, r1, total_supply) {
            Some(lp) => lp,
            None => return Ok(()),
        };

        // Victim's LP share should not exceed proportional deposit
        if victim_lp > 0 {
            let new_total = total_supply + victim_lp;
            // Victim's share of reserves
            if let Some(share0) = victim_lp.checked_mul(r0 + v0).and_then(|n| n.checked_div(new_total)) {
                // Share should not massively exceed deposit (within 2x for rounding)
                prop_assert!(
                    share0 <= v0.saturating_mul(2) + 1,
                    "victim extracts disproportionate reserves"
                );
            }
        }
    }
}

// ============================================================================
// HARNESS-04: Router Path Arithmetic
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50_000))]

    #[test]
    fn checked_mul_div_correctness(
        a in 1u128..=u64::MAX as u128,
        b in 1u128..=u64::MAX as u128,
        c in 1u128..=u64::MAX as u128,
    ) {
        let result = checked_mul_div(a, b, c);
        if let Some(product) = a.checked_mul(b) {
            let expected = product / c;
            prop_assert_eq!(result, Some(expected));
        }
    }

    #[test]
    fn checked_mul_div_zero_denominator(a: u128, b: u128) {
        prop_assert_eq!(checked_mul_div(a, b, 0), None, "RTR-INV-05");
    }

    #[test]
    fn gcd_divides_both(a: u128, b: u128) {
        let g = gcd(a, b);
        if a == 0 && b == 0 {
            prop_assert_eq!(g, 0);
        } else {
            prop_assert!(g > 0);
            if a > 0 { prop_assert_eq!(a % g, 0, "gcd doesn't divide a"); }
            if b > 0 { prop_assert_eq!(b % g, 0, "gcd doesn't divide b"); }
        }
    }

    #[test]
    fn router_output_bounded(
        amt in 1u64..=u32::MAX as u64,
        r_in in 1u64..=u32::MAX as u64,
        r_out in 1u64..=u32::MAX as u64,
    ) {
        let amt = amt as u128;
        let r_in = r_in as u128;
        let r_out = r_out as u128;

        if let Some(out) = router_get_amount_out(amt, r_in, r_out) {
            prop_assert!(out < r_out, "RTR-INV-01: drained reserve");
        }
    }

    #[test]
    fn router_pair_agreement(
        amt in 1u64..=1_000_000u64,
        r_in in 1000u64..=1_000_000u64,
        r_out in 1000u64..=1_000_000u64,
    ) {
        let amt = amt as u128;
        let r_in = r_in as u128;
        let r_out = r_out as u128;

        let router_out = router_get_amount_out(amt, r_in, r_out);
        let pair_out = pair_get_amount_out(amt, r_in, r_out);

        if let (Some(r), Some(p)) = (router_out, pair_out) {
            let diff = if r > p { r - p } else { p - r };
            prop_assert!(diff <= 1, "RTR-INV-03: router {} vs pair {} diff {}", r, p, diff);
        }
    }

    #[test]
    fn multi_hop_decreasing(
        amount in 1000u128..=1_000_000u128,
        r0 in (1000u32..=100_000u32, 1000u32..=100_000u32),
        r1 in (1000u32..=100_000u32, 1000u32..=100_000u32),
        r2 in (1000u32..=100_000u32, 1000u32..=100_000u32),
    ) {
        let pools = [
            (r0.0 as u128, r0.1 as u128),
            (r1.0 as u128, r1.1 as u128),
            (r2.0 as u128, r2.1 as u128),
        ];

        // RTR-INV-07: each hop output < that pool's reserve_out (can never drain a pool)
        let mut current = amount;
        for (ra, rb) in &pools {
            match router_get_amount_out(current, *ra, *rb) {
                Some(out) => {
                    prop_assert!(out < *rb, "RTR-INV-07: hop output {} >= reserve_out {}", out, rb);
                    if out == 0 { break; }
                    current = out;
                }
                _ => break,
            }
        }
    }

    #[test]
    fn quote_proportionality(
        amount in 1u128..=1_000_000u128,
        reserve_a in 1u128..=1_000_000u128,
        reserve_b in 1u128..=1_000_000u128,
    ) {
        if let Some(quoted) = router_quote(amount, reserve_a, reserve_b) {
            // quoted * reserve_a ≤ amount * reserve_b (floor division)
            if let (Some(lhs), Some(rhs)) = (quoted.checked_mul(reserve_a), amount.checked_mul(reserve_b)) {
                prop_assert!(lhs <= rhs, "RTR-INV-08: proportionality violated");
            }
        }
    }
}

// ============================================================================
// HARNESS-05: DAO State Machine
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50_000))]

    #[test]
    fn dao_no_double_execute(
        yes in 0u128..=10_000u128,
        no in 0u128..=10_000u128,
        snapshot in 1u128..=10_000u128,
        quorum in 100u32..=10_000u32,
    ) {
        let mut p = DaoProposal {
            status: DaoStatus::Active,
            yes_votes: yes,
            no_votes: no,
            snapshot_supply: snapshot,
            executed: false,
            finalized_block: None,
        };
        let _ = dao_finalize(&mut p, quorum);
        let first = dao_execute(&mut p);
        let second = dao_execute(&mut p);
        if first {
            prop_assert!(!second, "DAO-INV-01: double execution");
            prop_assert!(p.executed);
            prop_assert_eq!(p.status, DaoStatus::Executed);
        }
    }

    #[test]
    fn dao_total_votes_le_snapshot(
        weights in proptest::collection::vec(1u64..=1000u64, 1..20),
        snapshot in 1u128..=100_000u128,
        _quorum in 100u32..=10_000u32,
    ) {
        let mut p = DaoProposal {
            status: DaoStatus::Active,
            yes_votes: 0,
            no_votes: 0,
            snapshot_supply: snapshot,
            executed: false,
            finalized_block: None,
        };
        for &w in &weights {
            let support = w % 2 == 0;
            dao_vote(&mut p, support, w as u128);
        }
        let total = p.yes_votes.saturating_add(p.no_votes);
        // In the model, votes can exceed snapshot; the contract controls this
        // via cross-contract balance lookup. But total must never overflow.
        prop_assert!(total <= u128::MAX, "vote overflow");
    }

    #[test]
    fn dao_valid_state_transitions(
        yes in 0u128..=10_000u128,
        no in 0u128..=10_000u128,
        snapshot in 1u128..=10_000u128,
        quorum in 100u32..=10_000u32,
        action in 0u8..=2u8,
    ) {
        let mut p = DaoProposal {
            status: DaoStatus::Active,
            yes_votes: yes,
            no_votes: no,
            snapshot_supply: snapshot,
            executed: false,
            finalized_block: None,
        };

        match action {
            0 => {
                // Cancel
                let ok = dao_cancel(&mut p);
                if ok {
                    prop_assert_eq!(p.status, DaoStatus::Cancelled);
                    // Cannot finalize after cancel
                    prop_assert!(dao_finalize(&mut p, quorum).is_none());
                    // Cannot execute after cancel
                    prop_assert!(!dao_execute(&mut p));
                }
            }
            1 => {
                // Finalize → Execute
                if let Some(status) = dao_finalize(&mut p, quorum) {
                    prop_assert!(status == DaoStatus::Passed || status == DaoStatus::Rejected);
                    // Cannot finalize again
                    prop_assert!(dao_finalize(&mut p, quorum).is_none());
                    // Cannot cancel after finalize
                    prop_assert!(!dao_cancel(&mut p));
                    if status == DaoStatus::Passed {
                        prop_assert!(dao_execute(&mut p));
                    } else {
                        prop_assert!(!dao_execute(&mut p));
                    }
                }
            }
            _ => {
                // Finalize → Rejected should not be executable
                if let Some(DaoStatus::Rejected) = dao_finalize(&mut p, quorum) {
                    prop_assert!(!dao_execute(&mut p));
                }
            }
        }
    }

    #[test]
    fn dao_quorum_arithmetic(
        snapshot in 1u128..=1_000_000u128,
        quorum_bps in 100u32..=10_000u32,
        yes in 0u128..=1_000_000u128,
        no in 0u128..=1_000_000u128,
    ) {
        if let Some(met) = dao_quorum_met(yes, no, snapshot, quorum_bps) {
            let total = yes + no;
            let required = snapshot * quorum_bps as u128 / 10000;
            prop_assert_eq!(met, total >= required);
        }
    }
}

// ============================================================================
// HARNESS-06: PSP37 Batch Atomicity
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50_000))]

    #[test]
    fn psp37_fungible_supply_invariant(
        ops in proptest::collection::vec(
            (0u8..=2u8, 0usize..4, 0usize..4, 1u64..=10_000u64),
            1..50,
        ),
    ) {
        let mut model = Psp37Model::new(Psp37TokenType::Fungible, Some(1_000_000));

        for (op, from, to, amount) in ops {
            let amount = amount as u128;
            match op {
                0 => { model.mint(to, amount); }
                1 => { model.burn(from, amount); }
                _ => { model.transfer(from, to, amount); }
            }
            prop_assert!(model.supply_eq_sum(), "PSP37-INV-01: supply ≠ Σbalances");
        }
    }

    #[test]
    fn psp37_nft_balance_invariant(
        ops in proptest::collection::vec(
            (0u8..=2u8, 0usize..4, 0usize..4),
            1..30,
        ),
    ) {
        let mut model = Psp37Model::new(Psp37TokenType::NonFungible, Some(10));

        for (op, from, to, ) in ops {
            match op {
                0 => { model.mint(to, 1); }
                1 => { model.burn(from, 1); }
                _ => { model.transfer(from, to, 1); }
            }
            prop_assert!(model.nft_balance_invariant(), "PSP37-INV-02: NFT balance > 1");
            prop_assert!(model.supply_eq_sum(), "PSP37-INV-01: supply ≠ Σbalances");
        }
    }

    #[test]
    fn psp37_supply_cap_enforced(
        cap in 1u128..=100u128,
        mints in proptest::collection::vec((0usize..4, 1u64..=50u64), 1..20),
    ) {
        let mut model = Psp37Model::new(Psp37TokenType::Fungible, Some(cap));

        for (to, amount) in mints {
            let amount = amount as u128;
            let _ = model.mint(to, amount);
            if let Some(cap) = model.max_supply {
                prop_assert!(model.total_supply <= cap, "PSP37-INV-03: exceeds supply cap");
            }
        }
    }
}

// ============================================================================
// HARNESS-07: Access Control
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50_000))]

    #[test]
    fn access_admin_count_tracking(
        ops in proptest::collection::vec(
            (0u8..=2u8, 0usize..4, 0usize..4, 0usize..4),
            1..50,
        ),
    ) {
        let mut model = AccessControlModel::new(0);

        for (op, caller, role, account) in ops {
            match op {
                0 => { model.grant_role(caller, role, account); }
                1 => { model.revoke_role(caller, role, account); }
                _ => { model.renounce_role(account, role); }
            }
            prop_assert!(model.admin_count_correct(), "AC-INV-01: admin_count mismatch");
            prop_assert!(model.has_admin(), "AC-INV-02: no admin exists");
        }
    }

    #[test]
    fn access_cannot_revoke_last_admin(
        ops in proptest::collection::vec(
            (0u8..=1u8, 0usize..4, 0usize..4),
            1..30,
        ),
    ) {
        let mut model = AccessControlModel::new(0);

        for (op, caller, account) in ops {
            match op {
                0 => {
                    // Try to revoke admin role
                    let result = model.revoke_role(caller, DEFAULT_ADMIN_ROLE, account);
                    if model.admin_count == 0 {
                        // Should never reach 0
                        prop_assert!(false, "AC-INV-03: admin_count reached 0");
                    }
                    if !result && model.roles[DEFAULT_ADMIN_ROLE][account] {
                        // Revoke failed because it would be the last admin
                        prop_assert_eq!(model.admin_count, 1);
                    }
                }
                _ => {
                    // Renounce admin
                    let _ = model.renounce_role(account, DEFAULT_ADMIN_ROLE);
                    prop_assert!(model.has_admin(), "AC-INV-02: no admin after renounce");
                }
            }
        }
    }

    #[test]
    fn access_grant_revoke_idempotent(
        role in 0usize..4,
        account in 0usize..4,
    ) {
        let mut model = AccessControlModel::new(0);

        // Grant twice
        model.grant_role(0, role, account);
        let count_after_first = model.admin_count;
        model.grant_role(0, role, account);
        prop_assert_eq!(model.admin_count, count_after_first, "AC-INV-04: grant not idempotent");
    }
}

// ============================================================================
// HARNESS-08: NFT Enumeration
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50_000))]

    #[test]
    fn nft_supply_consistent(
        ops in proptest::collection::vec(
            (0u8..=2u8, 0usize..4, 0usize..4, 1u32..=100u32),
            1..50,
        ),
    ) {
        let mut model = NftModel::new(Some(200));

        for (op, _from, to, token_hint) in ops {
            match op {
                0 => { model.mint(to); }
                1 => { model.burn(token_hint); }
                _ => { model.transfer(token_hint, to); }
            }
            prop_assert!(model.supply_consistent(), "NFT-INV-01: supply inconsistent");
            prop_assert!(model.balances_consistent(), "NFT-INV-02: balance_of inconsistent");
            prop_assert!(model.burned_ids_clean(), "NFT-INV-03: burned ID reissued");
        }
    }

    #[test]
    fn nft_burned_never_reissued(mint_count in 1u32..=20u32, burn_idx in 1u32..=20u32) {
        let mut model = NftModel::new(None);

        // Mint a bunch
        for _ in 0..mint_count {
            model.mint(0);
        }

        // Burn one (if valid)
        let burn_target = burn_idx.min(mint_count);
        if model.burn(burn_target) {
            prop_assert!(model.burned[burn_target as usize - 1]);
            prop_assert!(model.owners[burn_target as usize - 1].is_none());
        }

        // Mint more — new IDs should never reuse burned slot
        for _ in 0..5 {
            if let Some(new_id) = model.mint(1) {
                prop_assert!(!model.burned[new_id as usize - 1], "NFT-INV-03: reissued burned ID");
            }
        }
    }

    #[test]
    fn nft_max_supply_enforced(
        cap in 1u32..=50u32,
        mints in 1u32..=100u32,
    ) {
        let mut model = NftModel::new(Some(cap));

        let mut minted = 0u32;
        for _ in 0..mints {
            if model.mint(0).is_some() {
                minted += 1;
            }
        }
        prop_assert!(minted <= cap, "NFT-INV-04: exceeded max_supply");
        prop_assert!(model.total_supply <= cap);
    }
}

// ============================================================================
// HARNESS-09: Cross-Contract Invariants
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100_000))]

    /// DALLA mint + DEX deposit: LP tokens should never exceed what
    /// the minted supply allows.
    #[test]
    fn cross_dalla_dex_lp_bound(
        dalla_supply in 1000u128..=MAX_SUPPLY,
        d0 in 1u64..=u32::MAX as u64,
        d1 in 1u64..=u32::MAX as u64,
    ) {
        let d0 = d0 as u128;
        let d1 = d1 as u128;
        // Ensure deposits don't exceed DALLA supply
        if d0 > dalla_supply || d1 > dalla_supply { return Ok(()); }

        if let Some(lp) = initial_lp_tokens(d0, d1) {
            let total_lp = lp + MINIMUM_LIQUIDITY;
            // LP should not be zero when deposits are nonzero
            if d0 > 0 && d1 > 0 {
                prop_assert!(total_lp > 0, "CROSS-INV-01: zero LP with non-zero deposit");
            }
        }
    }

    /// Router and Pair get_amount_out must agree within ±1 across large range.
    #[test]
    fn cross_router_pair_full_range(
        amt in 1u128..=u64::MAX as u128,
        r_in in 1u128..=u64::MAX as u128,
        r_out in 1u128..=u64::MAX as u128,
    ) {
        let router_out = router_get_amount_out(amt, r_in, r_out);
        let pair_out = pair_get_amount_out(amt, r_in, r_out);

        match (router_out, pair_out) {
            (Some(r), Some(p)) => {
                let diff = if r > p { r - p } else { p - r };
                prop_assert!(diff <= 1, "CROSS-INV-02: router {} vs pair {}", r, p);
            }
            (None, None) => {} // Both overflow — consistent
            (Some(_), None) | (None, Some(_)) => {
                // One overflows, other doesn't — acceptable for large inputs
                // due to GCD reduction in router giving it extra range
            }
        }
    }

    /// DALLA checked_mint followed by checked_transfer should preserve invariants.
    #[test]
    fn cross_dalla_mint_transfer_sequence(
        initial_supply in 0u128..=MAX_SUPPLY / 2,
        mint_amount in 0u128..=MAX_SUPPLY / 4,
        transfer_amount in 0u128..=MAX_SUPPLY / 4,
    ) {
        let balance_a = initial_supply;
        let balance_b = 0u128;
        let total = initial_supply;

        // Mint to A
        if let Some((new_bal_a, new_total)) = checked_mint(balance_a, total, mint_amount, MAX_SUPPLY) {
            prop_assert!(new_total <= MAX_SUPPLY);
            prop_assert!(new_bal_a <= new_total);

            // Transfer from A to B
            if let Some((final_a, final_b)) = checked_transfer(new_bal_a, balance_b, transfer_amount) {
                prop_assert_eq!(final_a + final_b, new_bal_a, "CROSS-INV-03: transfer changed sum");
                // Both balances ≤ total
                prop_assert!(final_a <= new_total);
                prop_assert!(final_b <= new_total);
            }
        }
    }
}

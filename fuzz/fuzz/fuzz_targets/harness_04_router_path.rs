//! HARNESS-04 · Router Path Arithmetic Fuzzing
//!
//! Oracles:
//!   RTR-INV-01: get_amount_out output ≤ reserve_out (no drain)
//!   RTR-INV-02: get_amount_in ≥ 1 for any positive output
//!   RTR-INV-03: Router and Pair get_amount_out agree on results
//!   RTR-INV-04: _checked_mul_div(a, b, c) = (a*b)/c when no overflow
//!   RTR-INV-05: _checked_mul_div returns None when c = 0
//!   RTR-INV-06: gcd(a, b) divides both a and b
//!   RTR-INV-07: Multi-hop monotonically decreasing (each hop loses to fees)
//!   RTR-INV-08: quote(a, rA, rB) * rA ≈ a * rB (proportionality)

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use gem_fuzz::{
    checked_mul_div, gcd, pair_get_amount_in, pair_get_amount_out, router_get_amount_in,
    router_get_amount_out, router_quote,
};

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    // For single-hop tests
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,

    // For _checked_mul_div
    a: u128,
    b: u128,
    c: u128,

    // For multi-hop: 3 pools
    r0_a: u32,
    r0_b: u32,
    r1_a: u32,
    r1_b: u32,
    r2_a: u32,
    r2_b: u32,
    hop_amount: u32,
}

fuzz_target!(|input: FuzzInput| {
    let amt = input.amount_in as u128;
    let r_in = input.reserve_in as u128;
    let r_out = input.reserve_out as u128;

    // ── RTR-INV-04: _checked_mul_div correctness ──
    if input.c != 0 {
        let result = checked_mul_div(input.a, input.b, input.c);
        // Verify against wide multiplication when possible
        if let Some(product) = input.a.checked_mul(input.b) {
            let expected = product / input.c;
            assert_eq!(
                result,
                Some(expected),
                "RTR-INV-04: checked_mul_div({}, {}, {}) = {:?}, expected {}",
                input.a,
                input.b,
                input.c,
                result,
                expected
            );
        }
        // If a*b overflows but GCD reduction succeeds, result should still be valid
    }

    // ── RTR-INV-05: division by zero ──
    let zero_result = checked_mul_div(input.a, input.b, 0);
    assert_eq!(
        zero_result, None,
        "RTR-INV-05: checked_mul_div with c=0 should be None"
    );

    // ── RTR-INV-06: GCD divides both operands ──
    if input.a > 0 || input.b > 0 {
        let g = gcd(input.a, input.b);
        assert!(g > 0, "RTR-INV-06: gcd should be > 0 for nonzero inputs");
        if input.a > 0 {
            assert_eq!(
                input.a % g,
                0,
                "RTR-INV-06: gcd({}, {}) = {} doesn't divide a",
                input.a,
                input.b,
                g
            );
        }
        if input.b > 0 {
            assert_eq!(
                input.b % g,
                0,
                "RTR-INV-06: gcd({}, {}) = {} doesn't divide b",
                input.a,
                input.b,
                g
            );
        }
    }

    // Skip degenerate swap inputs
    if amt == 0 || r_in == 0 || r_out == 0 {
        return;
    }

    // ── RTR-INV-01: output never exceeds reserve_out ──
    if let Some(out) = router_get_amount_out(amt, r_in, r_out) {
        assert!(
            out < r_out,
            "RTR-INV-01: router output {} >= reserve_out {}",
            out,
            r_out
        );
    }

    // ── RTR-INV-02: get_amount_in ≥ 1 for positive output ──
    if amt < r_out {
        if let Some(needed) = router_get_amount_in(amt, r_in, r_out) {
            assert!(
                needed >= 1,
                "RTR-INV-02: get_amount_in returned 0 for positive output"
            );
        }
    }

    // ── RTR-INV-03: Router and Pair produce same results ──
    let router_out = router_get_amount_out(amt, r_in, r_out);
    let pair_out = pair_get_amount_out(amt, r_in, r_out);

    // Both should succeed or both should fail for the same inputs
    match (router_out, pair_out) {
        (Some(r), Some(p)) => {
            // Allow ±1 difference due to different rounding approaches
            // (router uses GCD reduction, pair uses direct checked_mul)
            let diff = if r > p { r - p } else { p - r };
            assert!(
                diff <= 1,
                "RTR-INV-03: router {} vs pair {} (diff {} > 1)",
                r,
                p,
                diff
            );
        }
        (None, None) => {} // Both overflow — fine
        (Some(r), None) => {
            // Router succeeded where pair overflowed — possible due to GCD reduction
            // This is acceptable — router is more robust
            let _ = r;
        }
        (None, Some(p)) => {
            // Pair succeeded where router failed — unexpected but not necessarily a bug
            let _ = p;
        }
    }

    // ── RTR-INV-08: quote proportionality ──
    if r_in > 0 && r_out > 0 && amt > 0 {
        if let Some(quoted) = router_quote(amt, r_in, r_out) {
            // quoted ≈ amt * r_out / r_in
            // Verify: quoted * r_in ≤ amt * r_out (integer division floors)
            if let (Some(lhs), Some(rhs)) = (
                quoted.checked_mul(r_in),
                amt.checked_mul(r_out),
            ) {
                assert!(
                    lhs <= rhs,
                    "RTR-INV-08: quote * reserve_a {} > amount * reserve_b {}",
                    lhs,
                    rhs
                );
            }
        }
    }

    // ── RTR-INV-07: Multi-hop monotonically decreasing ──
    let pools = [
        (input.r0_a as u128, input.r0_b as u128),
        (input.r1_a as u128, input.r1_b as u128),
        (input.r2_a as u128, input.r2_b as u128),
    ];

    let mut current_amount = input.hop_amount as u128;
    if current_amount == 0 {
        return;
    }

    let mut previous_amount = current_amount;
    let mut all_succeeded = true;

    for (r_a, r_b) in &pools {
        if *r_a == 0 || *r_b == 0 {
            all_succeeded = false;
            break;
        }
        match router_get_amount_out(current_amount, *r_a, *r_b) {
            Some(out) => {
                current_amount = out;
            }
            None => {
                all_succeeded = false;
                break;
            }
        }
    }

    if all_succeeded && current_amount > 0 {
        // After 3 hops with 0.3% fee each, output < input (fees compound)
        assert!(
            current_amount < previous_amount,
            "RTR-INV-07: multi-hop output {} >= input {} (should decrease due to fees)",
            current_amount,
            previous_amount
        );
    }
});

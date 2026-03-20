//! HARNESS-02 · Pair K-Invariant Fuzzing
//!
//! Oracles:
//!   K-INV-01: K_new ≥ K_old after every valid swap (256-bit check)
//!   K-INV-02: mul_u256(a, b) == a * b (verified against u128 when possible)
//!   K-INV-03: mul_u256 is commutative: mul(a,b) == mul(b,a)
//!   K-INV-04: mul_u256(a, 0) == (0, 0) and mul_u256(0, b) == (0, 0)
//!   K-INV-05: sqrt(x)² ≤ x < (sqrt(x)+1)²
//!   K-INV-06: get_amount_out never returns more than reserve_out
//!   K-INV-07: get_amount_in round-trip: out(in(x)) ≥ x

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use gem_fuzz::{
    k_invariant_holds, mul_u256, pair_get_amount_in, pair_get_amount_out, sqrt,
};

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    // For mul_u256 / sqrt
    a: u128,
    b: u128,

    // For swap simulation
    reserve0: u64,
    reserve1: u64,
    amount_in: u64,
    direction: bool, // true = token0→token1, false = token1→token0
}

fuzz_target!(|input: FuzzInput| {
    // ── K-INV-02: mul_u256 correctness against u128 when no overflow ──
    let (hi, lo) = mul_u256(input.a, input.b);
    if let Some(expected) = input.a.checked_mul(input.b) {
        assert_eq!(hi, 0, "K-INV-02: hi should be 0 for non-overflowing mul");
        assert_eq!(lo, expected, "K-INV-02: lo mismatch");
    } else {
        // Overflowed — hi must be nonzero (unless one operand is 0, which can't overflow)
        // Actually: if both nonzero and overflow, hi > 0. But edge case: a=1, b=MAX can't overflow.
        // If we're here, checked_mul returned None, so a*b > u128::MAX, meaning hi > 0.
        assert!(hi > 0, "K-INV-02: overflowing product must have hi > 0");
    }

    // ── K-INV-03: commutativity ──
    let (hi2, lo2) = mul_u256(input.b, input.a);
    assert_eq!(hi, hi2, "K-INV-03: commutativity hi");
    assert_eq!(lo, lo2, "K-INV-03: commutativity lo");

    // ── K-INV-04: zero identity ──
    let (z_hi, z_lo) = mul_u256(input.a, 0);
    assert_eq!(z_hi, 0, "K-INV-04: mul(a,0) hi");
    assert_eq!(z_lo, 0, "K-INV-04: mul(a,0) lo");
    let (z_hi2, z_lo2) = mul_u256(0, input.b);
    assert_eq!(z_hi2, 0, "K-INV-04: mul(0,b) hi");
    assert_eq!(z_lo2, 0, "K-INV-04: mul(0,b) lo");

    // ── K-INV-05: sqrt correctness ──
    // Test with both a and b (more coverage)
    for &val in &[input.a, input.b] {
        let s = sqrt(val);
        if val == 0 {
            assert_eq!(s, 0, "K-INV-05: sqrt(0) should be 0");
        } else {
            // s² ≤ val
            let s_sq = (s as u128).checked_mul(s as u128);
            if let Some(sq) = s_sq {
                assert!(sq <= val, "K-INV-05: sqrt({})² = {} > {}", val, sq, val);
            }
            // (s+1)² > val
            let sp1 = (s as u128).checked_add(1);
            if let Some(sp) = sp1 {
                if let Some(sp_sq) = sp.checked_mul(sp) {
                    assert!(
                        sp_sq > val,
                        "K-INV-05: (sqrt({})+1)² = {} ≤ {}",
                        val,
                        sp_sq,
                        val
                    );
                }
            }
        }
    }

    // ── Swap simulation for K-INV-01, K-INV-06, K-INV-07 ──
    let r0 = input.reserve0 as u128;
    let r1 = input.reserve1 as u128;
    let amt = input.amount_in as u128;

    // Skip degenerate cases
    if r0 == 0 || r1 == 0 || amt == 0 {
        return;
    }

    let (reserve_in, reserve_out) = if input.direction {
        (r0, r1)
    } else {
        (r1, r0)
    };

    // ── K-INV-06: output never exceeds reserve_out ──
    if let Some(amount_out) = pair_get_amount_out(amt, reserve_in, reserve_out) {
        assert!(
            amount_out < reserve_out,
            "K-INV-06: amount_out {} >= reserve_out {}",
            amount_out,
            reserve_out
        );

        // ── K-INV-01: K invariant holds after swap ──
        let (b0, b1) = if input.direction {
            (r0 + amt, r1 - amount_out)
        } else {
            (r0 - amount_out, r1 + amt)
        };

        let (a0_in, a1_in) = if input.direction {
            (amt, 0u128)
        } else {
            (0u128, amt)
        };

        assert!(
            k_invariant_holds(r0, r1, b0, b1, a0_in, a1_in),
            "K-INV-01: K decreased after swap! r0={}, r1={}, b0={}, b1={}, in0={}, in1={}",
            r0,
            r1,
            b0,
            b1,
            a0_in,
            a1_in
        );

        // ── K-INV-07: round-trip consistency ──
        // get_amount_in(amount_out) should return ≤ amt (since amt produces at least amount_out)
        if amount_out > 0 {
            if let Some(needed_in) = pair_get_amount_in(amount_out, reserve_in, reserve_out) {
                assert!(
                    needed_in <= amt + 1, // +1 for rounding
                    "K-INV-07: round-trip fail: need {} to get {} but provided {}",
                    needed_in,
                    amount_out,
                    amt
                );
            }
        }
    }
});

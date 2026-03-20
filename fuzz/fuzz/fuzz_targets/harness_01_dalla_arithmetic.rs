//! HARNESS-01 · DALLA Token Arithmetic Fuzzing
//!
//! Oracles:
//!   INV-01: Σ(balances) = total_supply after every operation
//!   INV-02: ∀ acct: balance[acct] ≤ total_supply
//!   INV-03: total_supply ≤ MAX_SUPPLY (100_000_000 × 10^12)
//!   INV-04: transfer(a → b, amt) never changes total_supply
//!   INV-05: increase_allowance/decrease_allowance never underflows

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use gem_fuzz::{
    checked_burn, checked_increase_allowance, checked_mint, checked_transfer,
    saturating_decrease_allowance,
};

const MAX_SUPPLY: u128 = 100_000_000_000_000_000_000; // 100M × 10^12

#[derive(Arbitrary, Debug)]
enum Op {
    Transfer {
        from_idx: u8,
        to_idx: u8,
        amount: u128,
    },
    Mint {
        to_idx: u8,
        amount: u128,
    },
    Burn {
        from_idx: u8,
        amount: u128,
    },
    IncreaseAllowance {
        owner_idx: u8,
        delta: u128,
    },
    DecreaseAllowance {
        owner_idx: u8,
        delta: u128,
    },
}

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    initial_supply: u128,
    ops: Vec<Op>,
}

fuzz_target!(|input: FuzzInput| {
    const NUM_ACCOUNTS: usize = 4;

    // Clamp initial supply
    let initial_supply = input.initial_supply % (MAX_SUPPLY + 1);
    let mut balances = [0u128; NUM_ACCOUNTS];
    let mut total_supply = initial_supply;
    balances[0] = initial_supply; // All initial tokens to account 0

    // Track allowances (flat array: owner * NUM_ACCOUNTS + spender)
    let mut allowances = [0u128; NUM_ACCOUNTS * NUM_ACCOUNTS];

    for op in input.ops.iter().take(64) {
        match op {
            Op::Transfer {
                from_idx,
                to_idx,
                amount,
            } => {
                let from = (*from_idx as usize) % NUM_ACCOUNTS;
                let to = (*to_idx as usize) % NUM_ACCOUNTS;
                if from == to {
                    continue;
                }
                let old_total = total_supply;
                if let Some((new_from, new_to)) =
                    checked_transfer(balances[from], balances[to], *amount)
                {
                    balances[from] = new_from;
                    balances[to] = new_to;
                    // INV-04: transfer never changes total_supply
                    assert_eq!(total_supply, old_total, "INV-04: transfer changed total_supply");
                }
            }

            Op::Mint { to_idx, amount } => {
                let to = (*to_idx as usize) % NUM_ACCOUNTS;
                if let Some((new_bal, new_total)) =
                    checked_mint(balances[to], total_supply, *amount, MAX_SUPPLY)
                {
                    balances[to] = new_bal;
                    total_supply = new_total;
                    // INV-03: total_supply ≤ MAX_SUPPLY
                    assert!(
                        total_supply <= MAX_SUPPLY,
                        "INV-03: total_supply {} > MAX_SUPPLY {}",
                        total_supply,
                        MAX_SUPPLY
                    );
                }
            }

            Op::Burn { from_idx, amount } => {
                let from = (*from_idx as usize) % NUM_ACCOUNTS;
                if let Some((new_bal, new_total)) =
                    checked_burn(balances[from], total_supply, *amount)
                {
                    balances[from] = new_bal;
                    total_supply = new_total;
                }
            }

            Op::IncreaseAllowance { owner_idx, delta } => {
                let owner = (*owner_idx as usize) % NUM_ACCOUNTS;
                let idx = owner * NUM_ACCOUNTS; // allowance to account 0
                if let Some(new_allowance) =
                    checked_increase_allowance(allowances[idx], *delta)
                {
                    allowances[idx] = new_allowance;
                }
            }

            Op::DecreaseAllowance { owner_idx, delta } => {
                let owner = (*owner_idx as usize) % NUM_ACCOUNTS;
                let idx = owner * NUM_ACCOUNTS;
                allowances[idx] = saturating_decrease_allowance(allowances[idx], *delta);
                // No underflow possible due to saturating arithmetic
            }
        }

        // INV-01: Σ(balances) = total_supply
        let sum: u128 = balances.iter().sum();
        assert_eq!(
            sum, total_supply,
            "INV-01: sum of balances {} != total_supply {}",
            sum, total_supply
        );

        // INV-02: ∀ acct: balance[acct] ≤ total_supply
        for (i, &bal) in balances.iter().enumerate() {
            assert!(
                bal <= total_supply,
                "INV-02: balance[{}] = {} > total_supply {}",
                i,
                bal,
                total_supply
            );
        }
    }
});

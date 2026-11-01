// SPDX-License-Identifier: AGPL-3.0-only
//! Literal operation boundaries retain successful charge prefixes and atomic publication.

use crate::testing::common::TestResult;
use crate::testing::half_pi_limits::{KINDS, verify};
use crate::testing::half_pi_work::{Operation, attempt, success};
use nrz_cas::limits::BudgetKind;

/// Costs and failed prefixes exclude 37 prior Work; staging includes 19 caller bytes.
/// Each cold and warm row is a separate complete-operation server measurement.
const COSTS: [(Operation, bool, [u64; 18], [u64; 18]); 4] = [
    // Check, cold: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Operation::Check,
        false,
        [
            39_913, 70, 14, 2, 17_971, 7_372, 735, 0, 0, 16, 32, 15, 32, 3, 1, 20, 0, 0,
        ],
        [
            39_909, 39_000, 9_149, 4_267, 34_898, 9_743, 39_909, 0, 0, 33_909, 9_743, 9_743, 9_743,
            0, 2_886, 9_159, 0, 0,
        ],
    ),
    // Check, warm: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Operation::Check,
        true,
        [
            39_913, 70, 14, 2, 18_099, 210, 735, 0, 0, 16, 0, 0, 0, 3, 1, 20, 0, 0,
        ],
        [
            39_909, 39_000, 9_149, 4_267, 34_898, 9_503, 39_909, 0, 0, 33_909, 0, 0, 0, 0, 2_886,
            9_159, 0, 0,
        ],
    ),
    // Guarded, cold: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Operation::Guarded,
        false,
        [
            99_382, 100, 66, 4, 31_523, 14_542, 1_172, 0, 0, 28, 72, 28, 72, 6, 2, 20, 0, 0,
        ],
        [
            99_378, 98_007, 44_584, 44_585, 89_601, 45_196, 99_378, 0, 0, 45_196, 45_196, 45_196,
            45_196, 0, 36_570, 42_226, 0, 0,
        ],
    ),
    // Guarded, warm: positive prefixes retain prior successful charges; zero prefixes are initial preflights.
    (
        Operation::Guarded,
        true,
        [
            99_382, 100, 66, 4, 31_651, 258, 1_172, 0, 0, 28, 0, 28, 72, 6, 2, 20, 0, 0,
        ],
        [
            99_378, 98_007, 44_584, 44_585, 89_601, 44_908, 99_378, 0, 0, 88_612, 0, 28_864,
            28_864, 0, 36_570, 42_226, 0, 0,
        ],
    ),
];

/// Every positive ceiling has a typed one-below refusal; zero ceilings execute successfully.
/// ´test:integration:half-pi-checking-boundaries´
#[test]
fn half_pi_checking_boundaries() -> TestResult {
    for (operation, warm, costs, prefixes) in COSTS {
        verify(
            &KINDS,
            |kind| success(operation, warm, kind, costs[0]),
            &costs,
            &prefixes,
            |kind, amount| attempt(operation, warm, kind, amount),
        )?;
    }
    Ok(())
}

/// Operation, warmth, complete Work, bit ceilings and failed Work prefixes.
type BitRow = (Operation, bool, u64, [u64; 2], [u64; 2]);

/// Bit refusals retain the literal successful prefix for each independently measured row.
const BITS: [BitRow; 4] = [
    (Operation::Check, false, 39913, [65, 66], [525, 8_034]),
    (Operation::Check, true, 39913, [65, 66], [525, 8_034]),
    (Operation::Guarded, false, 99_382, [2, 2], [27_001, 26_873]),
    (Operation::Guarded, true, 99_382, [2, 2], [27_001, 26_873]),
];

/// Concrete integer components and exact arithmetic refuse before oversized allocation.
/// ´test:integration:half-pi-checking-bits´
#[test]
fn half_pi_checking_bits() -> TestResult {
    let kinds = [BudgetKind::ComponentBits, BudgetKind::IntermediateBits];
    for (operation, warm, work, costs, prefixes) in BITS {
        verify(
            &kinds,
            |kind| success(operation, warm, kind, work),
            &costs,
            &prefixes,
            |kind, amount| attempt(operation, warm, kind, amount),
        )?;
    }
    Ok(())
}

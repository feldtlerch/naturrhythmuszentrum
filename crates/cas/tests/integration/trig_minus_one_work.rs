// SPDX-License-Identifier: AGPL-3.0-only
//! Literal complete-operation boundaries include prior Work and retained payload.

use crate::testing::common::TestResult;
use crate::testing::half_pi_limits::{KINDS, verify};
use crate::testing::trig_minus_one_work::{Operation, attempt};
use nrz_cas::limits::BudgetKind;

/// One complete operation's measured ceilings, successful Work and refusal prefixes.
type ResourceRow = (usize, Operation, [u64; 18], u64, u64, [u64; 18]);

/// Independently measured literal, negation, conditional, quotient and sum portable operations.
const ROWS: [ResourceRow; 10] = [
    (
        0,
        Operation::Export,
        [
            3_566, 0, 0, 0, 3_585, 0, 3_566, 3_566, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
        3_566,
        3_566,
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    (
        0,
        Operation::Replay,
        [
            814_202, 48, 460, 8, 112_642, 52_840, 3_566, 0, 814_202, 98, 280, 98, 280, 32, 6, 16,
            0, 0,
        ],
        814_202,
        789_482,
        [
            814_158, 613_311, 576_264, 576_265, 726_008, 576_729, 260, 0, 814_158, 576_729,
            576_729, 576_729, 576_729, 576_729, 444_770, 1_520, 0, 0,
        ],
    ),
    (
        1,
        Operation::Export,
        [
            3_494, 0, 0, 0, 3_513, 0, 3_494, 3_494, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
        3_494,
        3_494,
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    (
        1,
        Operation::Replay,
        [
            772_697, 48, 449, 8, 107_900, 53_199, 3_494, 0, 772_697, 98, 281, 98, 281, 32, 6, 21,
            0, 0,
        ],
        772_697,
        746_612,
        [
            772_653, 579_850, 539_363, 539_364, 686_490, 539_828, 260, 0, 772_653, 539_828,
            539_828, 539_828, 539_828, 539_828, 503_717, 93_055, 0, 0,
        ],
    ),
    (
        2,
        Operation::Export,
        [
            4_305, 0, 0, 0, 4_324, 0, 4_305, 4_305, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
        4_305,
        4_305,
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    (
        2,
        Operation::Replay,
        [
            1_097_973, 125, 724, 8, 135_794, 65_463, 4_305, 0, 1_097_973, 114, 345, 114, 345, 36,
            11, 21, 0, 0,
        ],
        1_097_973,
        1_048_623,
        [
            1_097_863, 845_834, 1_097_683, 789_669, 977_977, 790_467, 264, 0, 1_097_863, 790_467,
            790_467, 790_467, 790_467, 790_467, 630_787, 124_884, 0, 0,
        ],
    ),
    (
        3,
        Operation::Export,
        [
            4_148, 0, 0, 0, 4_167, 0, 4_148, 4_148, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
        4_148,
        4_148,
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    (
        3,
        Operation::Replay,
        [
            1_042_420, 78, 616, 8, 130_834, 63_980, 4_148, 0, 1_042_420, 114, 343, 114, 343, 36,
            11, 21, 0, 0,
        ],
        1_042_420,
        998_945,
        [
            1_042_376, 801_370, 750_914, 750_915, 928_568, 751_577, 260, 0, 1_042_376, 751_577,
            751_577, 751_577, 751_577, 751_577, 598_757, 118_592, 0, 0,
        ],
    ),
    (
        4,
        Operation::Export,
        [
            4_098, 0, 0, 0, 4_117, 0, 4_098, 4_098, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
        4_098,
        4_098,
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ),
    (
        4,
        Operation::Replay,
        [
            994_021, 68, 539, 8, 129_698, 61_266, 4_098, 0, 994_021, 112, 325, 112, 325, 36, 8, 21,
            0, 0,
        ],
        994_021,
        961_826,
        [
            993_977, 756_290, 708_528, 708_529, 881_638, 709_059, 260, 0, 993_977, 709_059,
            709_059, 709_059, 709_059, 709_059, 566_102, 115_398, 0, 0,
        ],
    ),
];

/// Export prepays the copy; production and replay retain their completed incremental charges.
/// Every refusal compares the complete graph snapshot; zero ceilings still execute.
/// ´test:integration:trig-minus-one-portable-resource-boundaries´
#[test]
fn trig_minus_one_portable_resource_boundaries() -> TestResult {
    for (input, operation, costs, work, name_work, prefixes) in ROWS {
        verify(
            &KINDS,
            |kind| {
                if kind == BudgetKind::NameBytes {
                    name_work
                } else {
                    work
                }
            },
            &costs,
            &prefixes,
            |resource, amount| attempt(input, operation, resource, amount),
        )?;
    }
    Ok(())
}

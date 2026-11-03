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
            775_117, 48, 460, 8, 112_642, 52_840, 3_566, 0, 775_117, 98, 280, 98, 280, 32, 6, 16,
            0, 0,
        ],
        775_117,
        750_397,
        [
            775_073, 574_226, 537_179, 537_180, 686_923, 537_644, 260, 0, 775_073, 537_644,
            537_644, 537_644, 537_644, 537_644, 417_050, 1_520, 0, 0,
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
            733_261, 48, 449, 8, 107_900, 53_199, 3_494, 0, 733_261, 98, 281, 98, 281, 32, 6, 21,
            0, 0,
        ],
        733_261,
        707_176,
        [
            733_217, 540_414, 499_927, 499_928, 647_054, 500_392, 260, 0, 733_217, 500_392,
            500_392, 500_392, 500_392, 500_392, 472_041, 93_055, 0, 0,
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
            1_048_492, 125, 724, 8, 135_794, 65_463, 4_305, 0, 1_048_492, 114, 345, 114, 345, 36,
            11, 21, 0, 0,
        ],
        1_048_492,
        999_142,
        [
            1_048_382, 796_353, 1_048_202, 740_188, 928_496, 740_986, 264, 0, 1_048_382, 740_986,
            740_986, 740_986, 740_986, 740_986, 595_390, 124_884, 0, 0,
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
            995_164, 78, 616, 8, 130_834, 63_980, 4_148, 0, 995_164, 114, 343, 114, 343, 36, 11,
            21, 0, 0,
        ],
        995_164,
        951_689,
        [
            995_120, 754_114, 703_658, 703_659, 881_312, 704_321, 260, 0, 995_120, 704_321,
            704_321, 704_321, 704_321, 704_321, 564_885, 118_592, 0, 0,
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
            948_945, 68, 539, 8, 129_698, 61_266, 4_098, 0, 948_945, 112, 325, 112, 325, 36, 8, 21,
            0, 0,
        ],
        948_945,
        916_750,
        [
            948_901, 711_214, 663_452, 663_453, 836_562, 663_983, 260, 0, 948_901, 663_983,
            663_983, 663_983, 663_983, 663_983, 533_710, 115_398, 0, 0,
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

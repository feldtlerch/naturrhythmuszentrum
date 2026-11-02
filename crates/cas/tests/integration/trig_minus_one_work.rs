// SPDX-License-Identifier: AGPL-3.0-only
//! Literal complete-operation boundaries include prior Work and retained payload.

use crate::testing::common::TestResult;
use crate::testing::half_pi_limits::{KINDS, verify};
use crate::testing::trig_minus_one_work::{Operation, attempt};
use nrz_cas::limits::BudgetKind;

/// Export prepays the copy; production and replay retain their completed incremental charges.
/// Every refusal compares the complete graph snapshot; zero ceilings still execute.
/// ´test:integration:trig-minus-one-portable-resource-boundaries´
#[test]
fn trig_minus_one_portable_resource_boundaries() -> TestResult {
    for (input, operation, costs, work, name_work, prefixes) in [
        (
            0,
            Operation::Export,
            [
                3566, 0, 0, 0, 3585, 0, 3566, 3566, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            3566,
            3566,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ),
        (
            0,
            Operation::Replay,
            [
                775_117, 48, 460, 8, 112_642, 52840, 3566, 0, 775_117, 98, 280, 98, 280, 32, 6, 16,
                0, 0,
            ],
            775_117,
            750_397,
            [
                775_073, 574_226, 537_179, 537_180, 686_923, 537_644, 260, 0, 775_073, 537_644,
                537_644, 537_644, 537_644, 537_644, 417_050, 1520, 0, 0,
            ],
        ),
        (
            1,
            Operation::Export,
            [
                3494, 0, 0, 0, 3513, 0, 3494, 3494, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            3494,
            3494,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ),
        (
            1,
            Operation::Replay,
            [
                733_261, 48, 449, 8, 107_900, 53199, 3494, 0, 733_261, 98, 281, 98, 281, 32, 6, 21,
                0, 0,
            ],
            733_261,
            707_176,
            [
                733_217, 540_414, 499_927, 499_928, 647_054, 500_392, 260, 0, 733_217, 500_392,
                500_392, 500_392, 500_392, 500_392, 472_041, 93055, 0, 0,
            ],
        ),
    ] {
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

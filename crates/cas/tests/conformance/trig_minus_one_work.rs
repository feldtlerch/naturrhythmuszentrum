// SPDX-License-Identifier: AGPL-3.0-only
//! Literal complete-operation boundaries include prior Work and retained payload.

use crate::testing::common::TestResult;
use crate::testing::half_pi_limits::{KINDS, verify};
use crate::testing::trig_minus_one_work::{Operation, attempt};
use nrz_cas::limits::BudgetKind;

/// Export prepays the copy; production and replay retain their completed incremental charges.
/// Every refusal compares the complete graph snapshot; zero ceilings still execute.
/// ´test:integration:trig-minus-one-checking-resource-boundaries´
#[test]
fn trig_minus_one_checking_resource_boundaries() -> TestResult {
    for (input, operation, costs, work, name_work, prefixes) in [
        (
            0,
            Operation::Produce,
            [
                718_356, 38, 465, 8, 112_770, 51913, 3566, 0, 0, 96, 275, 96, 275, 32, 6, 16, 0, 0,
            ],
            718_356,
            693_876,
            [
                718_352, 518_211, 621_449, 484_661, 630_254, 485_125, 718_352, 0, 0, 485_125,
                485_125, 485_125, 485_125, 485_125, 401_514, 196_154, 0, 0,
            ],
        ),
        (
            1,
            Operation::Produce,
            [
                656_755, 37, 451, 8, 108_028, 52272, 3494, 0, 0, 96, 276, 96, 276, 32, 6, 21, 0, 0,
            ],
            656_755,
            630_905,
            [
                656_751, 464_689, 563_165, 427_629, 570_640, 428_093, 656_751, 0, 0, 428_093,
                428_093, 428_093, 428_093, 428_093, 347_254, 142_802, 0, 0,
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

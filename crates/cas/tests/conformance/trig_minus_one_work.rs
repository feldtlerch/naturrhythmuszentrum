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
                718_355, 38, 465, 8, 112_770, 51_913, 3_566, 0, 0, 96, 275, 96, 275, 32, 6, 16, 0,
                0,
            ],
            718_355,
            693_875,
            [
                718_351, 518_210, 621_448, 484_660, 630_253, 485_124, 718_351, 0, 0, 485_124,
                485_124, 485_124, 485_124, 485_124, 401_513, 196_153, 0, 0,
            ],
        ),
        (
            1,
            Operation::Produce,
            [
                656_753, 37, 451, 8, 108_028, 52_272, 3_494, 0, 0, 96, 276, 96, 276, 32, 6, 21, 0,
                0,
            ],
            656_753,
            630_903,
            [
                656_749, 464_687, 563_163, 427_627, 570_638, 428_091, 656_749, 0, 0, 428_091,
                428_091, 428_091, 428_091, 428_091, 347_252, 142_800, 0, 0,
            ],
        ),
        (
            2,
            Operation::Produce,
            [
                936_601, 38, 720, 8, 135_922, 64_856, 4_305, 0, 0, 113, 342, 113, 342, 36, 11, 21,
                0, 0,
            ],
            936_601,
            891_011,
            [
                936_597, 685_773, 806_863, 635_721, 816_998, 636_519, 936_597, 0, 0, 636_519,
                636_519, 636_519, 636_519, 636_519, 523_946, 263_242, 0, 0,
            ],
        ),
        (
            3,
            Operation::Produce,
            [
                905_614, 39, 617, 8, 130_962, 62_984, 4_148, 0, 0, 112, 338, 112, 338, 36, 11, 21,
                0, 0,
            ],
            905_614,
            862_844,
            [
                905_610, 665_481, 781_719, 619_047, 791_854, 619_709, 905_610, 0, 0, 619_709,
                619_709, 619_709, 619_709, 619_709, 513_000, 263_028, 0, 0,
            ],
        ),
        (
            4,
            Operation::Produce,
            [
                864_544, 38, 545, 8, 129_826, 60_339, 4_098, 0, 0, 110, 320, 110, 320, 36, 8, 21,
                0, 0,
            ],
            864_544,
            832_584,
            [
                864_540, 627_594, 742_118, 583_819, 752_253, 584_349, 864_540, 0, 0, 584_349,
                584_349, 584_349, 584_349, 584_349, 487_254, 262_919, 0, 0,
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

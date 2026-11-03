// SPDX-License-Identifier: AGPL-3.0-only
//! Literal complete-operation ceilings and consumed prefixes with independent prior usage.

use crate::testing::common::TestResult;
use crate::testing::series_boundaries::{KINDS, attempt};

/// Totals include 37 prior Work and 113 retained caller bytes. Work charges prepay atomically.
/// Other refusals consume the literal prefix; component ceiling zero is the existing policy fault.
/// Modes are checking, absent-output production, retained production, export and fresh replay.
/// ´test:integration:series-checking-resource-boundaries´
#[test]
fn series_checking_resource_boundaries() -> TestResult {
    for (mode, kind, ceilings, prefixes) in [
        (
            0,
            0,
            [
                35_317, 17_265, 7262, 16, 32, 16, 32, 731, 705, 1, 33, 1, 2, 2, 3, 1, 0, 2,
            ],
            [
                35_313, 33_250, 3873, 3873, 3635, 3873, 3635, 35_313, 3635, 2772, 32_060, 2772,
                30_853, 2254, 2254, 37, 0, 30_853,
            ],
        ),
        (
            0,
            1,
            [
                62_627, 25_585, 9890, 20, 48, 20, 48, 1034, 1076, 1, 262, 1, 3, 2, 9, 1, 0, 4,
            ],
            [
                62_623, 59_611, 12_018, 12_018, 11_554, 12_018, 11_554, 62_623, 11_554, 9934,
                58_421, 9934, 96, 5606, 9709, 37, 0, 6695,
            ],
        ),
        (
            0,
            2,
            [
                104_559, 32_795, 11_584, 24, 53, 24, 53, 1242, 1114, 2, 307, 28, 2, 2, 6, 2, 0, 3,
            ],
            [
                104_555, 100_265, 45_085, 45_085, 44_617, 45_085, 44_617, 104_555, 44_617, 42_965,
                99_075, 42_965, 49, 37_246, 42_580, 37, 0, 38_608,
            ],
        ),
        (
            0,
            3,
            [
                102_905, 30_289, 11_035, 21, 50, 21, 50, 1307, 1745, 1, 713, 1, 5, 2, 15, 1, 0, 5,
            ],
            [
                102_901, 99_060, 27_877, 27_877, 26_967, 27_877, 26_967, 102_901, 26_967, 23_975,
                97_870, 23_975, 1107, 11_738, 23_750, 37, 0, 1107,
            ],
        ),
        (
            0,
            4,
            [
                99_720, 33_361, 11_713, 24, 58, 23, 58, 1395, 2047, 2, 419, 3, 4, 2, 12, 2, 0, 5,
            ],
            [
                99_716, 95_621, 21_383, 94_414, 20_785, 21_383, 20_785, 99_716, 20_785, 18_769,
                94_431, 18_769, 516, 6225, 18_544, 37, 0, 17_520,
            ],
        ),
        (
            0,
            5,
            [
                34_091, 15_665, 6690, 15, 27, 15, 27, 703, 726, 1, 33, 1, 2, 2, 3, 1, 0, 2,
            ],
            [
                34_087, 32_119, 3914, 3914, 3662, 3914, 3662, 34_087, 3662, 2779, 30_929, 2779,
                26_164, 2270, 2270, 37, 0, 26_164,
            ],
        ),
        (
            0,
            6,
            [
                34_091, 15_665, 6690, 15, 27, 15, 27, 703, 726, 1, 33, 1, 2, 2, 3, 1, 0, 2,
            ],
            [
                34_087, 32_119, 3914, 3914, 3662, 3914, 3662, 34_087, 3662, 2779, 30_929, 2779, 44,
                2270, 2270, 37, 0, 44,
            ],
        ),
    ] {
        for ((dimension, ceiling), prefix) in KINDS.into_iter().zip(ceilings).zip(prefixes) {
            assert_eq!(
                attempt(mode, kind, dimension, ceiling)?,
                (true, ceilings[0]),
                "{mode}/{kind} {dimension:?}"
            );
            if ceiling > 0 {
                assert_eq!(
                    attempt(mode, kind, dimension, ceiling - 1)?,
                    (false, prefix),
                    "{mode}/{kind} {dimension:?}"
                );
            }
        }
    }
    Ok(())
}

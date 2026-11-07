// SPDX-License-Identifier: AGPL-3.0-only
//! Literal server measurements on seed 579d171; no production limit or older pin changes.

use super::fixture::Pin;
use nrz_scalar::{Precision, Scalar};

/// Native f32/f64 followed by original-block f32/f64, each negative/zero/undecided/positive.
const PINS: [[Pin; 4]; 4] = [
    [
        Pin {
            work: [309_883, 65, 34, 42],
            bytes: [57_976, 33_792, 33_792, 1_168],
        },
        Pin {
            work: [201_137, 65, 34, 34],
            bytes: [56_068, 17_132, 17_132, 1_168],
        },
        Pin {
            work: [55_169, 65, 34, 34],
            bytes: [30_343, 4_060, 4_060, 1_168],
        },
        Pin {
            work: [307_873, 65, 34, 42],
            bytes: [57_976, 33_792, 33_792, 1_168],
        },
    ],
    [
        Pin {
            work: [309_891, 65, 34, 42],
            bytes: [57_984, 33_808, 33_808, 1_176],
        },
        Pin {
            work: [201_145, 65, 34, 34],
            bytes: [56_076, 17_148, 17_148, 1_176],
        },
        Pin {
            work: [55_177, 65, 34, 34],
            bytes: [30_351, 4_076, 4_076, 1_176],
        },
        Pin {
            work: [307_881, 65, 34, 42],
            bytes: [57_984, 33_808, 33_808, 1_176],
        },
    ],
    [
        Pin {
            work: [387_264, 39_172, 39_141, 39_149],
            bytes: [57_008, 32_824, 32_824, 200],
        },
        Pin {
            work: [278_434, 39_130, 39_099, 39_099],
            bytes: [55_100, 16_164, 16_164, 200],
        },
        Pin {
            work: [132_466, 39_130, 39_099, 39_099],
            bytes: [29_375, 3_092, 3_092, 200],
        },
        Pin {
            work: [385_254, 39_172, 39_141, 39_149],
            bytes: [57_008, 32_824, 32_824, 200],
        },
    ],
    [
        Pin {
            work: [387_312, 39_172, 39_141, 39_149],
            bytes: [57_056, 32_880, 32_880, 248],
        },
        Pin {
            work: [278_482, 39_130, 39_099, 39_099],
            bytes: [55_148, 16_220, 16_220, 248],
        },
        Pin {
            work: [132_514, 39_130, 39_099, 39_099],
            bytes: [29_423, 3_148, 3_148, 248],
        },
        Pin {
            work: [385_302, 39_172, 39_141, 39_149],
            bytes: [57_056, 32_880, 32_880, 248],
        },
    ],
];

/// The format and original source route select literal evidence, never a raised policy.
pub(super) const fn pins<P: Scalar>(block: bool) -> [Pin; 4] {
    let format = match P::PRECISION {
        Precision::F32 => 0,
        Precision::F64 => 1,
    };
    PINS[format + if block { 2 } else { 0 }]
}

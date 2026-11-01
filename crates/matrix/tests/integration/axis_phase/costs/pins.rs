// SPDX-License-Identifier: AGPL-3.0-only
//! Literal server measurements on seed 579d171; no production limit or older pin changes.

use super::fixture::Pin;
use nrz_scalar::{Precision, Scalar};

/// Native f32/f64 followed by original-block f32/f64, each negative/zero/undecided/positive.
const PINS: [[Pin; 4]; 4] = [
    [
        Pin {
            work: [305_544, 65, 34, 42],
            bytes: [57_976, 33_792, 33_792, 1_168],
        },
        Pin {
            work: [200_326, 65, 34, 34],
            bytes: [55_368, 17_132, 17_132, 1_168],
        },
        Pin {
            work: [55_169, 65, 34, 34],
            bytes: [30_343, 4_060, 4_060, 1_168],
        },
        Pin {
            work: [303_534, 65, 34, 42],
            bytes: [57_976, 33_792, 33_792, 1_168],
        },
    ],
    [
        Pin {
            work: [305_552, 65, 34, 42],
            bytes: [57_984, 33_808, 33_808, 1_176],
        },
        Pin {
            work: [200_334, 65, 34, 34],
            bytes: [55_376, 17_148, 17_148, 1_176],
        },
        Pin {
            work: [55_177, 65, 34, 34],
            bytes: [30_351, 4_076, 4_076, 1_176],
        },
        Pin {
            work: [303_542, 65, 34, 42],
            bytes: [57_984, 33_808, 33_808, 1_176],
        },
    ],
    [
        Pin {
            work: [382_925, 39_172, 39_141, 39_149],
            bytes: [57_008, 32_824, 32_824, 200],
        },
        Pin {
            work: [277_623, 39_130, 39_099, 39_099],
            bytes: [54_400, 16_164, 16_164, 200],
        },
        Pin {
            work: [132_466, 39_130, 39_099, 39_099],
            bytes: [29_375, 3_092, 3_092, 200],
        },
        Pin {
            work: [380_915, 39_172, 39_141, 39_149],
            bytes: [57_008, 32_824, 32_824, 200],
        },
    ],
    [
        Pin {
            work: [382_973, 39_172, 39_141, 39_149],
            bytes: [57_056, 32_880, 32_880, 248],
        },
        Pin {
            work: [277_671, 39_130, 39_099, 39_099],
            bytes: [54_448, 16_220, 16_220, 248],
        },
        Pin {
            work: [132_514, 39_130, 39_099, 39_099],
            bytes: [29_423, 3_148, 3_148, 248],
        },
        Pin {
            work: [380_963, 39_172, 39_141, 39_149],
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

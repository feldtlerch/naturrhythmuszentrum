// SPDX-License-Identifier: AGPL-3.0-only
//! Measured complete object admission with independent active ceilings.

use crate::testing::common::TestResult;
use crate::testing::series_work::{KINDS, attempt};
/// Literal totals include 37 prior Work and 113 independently retained logical bytes.
/// A failed Work charge prepays atomically; other ceilings retain the stated consumed prefix.
/// ´test:integration:series-object-resource-boundaries´
#[test]
fn series_object_resource_boundaries() -> TestResult {
    for (warm, ceilings, prefixes) in [
        (
            false,
            [1498, 6684, 5170, 12, 20, 12, 20, 2, 2, 3, 0],
            [1420, 1498, 1498, 1498, 1498, 1498, 1498, 1335, 37, 37, 0],
        ),
        (
            true,
            [1498, 6748, 83, 0, 7, 12, 20, 2, 2, 3, 0],
            [1420, 120, 112, 0, 1343, 120, 120, 1335, 37, 37, 0],
        ),
    ] {
        for ((kind, ceiling), prefix) in KINDS.into_iter().zip(ceilings).zip(prefixes) {
            assert_eq!(
                attempt(warm, kind, ceiling)?,
                (true, 1498),
                "{warm} {kind:?}"
            );
            if ceiling > 0 {
                assert_eq!(
                    attempt(warm, kind, ceiling - 1)?,
                    (false, prefix),
                    "{warm} {kind:?}"
                );
            }
        }
    }
    Ok(())
}

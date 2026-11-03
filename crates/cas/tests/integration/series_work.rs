// SPDX-License-Identifier: AGPL-3.0-only
//! Complete checking, production, retention and portable resource contracts.

use crate::testing::common::TestResult;

/// Complete-operation discovery is separate from the literal enforcement rows.
/// ´test:integration:series-complete-measurements´
#[test]
#[ignore = "explicit complete-operation measurement"]
#[expect(
    clippy::print_stdout,
    reason = "Explicit ignored discovery emits complete measured rows for literal boundary tests"
)]
fn series_complete_measurements() -> TestResult {
    for mode in 0..5 {
        for kind in 0..7 {
            let (ceilings, prefixes) = crate::testing::series_boundaries::measure(mode, kind)?;
            println!("SERIES_PIN ({mode}, {kind}, {ceilings:?}, {prefixes:?}),");
        }
    }
    Ok(())
}

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
        for kind in 0..13 {
            let (ceilings, prefixes) = crate::testing::series_boundaries::measure(mode, kind)?;
            println!("SERIES_PIN ({mode}, {kind}, {ceilings:?}, {prefixes:?}),");
        }
    }
    Ok(())
}

/// Admission of the enlarged hand integral is measured independently of its producer.
/// ´test:integration:series-calculus-admission-measurements´
#[test]
#[ignore = "explicit complete-operation measurement"]
#[expect(
    clippy::print_stdout,
    reason = "Explicit discovery emits complete measured admission rows"
)]
fn series_calculus_admission_measurements() -> TestResult {
    use crate::testing::common::default_budget;
    use crate::testing::series_work::{KINDS, attempt_values};
    use nrz_cas::limits::BudgetKind;
    for warm in [false, true] {
        let mut ceilings = Vec::new();
        let mut prefixes = Vec::new();
        for kind in KINDS {
            let mut low = if kind == BudgetKind::Work { 37 } else { 0 };
            let mut high = default_budget()?.limits().get(kind);
            while low < high {
                let middle = low + (high - low) / 2;
                if attempt_values(warm, kind, middle, &["-1/2", "1", "-1", "1"])?.0 {
                    high = middle;
                } else {
                    low = middle + 1;
                }
            }
            ceilings.push(low);
            prefixes.push(if low > 0 {
                attempt_values(warm, kind, low - 1, &["-1/2", "1", "-1", "1"])?.1
            } else {
                0
            });
        }
        println!("CALCULUS_ADMISSION {warm} {ceilings:?} {prefixes:?}");
    }
    Ok(())
}

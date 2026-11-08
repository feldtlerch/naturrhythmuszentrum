// SPDX-License-Identifier: AGPL-3.0-only
//! Consumer boundaries preserve the original ledger and disabled observation path.

use crate::budget::{Budget, Limits};
use crate::error::{Fault, Location};
use crate::testing::common::TestResult;
use crate::testing::work_trace::{observed, without_observation};
use crate::work_observation::observe;

/// Consumer and CAS intervals reconcile on one ledger, with no observer tariff.
/// ´test:crate:work-cross-crate-boundary-receipts´
#[test]
fn work_cross_crate_boundary_receipts() -> TestResult {
    let (result, report) = observed(16, || -> Result<(), Fault> {
        let mut budget = Budget::new(Limits::default())?;
        observe!(budget, Location::new("cas parent"), {
            budget.work(2, Location::new("parent charge"))?;
            budget.with_work_phase("matrix", "consumer", |budget| {
                budget.work(3, Location::new("consumer charge"))
            })
        })?;
        assert_eq!(budget.usage().work, 5);
        Ok(())
    });
    result?;
    assert!(report.complete);
    assert_eq!(report.ledgers.len(), 1);
    let ledger = report.ledgers.values().next().ok_or("missing ledger")?;
    assert_eq!((ledger.classified, ledger.remainder), (5, 0));
    let receipts: Vec<_> = report
        .receipts
        .values()
        .map(|receipt| (receipt.inclusive, receipt.exclusive))
        .collect();
    assert_eq!(receipts, [(5, 2), (3, 3)]);
    without_observation(|| -> TestResult {
        let mut budget = Budget::new(Limits::default())?;
        budget.with_work_phase("matrix", "consumer", |budget| {
            budget.work(3, Location::new("consumer charge"))
        })?;
        assert_eq!(budget.usage().work, 3);
        assert!(!budget.observation.assigned());
        assert_eq!(budget.observation.cold_entries(), 0);
        Ok(())
    })
}

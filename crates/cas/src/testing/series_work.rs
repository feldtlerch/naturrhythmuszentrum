// SPDX-License-Identifier: AGPL-3.0-only
//! Complete object admissions retain prior usage and compare full rollback snapshots.

use super::common::{TestResult, default_budget};
use super::half_pi_limits::ceiling;
use super::series::{admit, fixture};
use crate::test_api::limits::{Budget, BudgetKind};
use crate::test_api::syntax::Draft;
use crate::test_api::{FaultKind, Location};

pub(crate) const KINDS: [BudgetKind; 11] = [
    BudgetKind::Work,
    BudgetKind::StagedPayloadBytes,
    BudgetKind::PayloadBytes,
    BudgetKind::Vertices,
    BudgetKind::Edges,
    BudgetKind::StagedVertices,
    BudgetKind::StagedEdges,
    BudgetKind::ComponentBits,
    BudgetKind::JetOrder,
    BudgetKind::JetEntries,
    BudgetKind::Depth,
];

/// Each attempt starts with identical committed syntax and an independent active allowance.
pub(crate) fn attempt(warm: bool, kind: BudgetKind, amount: u64) -> TestResult<(bool, u64)> {
    attempt_values(warm, kind, amount, &["1", "2", "0"])
}

/// The same complete admission control covers the enlarged order-four integral output.
pub(crate) fn attempt_values(
    warm: bool,
    kind: BudgetKind,
    amount: u64,
    values: &[&str],
) -> TestResult<(bool, u64)> {
    let (mut f, value) = fixture(values)?;
    if warm {
        admit(&mut f, value.clone())?;
    }
    let snapshot = f.cas.canonical_snapshot(&mut default_budget()?)?;
    let mut limits = ceiling(kind, amount);
    match kind {
        BudgetKind::JetOrder => limits.jet_order = amount,
        BudgetKind::JetEntries => limits.jet_entries = amount,
        _ => {}
    }
    let mut budget = Budget::new(limits)?;
    let at = Location::new("formal object boundary fixture");
    budget.work(37, at)?;
    let result = budget.with_retained_payload(113, at, |budget| {
        f.cas.admit(&[Draft::SeriesObject(value)], budget)
    });
    match result {
        Ok(_) => Ok((true, budget.usage().work)),
        Err(fault) => {
            assert!(
                matches!(fault.kind, FaultKind::Exhausted { budget: actual, .. } if actual == kind),
                "{fault:?}"
            );
            assert_eq!(f.cas.canonical_snapshot(&mut default_budget()?)?, snapshot);
            Ok((false, budget.usage().work))
        }
    }
}

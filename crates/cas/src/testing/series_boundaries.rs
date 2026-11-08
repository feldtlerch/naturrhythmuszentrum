// SPDX-License-Identifier: AGPL-3.0-only
//! Complete finite operations under independently restricted active resource dimensions.

use super::common::{TestResult, default_budget};
use super::half_pi_limits::ceiling;
use super::series_proof::{fixture, production};
use crate::test_api::limits::{Budget, BudgetKind};
use crate::test_api::{FaultKind, Location};

/// The same dimensions cover arithmetic, graph publication and portable checking.
pub(crate) const KINDS: [BudgetKind; 18] = [
    BudgetKind::Work,
    BudgetKind::StagedPayloadBytes,
    BudgetKind::PayloadBytes,
    BudgetKind::Vertices,
    BudgetKind::Edges,
    BudgetKind::StagedVertices,
    BudgetKind::StagedEdges,
    BudgetKind::WitnessBytes,
    BudgetKind::OutputBytes,
    BudgetKind::Depth,
    BudgetKind::ArithmeticSteps,
    BudgetKind::ConditionSteps,
    BudgetKind::ComponentBits,
    BudgetKind::JetOrder,
    BudgetKind::JetEntries,
    BudgetKind::ProofNodes,
    BudgetKind::ReplayWork,
    BudgetKind::IntermediateBits,
];

/// Preserve the established compact measurement interface.
pub(crate) fn attempt(
    mode: usize,
    kind: usize,
    dimension: BudgetKind,
    amount: u64,
) -> TestResult<(bool, u64)> {
    let (success, work, _) = attempt_detail(mode, kind, dimension, amount)?;
    Ok((success, work))
}

/// Every retry reconstructs the exact original store, prior usage and reuse state.
#[expect(
    clippy::too_many_lines,
    reason = "Each complete resource attempt retains its setup, call and rollback assertion together"
)]
pub(crate) fn attempt_detail(
    mode: usize,
    kind: usize,
    dimension: BudgetKind,
    amount: u64,
) -> TestResult<(bool, u64, Option<crate::test_api::Fault>)> {
    let mut limits = ceiling(dimension, amount);
    match dimension {
        BudgetKind::JetOrder => limits.jet_order = amount,
        BudgetKind::JetEntries => limits.jet_entries = amount,
        _ => {}
    }
    if matches!(
        dimension,
        BudgetKind::ComponentBits | BudgetKind::IntermediateBits
    ) && amount == 0
    {
        assert_eq!(
            Budget::new(limits)
                .expect_err("positive component policy")
                .kind,
            FaultKind::Invalid(crate::test_api::InvalidCode::Policy)
        );
        return Ok((false, 0, None));
    }
    let mut budget = Budget::new(limits)?;
    let at = Location::new("formal series complete boundary");
    budget.work(37, at)?;
    let (result, unchanged) = if mode == 1 || mode == 2 {
        let mut f = production(kind)?;
        if mode == 2 {
            f.base
                .cas
                .compute_series(&f.operation, &f.proofs, &mut default_budget()?)?;
        }
        let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
        let result = budget.with_retained_payload(113, at, |budget| {
            f.base
                .cas
                .compute_series(&f.operation, &f.proofs, budget)
                .map(|_| ())
        });
        let unchanged = f.base.cas.canonical_snapshot(&mut default_budget()?)? == before;
        (result, unchanged)
    } else {
        let mut f = fixture(kind, false)?;
        if mode == 0 {
            let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
            let result = budget.with_retained_payload(113, at, |budget| {
                f.base
                    .cas
                    .check_proof(&f.base.context, &f.steps, budget)
                    .map(|_| ())
            });
            (
                result,
                f.base.cas.canonical_snapshot(&mut default_budget()?)? == before,
            )
        } else {
            let witness =
                f.base
                    .cas
                    .check_proof(&f.base.context, &f.steps, &mut default_budget()?)?;
            if mode == 3 {
                let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
                let result = budget.with_retained_payload(113, at, |budget| {
                    f.base.cas.export_witness(&witness, budget).map(|_| ())
                });
                (
                    result,
                    f.base.cas.canonical_snapshot(&mut default_budget()?)? == before,
                )
            } else {
                let bytes = f
                    .base
                    .cas
                    .export_witness(&witness, &mut default_budget()?)?;
                let mut receiver = fixture(kind, true)?;
                let before = receiver
                    .base
                    .cas
                    .canonical_snapshot(&mut default_budget()?)?;
                let result = budget.with_retained_payload(113, at, |budget| {
                    receiver
                        .base
                        .cas
                        .replay(&bytes, &receiver.expected, budget)
                        .map(|_| ())
                });
                (
                    result,
                    receiver
                        .base
                        .cas
                        .canonical_snapshot(&mut default_budget()?)?
                        == before,
                )
            }
        }
    };
    match result {
        Ok(()) => Ok((true, budget.usage().work, None)),
        Err(fault) => {
            assert!(
                matches!(fault.kind,FaultKind::Exhausted {budget:actual,..} if actual==dimension || (dimension==BudgetKind::IntermediateBits && actual==BudgetKind::ComponentBits)),
                "mode {mode} kind {kind} {dimension:?} {amount}: {fault:?}"
            );
            assert!(unchanged, "complete refusal rollback");
            Ok((false, budget.usage().work, Some(fault)))
        }
    }
}

/// Development measurements search successful complete operations, never a first failed charge.
pub(crate) fn measure(mode: usize, kind: usize) -> TestResult<([u64; 18], [u64; 18])> {
    let mut ceilings = [0; 18];
    let mut prefixes = [0; 18];
    for (index, dimension) in KINDS.into_iter().enumerate() {
        let mut low = if dimension == BudgetKind::Work { 37 } else { 0 };
        let mut high = default_budget()?.limits().get(dimension);
        assert!(attempt(mode, kind, dimension, high)?.0);
        while low < high {
            let middle = low + (high - low) / 2;
            if attempt(mode, kind, dimension, middle)?.0 {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        ceilings[index] = low;
        if low > 0 {
            prefixes[index] = attempt(mode, kind, dimension, low - 1)?.1;
        }
    }
    Ok((ceilings, prefixes))
}

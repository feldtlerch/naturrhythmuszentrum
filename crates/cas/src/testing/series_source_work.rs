// SPDX-License-Identifier: AGPL-3.0-only
//! Complete source operations retain caller spend, simultaneous buffers and exact rollback.

use super::common::{TestResult, default_budget};
use super::series_boundaries::KINDS;
use super::series_source::{Fixture, lifted, setup, statement};
use crate::test_api::limits::{Budget, BudgetKind};
use crate::test_api::{FaultKind, InvalidCode, Location};

/// Four direct laws/seeds and eight original-operator lifted applications.
fn fixture(kind: usize, reordered: bool, output: bool) -> TestResult<Fixture> {
    if kind < 4 {
        setup(kind, 6, reordered, output)
    } else {
        lifted(kind - 4, reordered)
    }
}

/// Every retry starts from identical syntax, earlier facts, reuse state and prior caller usage.
pub(crate) fn attempt(
    mode: usize,
    kind: usize,
    dimension: BudgetKind,
    amount: u64,
) -> TestResult<(bool, u64)> {
    let mut limits = super::half_pi_limits::ceiling(dimension, amount);
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
            Budget::new(limits).expect_err("positive bit policy").kind,
            FaultKind::Invalid(InvalidCode::Policy)
        );
        return Ok((false, 0));
    }
    let mut budget = Budget::new(limits)?;
    let at = Location::new("complete source boundary");
    budget.work(37, at)?;
    let mut f = fixture(kind, false, !matches!(mode, 1 | 2))?;
    let prior = if mode >= 2 {
        Some(
            f.base
                .cas
                .expand_series_source(&f.request, &f.proofs, &mut default_budget()?)?,
        )
    } else {
        None
    };
    if mode == 4 {
        let bytes = f
            .base
            .cas
            .export_witness(prior.as_ref().ok_or("prior")?, &mut default_budget()?)?;
        f = super::series_source::receiver(kind, 6, true)?;
        let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
        let result = budget.with_retained_payload(113, at, |budget| {
            f.base
                .cas
                .replay(
                    &bytes,
                    f.expected
                        .as_ref()
                        .ok_or_else(|| crate::test_api::Fault::invalid(InvalidCode::Rule, at))?,
                    budget,
                )
                .map(|_| ())
        });
        return Ok(result_row(
            result,
            before == f.base.cas.canonical_snapshot(&mut default_budget()?)?,
            &budget,
            mode,
            kind,
            dimension,
            amount,
        ));
    }
    let before = f.base.cas.canonical_snapshot(&mut default_budget()?)?;
    let expected = f.expected.as_ref().map(statement).transpose()?;
    let result = budget.with_retained_payload(113, at, |budget| match mode {
        0 => f
            .base
            .cas
            .check_series_source(
                expected.ok_or_else(|| crate::test_api::Fault::invalid(InvalidCode::Rule, at))?,
                &f.certificate,
                &f.proofs,
                budget,
            )
            .map(|_| ()),
        1 | 2 => f
            .base
            .cas
            .expand_series_source(&f.request, &f.proofs, budget)
            .map(|_| ()),
        3 => f
            .base
            .cas
            .export_witness(
                prior
                    .as_ref()
                    .ok_or_else(|| crate::test_api::Fault::invalid(InvalidCode::Rule, at))?,
                budget,
            )
            .map(|_| ()),
        _ => Err(crate::test_api::Fault::invalid(InvalidCode::Rule, at)),
    });
    Ok(result_row(
        result,
        before == f.base.cas.canonical_snapshot(&mut default_budget()?)?,
        &budget,
        mode,
        kind,
        dimension,
        amount,
    ))
}

/// Resource exhaustion retains earlier canonical state and never refunds caller Work.
fn result_row(
    result: Result<(), crate::test_api::Fault>,
    unchanged: bool,
    budget: &Budget,
    mode: usize,
    kind: usize,
    dimension: BudgetKind,
    amount: u64,
) -> (bool, u64) {
    match result {
        Ok(()) => (true, budget.usage().work),
        Err(fault) => {
            assert!(
                matches!(fault.kind,FaultKind::Exhausted { budget: actual,.. } if actual == dimension || (dimension == BudgetKind::IntermediateBits && actual == BudgetKind::ComponentBits)),
                "source {mode}/{kind} {dimension:?} {amount}: {fault:?}"
            );
            assert!(unchanged, "source refusal rollback");
            (false, budget.usage().work)
        }
    }
}

/// Search complete successful ceilings and independently execute every one-unit-short refusal.
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

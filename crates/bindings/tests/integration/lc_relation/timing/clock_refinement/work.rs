// SPDX-License-Identifier: AGPL-3.0-only
//! Exhausted solver attempts retain all prior evidence and the complete refined measurement domain.

use super::{TestResult, clock, cycles, ledger, number, pi, timing};
use nrz_bindings::{
    Budget, LcClockRefinement, LcClockRefinementState, LcRelationError, Limits, LimitsOverride,
};
use nrz_cas::{Fault, FaultKind, Location, limits::BudgetKind};

/// Work exhaustion inside the new solver preserves the old clock cover, horizon and complete inclusion proof.
/// ´test:integration:lc-relation-clock-refinement-exhaustion´
#[test]
fn lc_relation_clock_refinement_exhaustion() -> TestResult {
    let old = clock(false, "1/100", ["-1/10", "1/10"])?;
    let refined = clock(false, "1/200", ["-1/20", "1/20"])?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let attempt = exhausted(|budget| previous.refine(&inclusion, &horizon, cycles(), budget))?;
    assert!(
        attempt
            .previous_clock()
            .ok_or("old cover")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .previous_horizon()
            .clock_cover()
            .ok_or("old horizon source")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .inclusion()
            .refined_clock()
            .ok_or("new original question")?
            .shares_allocation(&refined)
    );
    assert!(
        attempt
            .inclusion()
            .original_inputs()
            .refined()
            .shares_allocation(refined.relation())
    );
    assert_eq!(attempt.proposed_cycles(), cycles());
    assert_eq!(
        attempt.inclusion().original_inputs().counterparts().len(),
        old.relation().inputs().len()
    );
    boundary_inclusion(|budget| old.refinement_inputs(&refined, budget))
}

/// Both independent and systematic timing layers survive exhaustion during the refined physical solve.
/// ´test:integration:lc-relation-both-timing-refinement-exhaustion´
#[test]
fn lc_relation_both_timing_refinement_exhaustion() -> TestResult {
    let old = timing(false, false, false)?;
    let refined = timing(false, true, true)?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_horizon(&mut ledger()?)?;
    assert!(
        previous
            .forward_horizon_from(&number("2")?, &mut ledger()?)
            .is_err()
    );
    let reference = number("3")?;
    boundary_inclusion(|budget| previous.forward_horizon_from(&reference, budget))?;

    let attempt = exhausted(|budget| previous.refine(&inclusion, &horizon, cycles(), budget))?;
    assert!(
        attempt
            .previous_shared_timing()
            .ok_or("old timing")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .previous_horizon()
            .shared_timing_cover()
            .ok_or("old horizon source")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .inclusion()
            .refined_shared_timing()
            .ok_or("new timing question")?
            .shares_allocation(&refined)
    );
    assert_eq!(attempt.proposed_cycles(), cycles());
    boundary_inclusion(|budget| old.refinement_inputs(&refined, budget))
}

/// Exact full-work and one-short budgets distinguish certified success from a retained undecided attempt.
fn exhausted(
    operation: impl Fn(&mut Budget) -> Result<LcClockRefinement, LcRelationError>,
) -> TestResult<LcClockRefinement> {
    let mut measured = ledger()?;
    let complete = operation(&mut measured)?;
    assert!(matches!(
        complete.state(),
        LcClockRefinementState::Refined(_)
    ));
    let work = measured.usage().work;
    let start = complete.solving_usage().ok_or("measured solver usage")?[0].work;
    for (limit, success) in [(work + 17, true), (work + 16, false)] {
        let mut budget = limited(limit)?;
        let attempt = operation(&mut budget)?;
        assert_eq!(
            matches!(attempt.state(), LcClockRefinementState::Refined(_)),
            success
        );
        if !success {
            typed(&attempt);
        }
        assert_eq!(attempt.usage()[0].work, 17);
        assert_eq!(attempt.usage()[1].work, budget.usage().work);
    }
    let mut budget = limited(start + 18)?;
    let attempt = operation(&mut budget)?;
    typed(&attempt);
    let solving = attempt
        .solving_usage()
        .ok_or("exhaustion must occur inside solving")?;
    assert_eq!(solving[0].work, start + 17);
    assert!(solving[1].work > solving[0].work);
    assert_eq!(attempt.usage()[1].work, budget.usage().work);
    assert!(attempt.refined_horizon().is_none());
    Ok(attempt)
}

/// The unresolved reason preserves the typed Work exhaustion, not fabricated impossibility.
fn typed(attempt: &LcClockRefinement) {
    assert!(matches!(
        attempt.state(),
        LcClockRefinementState::Undecided(LcRelationError::Algebra(Fault {
            kind: FaultKind::Exhausted {
                budget: BudgetKind::Work,
                ..
            },
            ..
        }))
    ));
}

/// A consumed prefix remains part of the caller's fixed bound.
fn limited(work: u64) -> TestResult<Budget> {
    let mut budget = Budget::new(Limits::default().resolve(&LimitsOverride {
        work: Some(work),
        ..LimitsOverride::default()
    })?)?;
    budget.work(17, Location::new("clock refinement prefix"))?;
    Ok(budget)
}

/// Inclusion checking itself uses the same cumulative caller ledger.
fn boundary_inclusion<T>(
    operation: impl Fn(&mut Budget) -> Result<T, LcRelationError>,
) -> TestResult {
    let mut measured = ledger()?;
    operation(&mut measured)?;
    for (work, success) in [
        (measured.usage().work + 17, true),
        (measured.usage().work + 16, false),
    ] {
        let mut budget = limited(work)?;
        let result = operation(&mut budget);
        assert_eq!(result.is_ok(), success);
        if !success {
            assert!(matches!(
                result,
                Err(LcRelationError::Algebra(Fault {
                    kind: FaultKind::Exhausted {
                        budget: BudgetKind::Work,
                        ..
                    },
                    ..
                }))
            ));
        }
    }
    Ok(())
}

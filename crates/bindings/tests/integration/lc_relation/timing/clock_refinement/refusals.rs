// SPDX-License-Identifier: AGPL-3.0-only
//! Inclusion requires original physical declarations, nominal identities and contained shared error layers.

use super::support::interval;
use super::{TestResult, clock, cycles, ledger, number, pi, timing};
use nrz_bindings::{
    LcClockRefinementState, LcQuantity as Q, LcRelation, LcSharedClock, LcSharedTiming,
};

/// Weaker original equations, widened declared offsets/errors and changed nominal coordinates refuse inclusion.
/// ´test:integration:lc-relation-clock-refinement-input-refusals´
#[test]
fn lc_relation_clock_refinement_input_refusals() -> TestResult {
    let old = clock(false, "1/100", ["-1/10", "1/10"])?;
    let refined = clock(false, "1/200", ["-1/20", "1/20"])?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    assert!(
        inclusion
            .previous_clock()
            .ok_or("old question")?
            .shares_allocation(&old)
    );
    assert!(
        inclusion
            .original_inputs()
            .previous()
            .shares_allocation(old.relation())
    );
    assert_eq!(
        inclusion.original_inputs().counterparts().len(),
        old.relation().inputs().len()
    );
    assert!(refined.refinement_inputs(&old, &mut ledger()?).is_err());
    let wider = clock(false, "1/200", ["-3/20", "3/20"])?;
    assert!(old.refinement_inputs(&wider, &mut ledger()?).is_err());
    let shifted = LcSharedClock::new(
        refined.relation().clone(),
        vec![number("101/100")?, number("2")?],
        &interval(["-1/20", "1/20"])?,
        &mut ledger()?,
    )?;
    assert!(old.refinement_inputs(&shifted, &mut ledger()?).is_err());
    let mut inputs = refined.relation().inputs().to_vec();
    inputs.retain(|value| !matches!(value.quantity(), Q::Time(_)));
    let dropped = LcRelation::new(old.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
    let dropped = LcSharedClock::new(
        dropped,
        refined.nominal_times().to_vec(),
        &interval(["-1/20", "1/20"])?,
        &mut ledger()?,
    )?;
    assert!(old.refinement_inputs(&dropped, &mut ledger()?).is_err());
    timing_refusals()
}

/// Independent errors are declarations; a narrower physical timestamp cannot excuse widening them.
fn timing_refusals() -> TestResult {
    let old = timing(false, false, false)?;
    let refined = timing(false, true, true)?;
    old.refinement_inputs(&refined, &mut ledger()?)?;
    let wider = LcSharedTiming::new(
        refined.relation().clone(),
        refined.nominal_times().to_vec(),
        &[interval(["-1/50", "1/50"])?, interval(["-1/25", "1/25"])?],
        &interval(["-1/20", "1/20"])?,
        &mut ledger()?,
    )?;
    assert!(old.refinement_inputs(&wider, &mut ledger()?).is_err());
    assert!(refined.refinement_inputs(&old, &mut ledger()?).is_err());
    Ok(())
}

/// A foreign equal-value horizon cannot replace source authority, and wrong cycles remain unresolved.
/// ´test:integration:lc-relation-clock-refinement-identity-refusals´
#[test]
fn lc_relation_clock_refinement_identity_refusals() -> TestResult {
    let old = clock(true, "1/100", ["-1/10", "1/10"])?;
    let refined = clock(true, "1/200", ["-1/20", "1/20"])?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let pi = pi()?;
    let previous = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let foreign = old
        .pairs_cover(cycles(), &pi, &mut ledger()?)?
        .forward_reported_horizon(&mut ledger()?)?;
    assert!(
        previous
            .refine(&inclusion, &foreign, cycles(), &mut ledger()?)
            .is_err()
    );
    let equal_question = clock(true, "1/100", ["-1/10", "1/10"])?;
    let foreign_input = equal_question.refinement_inputs(&refined, &mut ledger()?)?;
    assert!(
        previous
            .refine(&foreign_input, &horizon, cycles(), &mut ledger()?)
            .is_err()
    );
    let proposed = vec![number("0")?, number("1")?];
    let attempt = previous.refine(&inclusion, &horizon, proposed.clone(), &mut ledger()?)?;
    assert!(matches!(
        attempt.state(),
        LcClockRefinementState::Undecided(nrz_bindings::LcRelationError::CycleCover)
    ));
    assert!(
        attempt
            .previous_clock()
            .ok_or("original cover")?
            .shares_allocation(&previous)
    );
    assert_eq!(attempt.proposed_cycles(), proposed);
    assert!(attempt.refined_horizon().is_none());
    assert!(attempt.solving_usage().is_none());
    Ok(())
}

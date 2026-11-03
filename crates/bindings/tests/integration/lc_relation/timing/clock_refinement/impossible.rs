// SPDX-License-Identifier: AGPL-3.0-only
//! A separately certified original contradiction excludes every shared-clock refinement branch.

use super::{TestResult, clock, cycles, input, ledger, number, pi, timing};
use nrz_bindings::{
    Budget, LcCertainty, LcClockRefinementState, LcDeterminacy, LcInterval, LcQuantity as Q,
    LcRelation, LcRelationError, LcSharedClock, LcSharedTiming, Limits, LimitsOverride,
};
use nrz_cas::Rational;

/// Original observation identities used by the same-time all-alias contradiction.
const FIRST: usize = 0;
/// New observation at the first original timestamp.
const ADDED: usize = 2;

/// Exact added constraints precede broad originals, which remain retained in full.
fn original(old: &LcRelation, phase: &str) -> TestResult<LcRelation> {
    let mut inputs = vec![
        input(Q::Time(FIRST), ["1", "1"])?,
        input(Q::Phase(FIRST), ["2", "2"])?,
    ];
    inputs.extend_from_slice(old.inputs());
    inputs.push(input(Q::Time(ADDED), ["1", "1"])?);
    inputs.push(input(Q::Phase(ADDED), [phase, phase])?);
    Ok(LcRelation::new(
        old.epoch().into(),
        3,
        inputs,
        &mut ledger()?,
    )?)
}

/// Signed supports keep the original error coordinate meanings.
fn interval(bounds: &[Rational; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(bounds[0].clone().into()),
        Some(bounds[1].clone().into()),
        &mut ledger()?,
    )?)
}

/// Added physical timestamp equality is distinct from the shared-offset declaration.
fn refined_clock(old: &LcSharedClock, phase: &str) -> TestResult<LcSharedClock> {
    let mut nominal = old.nominal_times().to_vec();
    nominal.push(number("1")?);
    Ok(LcSharedClock::new(
        original(old.relation(), phase)?,
        nominal,
        &interval(old.offset())?,
        &mut ledger()?,
    )?)
}

/// Both independent and shared timing supports remain in the refined original question.
fn refined_timing(old: &LcSharedTiming) -> TestResult<LcSharedTiming> {
    let mut nominal = old.nominal_times().to_vec();
    nominal.push(number("1")?);
    let mut errors = old
        .errors()
        .iter()
        .map(interval)
        .collect::<TestResult<Vec<_>>>()?;
    errors.push(interval(&old.errors()[FIRST])?);
    Ok(LcSharedTiming::new(
        original(old.relation(), "3")?,
        nominal,
        &errors,
        &interval(old.offset())?,
        &mut ledger()?,
    )?)
}

/// Whole-domain impossibility retains prior evidence in both future clock coordinates.
/// ´test:integration:lc-relation-clock-refinement-certified-empty´
#[test]
fn lc_relation_clock_refinement_certified_empty() -> TestResult {
    let pi = pi()?;
    let old = clock(true, "1/100", ["-1/10", "1/10"])?;
    let refined = refined_clock(&old, "3")?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let certificate = refined.relation().impossible_pair(
        [FIRST, ADDED],
        [number("0")?, number("0")?],
        &pi,
        &mut ledger()?,
    )?;
    assert_eq!(certificate.determinacy(), LcDeterminacy::Impossible);
    assert_eq!(certificate.certainty(), LcCertainty::Certified);
    assert_eq!(certificate.gap(), &[number("1")?, number("1")?]);
    for horizon in [
        previous.forward_horizon(&mut ledger()?)?,
        previous.forward_reported_horizon(&mut ledger()?)?,
    ] {
        let attempt =
            previous.refine_impossible(&inclusion, &horizon, &certificate, &mut ledger()?)?;
        let LcClockRefinementState::Impossible(retained) = attempt.state() else {
            return Err("certified whole-domain emptiness".into());
        };
        assert!(retained.relation().shares_allocation(refined.relation()));
        assert!(
            attempt
                .previous_clock()
                .ok_or("old clock cover")?
                .shares_allocation(&previous)
        );
        assert!(
            attempt
                .inclusion()
                .refined_clock()
                .ok_or("new joint question")?
                .shares_allocation(&refined)
        );
        assert_eq!(
            attempt.previous_horizon().coordinate(),
            horizon.coordinate()
        );
        assert_eq!(
            attempt.previous_horizon().mathematical().reference(),
            horizon.mathematical().reference()
        );
        assert!(attempt.refined_horizon().is_none());
        assert!(attempt.solving_usage().is_none());
        assert_eq!(attempt.proposed_cycles(), [] as [Rational; 0]);
    }
    Ok(())
}

/// A physical original contradiction excludes the whole two-layer measurement subset as well.
/// ´test:integration:lc-relation-both-timing-refinement-certified-empty´
#[test]
fn lc_relation_both_timing_refinement_certified_empty() -> TestResult {
    let pi = pi()?;
    let old = timing(true, false, false)?;
    let refined = refined_timing(&old)?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let certificate = refined.relation().impossible_pair(
        [FIRST, ADDED],
        [number("0")?, number("0")?],
        &pi,
        &mut ledger()?,
    )?;
    let attempt = previous.refine_impossible(&inclusion, &horizon, &certificate, &mut ledger()?)?;
    assert!(matches!(
        attempt.state(),
        LcClockRefinementState::Impossible(_)
    ));
    assert!(
        attempt
            .previous_shared_timing()
            .ok_or("old complete timing")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .inclusion()
            .refined_shared_timing()
            .ok_or("full refined timing")?
            .shares_allocation(&refined)
    );
    assert!(attempt.refined_horizon().is_none());
    Ok(())
}

/// Equal-valued foreign certificates and false whole-domain contradiction proposals refuse.
/// ´test:integration:lc-relation-clock-refinement-empty-authority´
#[test]
fn lc_relation_clock_refinement_empty_authority() -> TestResult {
    let pi = pi()?;
    let old = clock(true, "1/100", ["-1/10", "1/10"])?;
    let refined = refined_clock(&old, "3")?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let horizon = previous.forward_horizon(&mut ledger()?)?;
    let foreign = refined_clock(&old, "3")?.relation().impossible_pair(
        [FIRST, ADDED],
        [number("0")?, number("0")?],
        &pi,
        &mut ledger()?,
    )?;
    assert!(matches!(
        previous.refine_impossible(&inclusion, &horizon, &foreign, &mut ledger()?),
        Err(LcRelationError::Containment)
    ));
    let possible = refined_clock(&old, "2")?;
    assert!(
        possible
            .relation()
            .impossible_pair(
                [FIRST, ADDED],
                [number("0")?, number("0")?],
                &pi,
                &mut ledger()?
            )
            .is_err()
    );
    let attempt = previous.refine(
        &inclusion,
        &horizon,
        vec![number("0")?, number("0")?, number("0")?],
        &mut ledger()?,
    )?;
    assert!(matches!(
        attempt.state(),
        LcClockRefinementState::Undecided(_)
    ));
    Ok(())
}

/// Retaining certified emptiness spends the cumulative ledger and never fabricates a new horizon.
/// ´test:integration:lc-relation-clock-refinement-empty-work´
#[test]
fn lc_relation_clock_refinement_empty_work() -> TestResult {
    let pi = pi()?;
    let old = clock(true, "1/100", ["-1/10", "1/10"])?;
    let refined = refined_clock(&old, "3")?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let horizon = previous.forward_horizon(&mut ledger()?)?;
    let certificate = refined.relation().impossible_pair(
        [FIRST, ADDED],
        [number("0")?, number("0")?],
        &pi,
        &mut ledger()?,
    )?;
    let mut measured = ledger()?;
    let attempt = previous.refine_impossible(&inclusion, &horizon, &certificate, &mut measured)?;
    assert_eq!(
        attempt.usage()[1].work - attempt.usage()[0].work,
        measured.usage().work
    );
    for (limit, succeeds) in [
        (measured.usage().work + 17, true),
        (measured.usage().work + 16, false),
    ] {
        let mut budget = Budget::new(Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?)?;
        budget.work(
            17,
            nrz_cas::Location::new("certified empty refinement prefix"),
        )?;
        assert_eq!(
            previous
                .refine_impossible(&inclusion, &horizon, &certificate, &mut budget)
                .is_ok(),
            succeeds
        );
    }
    assert!(
        previous
            .refine_impossible(&inclusion, &horizon, &certificate, &mut ledger()?)
            .is_ok()
    );
    Ok(())
}

/// The same whole-original certificate also excludes the independent-only timing subset.
/// ´test:integration:lc-relation-independent-refinement-certified-empty´
#[test]
fn lc_relation_independent_refinement_certified_empty() -> TestResult {
    let pi = pi()?;
    let old = super::super::support::sample_times()?;
    let refined = nrz_bindings::LcSampleTimes::new(
        original(old.relation(), "3")?,
        vec![number("1")?, number("2")?, number("1")?],
        &mut ledger()?,
    )?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let horizon = previous.forward_horizon(&mut ledger()?)?;
    let certificate = refined.relation().impossible_pair(
        [FIRST, ADDED],
        [number("0")?, number("0")?],
        &pi,
        &mut ledger()?,
    )?;
    let attempt = previous.refine_impossible(&inclusion, &horizon, &certificate, &mut ledger()?)?;
    assert!(matches!(
        attempt.state(),
        LcClockRefinementState::Impossible(_)
    ));
    assert!(
        attempt
            .previous_timing()
            .ok_or("old independent timing")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .inclusion()
            .refined_timing()
            .ok_or("refined independent question")?
            .shares_allocation(&refined)
    );
    assert!(attempt.refined_horizon().is_none());
    Ok(())
}

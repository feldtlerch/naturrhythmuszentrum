// SPDX-License-Identifier: AGPL-3.0-only
//! Independent-only timing refinement retains the same original source and common future reference.

use super::super::super::support::{TestResult, ledger};
use super::super::support::{cycles, input, number, pi};
use nrz_bindings::{
    Budget, LcClockRefinementState, LcHorizonCoordinate, LcHorizonValue, LcPairParameters,
    LcQuantity as Q, LcRelation, LcRelationError, LcSampleTimes, LcTimingCover, Limits,
    LimitsOverride,
};
use nrz_cas::Rational;
use std::cmp::Ordering;

/// Narrowed timestamp supports keep every old nominal coordinate and original physical quantity.
fn question(known: bool, width: usize) -> TestResult<LcSampleTimes> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if known {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
    }
    let times = match width {
        0 => [["9/10", "11/10"], ["9/5", "23/10"]],
        1 => [["19/20", "21/20"], ["19/10", "21/10"]],
        _ => [["1", "1"], ["2", "2"]],
    };
    for (index, (time, phase)) in times.into_iter().zip(["2", "4"]).enumerate() {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    let original = LcRelation::new(
        "independent refinement epoch".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    Ok(LcSampleTimes::new(
        original,
        vec![number("1")?, number("2")?],
        &mut ledger()?,
    )?)
}

/// Exact rational endpoint inclusion is independent of the production refinement check.
fn included(old: &[Rational; 2], new: &[Rational; 2]) -> TestResult {
    assert_ne!(old[0].compare(&new[0], &mut ledger()?)?, Ordering::Greater);
    assert_ne!(old[1].compare(&new[1], &mut ledger()?)?, Ordering::Less);
    Ok(())
}

/// Every endpoint fibre restores original timing equations and satisfies all previous inputs.
fn witnesses(old: &LcTimingCover, new: &LcTimingCover) -> TestResult {
    included(old.rate(), new.rate())?;
    for rate in new.rate() {
        let old_phase = old.phase_at_rate(rate, &mut ledger()?)?;
        let new_phase = new.phase_at_rate(rate, &mut ledger()?)?;
        included(&old_phase, &new_phase)?;
        for phase in new_phase {
            let parameters = LcPairParameters {
                rate: rate.clone(),
                initial_phase: phase,
                gauge: number("0")?,
                ratio: number("1")?,
                voltage: number("1")?,
            };
            let point = new.witness(&parameters, &mut ledger()?)?;
            old.relation()
                .check_witness(point.values().clone(), &mut ledger()?)?;
        }
    }
    Ok(())
}

/// Known and unknown initial phase retain two narrowing steps and an explicit infinite final horizon.
/// ´test:integration:lc-relation-independent-timing-refinement´
#[test]
fn lc_relation_independent_timing_refinement() -> TestResult {
    let pi = pi()?;
    for known in [false, true] {
        let mut original = question(known, 0)?;
        let mut previous = original.pairs_cover(cycles(), &pi, &mut ledger()?)?;
        let mut horizon = previous.forward_horizon(&mut ledger()?)?;
        for width in [1, 2] {
            let refined = question(known, width)?;
            let inclusion = original.refinement_inputs(&refined, &mut ledger()?)?;
            let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
            let LcClockRefinementState::Refined(success) = attempt.state() else {
                return Err(format!("independent refinement {:?}", attempt.state()).into());
            };
            let next = success.timing_cover().ok_or("independent timing success")?;
            assert!(
                attempt
                    .previous_timing()
                    .ok_or("old timing")?
                    .shares_allocation(&previous)
            );
            assert!(
                attempt
                    .inclusion()
                    .previous_timing()
                    .ok_or("old question")?
                    .shares_allocation(&original)
            );
            assert!(
                attempt
                    .inclusion()
                    .refined_timing()
                    .ok_or("new question")?
                    .shares_allocation(&refined)
            );
            assert!(
                success
                    .horizon()
                    .timing_cover()
                    .ok_or("original horizon source")?
                    .shares_allocation(next)
            );
            assert_eq!(
                success.horizon().coordinate(),
                LcHorizonCoordinate::ActualTime
            );
            assert_eq!(
                success.horizon().mathematical().reference(),
                horizon.mathematical().reference()
            );
            included(
                &success.product_inclusion()[0],
                &success.product_inclusion()[1],
            )?;
            witnesses(&previous, next)?;
            if width == 2 {
                assert!(matches!(
                    success.horizon().value(),
                    LcHorizonValue::Infinite
                ));
            } else {
                assert!(
                    !success
                        .finite_horizon_margin()
                        .ok_or("finite margin")?
                        .is_negative()
                );
            }
            original = refined;
            previous = next.clone();
            horizon = success.horizon().clone();
        }
    }
    Ok(())
}

/// Equal values from another source allocation cannot replace the original cover or its horizon.
/// ´test:integration:lc-relation-independent-timing-refinement-authority´
#[test]
fn lc_relation_independent_timing_refinement_authority() -> TestResult {
    let pi = pi()?;
    let old = question(true, 0)?;
    let refined = question(true, 1)?;
    let cover = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let horizon = cover.forward_horizon(&mut ledger()?)?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let other_cover = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    assert!(matches!(
        other_cover.refine(&inclusion, &horizon, cycles(), &mut ledger()?),
        Err(LcRelationError::Containment)
    ));
    let duplicate = question(true, 0)?;
    let foreign = duplicate.refinement_inputs(&refined, &mut ledger()?)?;
    assert!(matches!(
        cover.refine(&foreign, &horizon, cycles(), &mut ledger()?),
        Err(LcRelationError::Containment)
    ));
    let shifted = LcSampleTimes::new(
        refined.relation().clone(),
        vec![number("0")?, number("2")?],
        &mut ledger()?,
    )?;
    assert!(old.refinement_inputs(&shifted, &mut ledger()?).is_err());
    assert!(refined.refinement_inputs(&old, &mut ledger()?).is_err());
    let bad = cover.refine(
        &inclusion,
        &horizon,
        vec![number("0")?, number("1")?],
        &mut ledger()?,
    )?;
    assert!(matches!(
        bad.state(),
        LcClockRefinementState::Undecided(LcRelationError::CycleCover)
    ));
    assert!(
        bad.previous_timing()
            .ok_or("preserved old cover")?
            .shares_allocation(&cover)
    );
    Ok(())
}

/// Exhausted independent solving retains all original questions, cycles, old horizon and spent Work.
/// ´test:integration:lc-relation-independent-timing-refinement-exhaustion´
#[test]
fn lc_relation_independent_timing_refinement_exhaustion() -> TestResult {
    let pi = pi()?;
    let old = question(true, 0)?;
    let refined = question(true, 1)?;
    let previous = old.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let horizon = previous.forward_horizon(&mut ledger()?)?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let mut measured = ledger()?;
    let complete = previous.refine(&inclusion, &horizon, cycles(), &mut measured)?;
    let solving = complete.solving_usage().ok_or("solving receipt")?;
    let limit = solving[0].work + 18;
    let mut budget = Budget::new(Limits::default().resolve(&LimitsOverride {
        work: Some(limit),
        ..LimitsOverride::default()
    })?)?;
    budget.work(17, nrz_cas::Location::new("independent refinement prefix"))?;
    let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut budget)?;
    assert!(matches!(
        attempt.state(),
        LcClockRefinementState::Undecided(_)
    ));
    assert!(attempt.refined_horizon().is_none());
    assert!(
        attempt
            .previous_timing()
            .ok_or("old cover")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .previous_horizon()
            .timing_cover()
            .ok_or("old horizon source")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .inclusion()
            .refined_timing()
            .ok_or("refined source")?
            .shares_allocation(&refined)
    );
    assert_eq!(attempt.proposed_cycles(), cycles());
    assert_eq!(attempt.usage()[1].work, budget.usage().work);
    assert!(
        attempt.solving_usage().ok_or("failed solve receipt")?[1].work
            > attempt.solving_usage().ok_or("start receipt")?[0].work
    );
    assert!(
        previous
            .refine(&inclusion, &horizon, cycles(), &mut ledger()?)
            .is_ok()
    );
    Ok(())
}

/// Input inclusion and complete successful refinement have exact N/N-1 cumulative Work boundaries.
/// ´test:integration:lc-relation-independent-timing-refinement-work´
#[test]
fn lc_relation_independent_timing_refinement_work() -> TestResult {
    let old = question(false, 0)?;
    let refined = question(false, 1)?;
    let mut measured = ledger()?;
    let inclusion = old.refinement_inputs(&refined, &mut measured)?;
    boundary(measured.usage().work, |budget| {
        old.refinement_inputs(&refined, budget).is_ok()
    })?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_horizon(&mut ledger()?)?;
    let mut measured = ledger()?;
    let complete = previous.refine(&inclusion, &horizon, cycles(), &mut measured)?;
    assert!(matches!(
        complete.state(),
        LcClockRefinementState::Refined(_)
    ));
    boundary(measured.usage().work, |budget| {
        previous
            .refine(&inclusion, &horizon, cycles(), budget)
            .is_ok_and(|attempt| matches!(attempt.state(), LcClockRefinementState::Refined(_)))
    })?;
    Ok(())
}

/// Previously spent Work is neither hidden nor refunded on an unsuccessful attempt.
fn boundary(work: u64, operation: impl Fn(&mut Budget) -> bool) -> TestResult {
    for (limit, success) in [(work + 17, true), (work + 16, false)] {
        let mut budget = Budget::new(Limits::default().resolve(&LimitsOverride {
            work: Some(limit),
            ..LimitsOverride::default()
        })?)?;
        budget.work(
            17,
            nrz_cas::Location::new("independent refinement boundary"),
        )?;
        assert_eq!(operation(&mut budget), success);
    }
    Ok(())
}

/// An added original observation before the common future reference can determine the exact rate.
/// ´test:integration:lc-relation-independent-timing-refinement-added-observation´
#[test]
fn lc_relation_independent_timing_refinement_added_observation() -> TestResult {
    const ADDED: usize = 2;
    let old = question(true, 0)?;
    let mut inputs = old.relation().inputs().to_vec();
    inputs.push(input(Q::Time(ADDED), ["3/2", "3/2"])?);
    inputs.push(input(Q::Phase(ADDED), ["3", "3"])?);
    let relation = LcRelation::new(old.relation().epoch().into(), 3, inputs, &mut ledger()?)?;
    let refined = LcSampleTimes::new(
        relation,
        vec![number("1")?, number("2")?, number("3/2")?],
        &mut ledger()?,
    )?;
    let inclusion = old.refinement_inputs(&refined, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_horizon(&mut ledger()?)?;
    let attempt = previous.refine(
        &inclusion,
        &horizon,
        vec![Rational::zero(); 3],
        &mut ledger()?,
    )?;
    let LcClockRefinementState::Refined(success) = attempt.state() else {
        return Err(format!("added independent observation {:?}", attempt.state()).into());
    };
    let next = success.timing_cover().ok_or("new timing cover")?;
    assert_eq!(next.rate(), &[number("2")?, number("2")?]);
    assert_eq!(next.cycles().len(), 3);
    assert!(next.timing().shares_allocation(&refined));
    assert!(matches!(
        success.horizon().value(),
        LcHorizonValue::Infinite
    ));
    assert_eq!(
        success.horizon().mathematical().reference(),
        horizon.mathematical().reference()
    );
    Ok(())
}

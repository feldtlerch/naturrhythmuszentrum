// SPDX-License-Identifier: AGPL-3.0-only
//! Redundant original-implied measured strips establish independent complete cycle coverage.

use super::support::{parameters, work_boundary};
use super::{TestResult, cycles, input, ledger, number, pi};
use nrz_bindings::{
    LcCertainty, LcInterval, LcQuantity as Q, LcRelation, LcSharedClock, LcSharedClockPhase,
};

/// Broad measured supports retain narrow repeated original physical phase restrictions.
fn question() -> TestResult<LcSharedClockPhase> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    for (index, (time, phase)) in [(["1/2", "3/2"], "2"), (["3/2", "5/2"], "4")]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), ["-20", "20"])?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    let relation = LcRelation::new(
        "independent original-implied cycle proof".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let offset = interval(["-1/4", "1/4"])?;
    let clock = LcSharedClock::new(
        relation,
        vec![number("1")?, number("2")?],
        &offset,
        &mut ledger()?,
    )?;
    Ok(LcSharedClockPhase::new(
        clock,
        vec![interval(["-10", "10"])?, interval(["-10", "10"])?],
        &offset,
        &mut ledger()?,
    )?)
}

/// Finite signed supports remain exact original intervals.
fn interval(bounds: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(bounds[0])?.into()),
        Some(number(bounds[1])?.into()),
        &mut ledger()?,
    )?)
}

/// The former relaxed measured problem has no admitted complete cover; redundant originals close it.
/// ´test:integration:lc-relation-original-implied-cycle-cover´
#[test]
fn lc_relation_original_implied_cycle_cover() -> TestResult {
    let source = question()?;
    let mut relaxed = vec![input(Q::Rate, ["1", "3"])?];
    for (index, time) in ["1", "2"].into_iter().enumerate() {
        relaxed.push(input(Q::Time(index), [time, time])?);
        relaxed.push(input(Q::Phase(index), ["-10", "10"])?);
    }
    let relaxed = LcRelation::new(source.relation().epoch().into(), 2, relaxed, &mut ledger()?)?;
    assert!(
        relaxed
            .pairs_cover(cycles(), &pi()?, &mut ledger()?)
            .is_err()
    );
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
    assert!(cover.phase().shares_allocation(&source));
    assert_eq!(
        source.samples(),
        &[interval(["-10", "10"])?, interval(["-10", "10"])?]
    );
    let proof = cover
        .measured_cycle_cover()
        .ok_or("independent measured complete certificate")?;
    assert_eq!(proof.certainty(), LcCertainty::Certified);
    assert_eq!(proof.cycles(), cycles());
    for (index, bounds) in [["7/4", "9/4"], ["15/4", "17/4"]].into_iter().enumerate() {
        let input = proof
            .relation()
            .inputs()
            .iter()
            .find(|value| value.quantity() == Q::Phase(index))
            .ok_or("derived redundant measured strip")?;
        assert_eq!(input.interval(), &interval(bounds)?);
    }
    assert!(
        source
            .pairs_cover(vec![number("0")?, number("1")?], &pi()?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Original shared phase and clock fibres survive the derived strips, including every common gauge.
/// ´test:integration:lc-relation-original-implied-cycle-witnesses´
#[test]
fn lc_relation_original_implied_cycle_witnesses() -> TestResult {
    let source = question()?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    for (alpha, clock) in [("-1/2", "1/4"), ("0", "0"), ("1/2", "-1/4")] {
        for phase in ["-1/4", "0", "1/4"] {
            for gauge in ["-1", "0", "1"] {
                let point = cover.witness(
                    &parameters("2", alpha, clock, phase, gauge)?,
                    &mut ledger()?,
                )?;
                source.check_witness(
                    point.original().original().values().clone(),
                    point.original().offset().clone(),
                    point.offset().clone(),
                    &mut ledger()?,
                )?;
            }
        }
    }
    assert!(
        cover
            .witness(&parameters("2", "1/2", "1/4", "0", "0")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .witness(&parameters("2", "0", "0", "1/2", "0")?, &mut ledger()?)
            .is_err()
    );
    let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
    assert!(matches!(
        horizon.value(),
        nrz_bindings::LcHorizonValue::Infinite
    ));
    assert!(horizon.calibrated_future().is_some());
    Ok(())
}

/// The independent complete proof and original reconstruction spend the caller's exact cumulative Work.
/// ´test:integration:lc-relation-original-implied-cycle-work´
#[test]
fn lc_relation_original_implied_cycle_work() -> TestResult {
    let source = question()?;
    let pi = pi()?;
    work_boundary(|budget| source.pairs_cover(cycles(), &pi, budget))?;
    let cover = source.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let point = parameters("2", "0", "0", "0", "0")?;
    work_boundary(|budget| cover.witness(&point, budget))?;
    Ok(())
}

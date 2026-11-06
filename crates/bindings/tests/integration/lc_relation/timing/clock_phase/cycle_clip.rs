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

/// Original initial phase and signed timestamps restrict the common measured intercept across every alias.
fn across_epoch() -> TestResult<LcSharedClockPhase> {
    let mut inputs = vec![
        input(Q::Rate, ["1", "3"])?,
        input(Q::InitialPhase, ["0", "0"])?,
    ];
    for (index, (time, phase)) in [
        (["-13/4", "-11/4"], ["-13/2", "-11/2"]),
        (["11/4", "13/4"], ["11/2", "13/2"]),
    ]
    .into_iter()
    .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), phase)?);
    }
    let relation = LcRelation::new(
        "signed original cycle domain".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let offset = interval(["-1/8", "1/8"])?;
    let clock = LcSharedClock::new(
        relation,
        vec![number("-3")?, number("3")?],
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

/// The original-implied measured initial bound certifies a previously refused signed-time cycle domain.
/// ´test:integration:lc-relation-original-implied-initial-cycle-cover´
#[test]
fn lc_relation_original_implied_initial_cycle_cover() -> TestResult {
    let source = across_epoch()?;
    let mut relaxed = vec![input(Q::Rate, ["1", "3"])?];
    for (index, (time, phase)) in [("-3", ["-53/8", "-43/8"]), ("3", ["43/8", "53/8"])]
        .into_iter()
        .enumerate()
    {
        relaxed.push(input(Q::Time(index), [time, time])?);
        relaxed.push(input(Q::Phase(index), phase)?);
    }
    let relaxed = LcRelation::new(source.relation().epoch().into(), 2, relaxed, &mut ledger()?)?;
    assert!(matches!(
        relaxed.pairs_cover(cycles(), &pi()?, &mut ledger()?),
        Err(nrz_bindings::LcRelationError::CycleCover)
    ));
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.rate(), &[number("11/6")?, number("13/6")?]);
    assert!(cover.phase().shares_allocation(&source));
    let measured = cover
        .measured_cycle_cover()
        .ok_or("independent signed-cycle certificate")?;
    let initial = measured
        .relation()
        .inputs()
        .iter()
        .find(|value| value.quantity() == Q::InitialPhase)
        .ok_or("original-implied measured initial phase")?;
    assert_eq!(initial.interval(), &interval(["-1/2", "1/2"])?);
    assert_eq!(measured.phase_gauge(), nrz_bindings::LcPhaseGauge::Fixed);
    assert_eq!(measured.certainty(), LcCertainty::Certified);
    assert!(
        source
            .pairs_cover(vec![number("0")?, number("1")?], &pi()?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Full-original witnesses retain both calibration fibres and the fixed common gauge at every rate extreme.
/// ´test:integration:lc-relation-original-implied-initial-cycle-witnesses´
#[test]
fn lc_relation_original_implied_initial_cycle_witnesses() -> TestResult {
    let source = across_epoch()?;
    let pi = pi()?;
    let cover = source.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    for (rate, clocks) in [
        ("11/6", vec!["0"]),
        ("2", vec!["-1/8", "0", "1/8"]),
        ("13/6", vec!["0"]),
    ] {
        for clock in clocks {
            for phase in ["-1/8", "0", "1/8"] {
                let witness =
                    cover.witness(&parameters(rate, "0", clock, phase, "0")?, &mut ledger()?)?;
                source.check_witness(
                    witness.original().original().values().clone(),
                    witness.original().offset().clone(),
                    witness.offset().clone(),
                    &mut ledger()?,
                )?;
            }
        }
    }
    assert!(
        cover
            .witness(&parameters("11/6", "0", "1/8", "0", "0")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .witness(&parameters("2", "0", "0", "0", "1")?, &mut ledger()?)
            .is_err()
    );
    work_boundary(|budget| source.pairs_cover(cycles(), &pi, budget))?;
    let parameters = parameters("2", "0", "0", "0", "0")?;
    work_boundary(|budget| cover.witness(&parameters, budget))?;
    Ok(())
}

/// Clock supports of either strict sign retain the correct original-implied product extrema.
/// ´test:integration:lc-relation-original-implied-initial-signed-clock´
#[test]
fn lc_relation_original_implied_initial_signed_clock() -> TestResult {
    let original = across_epoch()?;
    for (clocks, expected) in [
        (["1/16", "1/8"], ["-1/16", "1/2"]),
        (["-1/8", "-1/16"], ["-1/2", "1/16"]),
    ] {
        let clock = LcSharedClock::new(
            original.relation().clone(),
            original.clock().nominal_times().to_vec(),
            &interval(clocks)?,
            &mut ledger()?,
        )?;
        let source = LcSharedClockPhase::new(
            clock,
            original.samples().to_vec(),
            &interval(["-1/8", "1/8"])?,
            &mut ledger()?,
        )?;
        let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        let measured = cover
            .measured_cycle_cover()
            .ok_or("complete measured domain")?;
        let initial = measured
            .relation()
            .inputs()
            .iter()
            .find(|value| value.quantity() == Q::InitialPhase)
            .ok_or("signed original-implied initial strip")?;
        assert_eq!(initial.interval(), &interval(expected)?);
        for clock in clocks {
            for offset in ["-1/8", "0", "1/8"] {
                let point =
                    cover.witness(&parameters("2", "0", clock, offset, "0")?, &mut ledger()?)?;
                source.check_witness(
                    point.original().original().values().clone(),
                    point.original().offset().clone(),
                    point.offset().clone(),
                    &mut ledger()?,
                )?;
            }
        }
    }
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Narrow physical phase restrictions retain actual and measured strips with both original offsets.

use super::support::{parameters, work_boundary};
use super::{TestResult, cycles, input, ledger, number, pi};
use nrz_bindings::{
    LcInterval, LcPhaseGauge, LcQuantity as Q, LcRelation, LcSharedClock, LcSharedClockPhase,
    LcSharedMeasurementParameters, LcSharedMeasurements,
};

/// Repeated physical phase restrictions are narrower than the original measurement images.
fn question(known: bool) -> TestResult<LcSharedClockPhase> {
    let mut inputs = vec![input(Q::Rate, ["1", "3"])?];
    if known {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
    }
    for (index, (time, broad, narrow)) in [
        (["9/10", "11/10"], ["39/20", "21/10"], ["2", "41/20"]),
        (["19/10", "21/10"], ["79/20", "41/10"], ["4", "81/20"]),
    ]
    .into_iter()
    .enumerate()
    {
        inputs.push(input(Q::Time(index), time)?);
        inputs.push(input(Q::Phase(index), broad)?);
        inputs.push(input(Q::Phase(index), narrow)?);
    }
    let relation = LcRelation::new(
        "narrow physical and measured strips".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let clock = LcSharedClock::new(
        relation,
        vec![number("1")?, number("2")?],
        &interval(["-1/10", "1/10"])?,
        &mut ledger()?,
    )?;
    Ok(LcSharedClockPhase::new(
        clock,
        vec![interval(["19/10", "21/10"])?, interval(["39/10", "41/10"])?],
        &interval(["-1/5", "1/5"])?,
        &mut ledger()?,
    )?)
}

/// Actual and measured phase strips yield exact rate and shared-offset fibres, including crossed refusals.
/// ´test:integration:lc-relation-narrower-clock-phase-strips´
#[test]
fn lc_relation_narrower_clock_phase_strips() -> TestResult {
    let source = question(true)?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert!(cover.phase().shares_allocation(&source));
    assert!(cover.relation().shares_allocation(source.relation()));
    assert_eq!(cover.rate(), &[number("39/20")?, number("41/20")?]);
    assert_eq!(
        cover.offset_at_phase(&number("2")?, &number("0")?, &mut ledger()?)?,
        [number("-1/10")?, number("3/20")?]
    );
    for (offset, clock) in [("3/20", "1/40"), ("-1/10", "0")] {
        assert_eq!(
            cover.clock_offset_at_phases(
                &number("2")?,
                &number("0")?,
                &number(offset)?,
                &mut ledger()?
            )?,
            [number(clock)?, number(clock)?]
        );
    }
    for (rate, clock, offset) in [
        ("39/20", "2/39", "1/10"),
        ("41/20", "-1/41", "-1/20"),
        ("2", "1/40", "3/20"),
        ("2", "0", "-1/10"),
    ] {
        let point = cover.witness(&parameters(rate, "0", clock, offset, "0")?, &mut ledger()?)?;
        source.check_witness(
            point.original().original().values().clone(),
            point.original().offset().clone(),
            point.offset().clone(),
            &mut ledger()?,
        )?;
    }
    assert!(
        cover
            .witness(&parameters("2", "0", "0", "3/20", "0")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .clock_offset_at_phases(
                &number("2")?,
                &number("0")?,
                &number("1/5")?,
                &mut ledger()?
            )
            .is_err()
    );
    Ok(())
}

/// Physical alpha, both shared offsets and every integer gauge remain conditional original coordinates.
/// ´test:integration:lc-relation-narrower-clock-phase-gauges´
#[test]
fn lc_relation_narrower_clock_phase_gauges() -> TestResult {
    let source = question(false)?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::AllIntegers);
    assert_eq!(
        cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-1/5")?, number("1/4")?]
    );
    for (alpha, clock, phase) in [("-1/5", "1/10", "0"), ("1/4", "-1/10", "1/20")] {
        for gauge in ["-2", "0", "3"] {
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
    assert!(
        cover
            .witness(
                &parameters("2", "1/4", "1/10", "1/20", "0")?,
                &mut ledger()?
            )
            .is_err()
    );
    Ok(())
}

/// Expanded total phase images can be feasible even when no one measured intercept exists.
/// ´test:integration:lc-relation-narrower-clock-phase-marginal-control´
#[test]
fn lc_relation_narrower_clock_phase_marginal_control() -> TestResult {
    let mut inputs = vec![input(Q::Rate, ["3/2", "3/2"])?];
    for (index, (time, phase)) in [("1", "5/2"), ("2", "4")].into_iter().enumerate() {
        inputs.push(input(Q::Time(index), [time, time])?);
        inputs.push(input(Q::Phase(index), [phase, phase])?);
    }
    let relation = LcRelation::new(
        "separate phase offsets are not shared".into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let clock = LcSharedClock::new(
        relation,
        vec![number("1")?, number("2")?],
        &interval(["0", "0"])?,
        &mut ledger()?,
    )?;
    let physical = clock.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let point = physical.feasibility_witness();
    let source = LcSharedClockPhase::new(
        clock.clone(),
        vec![interval(["2", "2"])?, interval(["4", "4"])?],
        &interval(["-1", "1"])?,
        &mut ledger()?,
    )?;
    assert!(
        source
            .pairs_cover(cycles(), &pi()?, &mut ledger()?)
            .is_err()
    );
    clock.check_witness(
        point.original().values().clone(),
        point.offset().clone(),
        &mut ledger()?,
    )?;
    for offset in ["0", "1/2"] {
        assert!(
            source
                .check_witness(
                    point.original().values().clone(),
                    number("0")?,
                    number(offset)?,
                    &mut ledger()?
                )
                .is_err()
        );
    }
    Ok(())
}

/// Narrow original phase strips compose with either amplitude channel and the same physical owner.
/// ´test:integration:lc-relation-narrower-clock-phase-amplitude´
#[test]
fn lc_relation_narrower_clock_phase_amplitude() -> TestResult {
    let phase = question(true)?;
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        let source = LcSharedMeasurements::new(
            phase.clone(),
            channel,
            vec![interval(["1", "1"])?],
            &interval(["1", "1"])?,
            &mut ledger()?,
        )?;
        let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert_eq!(cover.rate(), &[number("39/20")?, number("41/20")?]);
        let parameters = LcSharedMeasurementParameters {
            rate: number("2")?,
            initial_phase: number("0")?,
            gauge: number("0")?,
            clock_offset: number("1/40")?,
            phase_offset: number("3/20")?,
            measured: number("1")?,
            gain: number("1")?,
            energy: number("1/4")?,
        };
        let point = cover.witness(&parameters, &mut ledger()?)?;
        source.check_witness(
            point.original().original().original().values().clone(),
            parameters.clock_offset,
            parameters.phase_offset,
            parameters.gain,
            &mut ledger()?,
        )?;
    }
    Ok(())
}

/// Exact cumulative Work includes both strip covers, their conditional fibres and original reconstruction.
/// ´test:integration:lc-relation-narrower-clock-phase-work´
#[test]
fn lc_relation_narrower_clock_phase_work() -> TestResult {
    let source = question(true)?;
    let pi = pi()?;
    work_boundary(|budget| source.pairs_cover(cycles(), &pi, budget))?;
    let cover = source.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let parameters = parameters("2", "0", "1/40", "3/20", "0")?;
    work_boundary(|budget| cover.witness(&parameters, budget))?;
    work_boundary(|budget| cover.phase_at_rate(&parameters.physical.rate, budget))?;
    work_boundary(|budget| {
        cover.offset_at_phase(
            &parameters.physical.rate,
            &parameters.physical.initial_phase,
            budget,
        )
    })?;
    work_boundary(|budget| {
        cover.clock_offset_at_phases(
            &parameters.physical.rate,
            &parameters.physical.initial_phase,
            &parameters.offset,
            budget,
        )
    })?;
    Ok(())
}

/// All exact endpoints retain their coherent original quantity meanings.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

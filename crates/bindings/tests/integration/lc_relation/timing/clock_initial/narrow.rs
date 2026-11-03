// SPDX-License-Identifier: AGPL-3.0-only
//! Original timestamp restrictions intersect one shared offset, preserving its declared support.

use super::support::{clock, parameters, work_boundary};
use super::{TestResult, cycles, input, ledger, number, pi};
use nrz_bindings::{
    LcInput, LcInterval, LcPhaseGauge, LcQuantity as Q, LcRelation, LcSharedClock,
    LcSharedClockPhase, LcSharedMeasurementParameters, LcSharedMeasurements,
};

/// Every repeated timestamp declaration contributes to the same common offset intersection.
fn restrictions() -> TestResult<Vec<LcInput>> {
    const FIRST: usize = 0;
    const SECOND: usize = 1;
    Ok(vec![
        input(Q::Time(FIRST), ["19/20", "21/20"])?,
        input(Q::Time(SECOND), ["79/40", "21/10"])?,
        input(Q::Time(FIRST), ["39/40", "43/40"])?,
    ])
}

/// Declared offsets and their exact original-time intersection remain separately queryable.
/// ´test:integration:lc-relation-narrower-clock-timestamps´
#[test]
fn lc_relation_narrower_clock_timestamps() -> TestResult {
    let source = clock(None, ["-1/10", "1/10"], restrictions()?)?;
    assert_eq!(source.offset(), &[number("-1/10")?, number("1/10")?]);
    assert_eq!(
        source.offset_support(),
        &[number("-1/40")?, number("1/20")?]
    );
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert!(cover.clock().shares_allocation(&source));
    assert!(cover.relation().shares_allocation(source.relation()));
    assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::AllIntegers);
    assert_eq!(
        cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-1/10")?, number("1/20")?]
    );
    assert_eq!(
        cover.offset_at_rate(&number("2")?, &mut ledger()?)?,
        *source.offset_support()
    );
    for (phase, offset) in [("1/20", "-1/40"), ("0", "0"), ("-1/10", "1/20")] {
        assert_eq!(
            cover.offset_at_phase(&number("2")?, &number(phase)?, &mut ledger()?)?,
            [number(offset)?, number(offset)?]
        );
        for gauge in ["-2", "0", "3"] {
            let point = cover
                .witness_with_phase(&parameters("2", phase, offset, gauge)?, &mut ledger()?)?;
            source.check_witness(
                point.original().values().clone(),
                point.offset().clone(),
                &mut ledger()?,
            )?;
        }
    }
    assert!(
        cover
            .witness_with_phase(&parameters("2", "-1/5", "1/10", "0")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .witness_with_phase(&parameters("2", "1/20", "1/20", "0")?, &mut ledger()?)
            .is_err()
    );
    let bounded = clock(Some(["0", "1/10"]), ["-1/10", "1/10"], restrictions()?)?.pairs_cover(
        cycles(),
        &pi()?,
        &mut ledger()?,
    )?;
    assert_eq!(
        bounded.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("0")?, number("1/20")?]
    );
    assert_eq!(
        bounded.offset_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-1/40")?, number("0")?]
    );
    Ok(())
}

/// Both shared phase and amplitude owners retain narrower physical timestamps and declared offsets.
/// ´test:integration:lc-relation-narrower-clock-composition´
#[test]
fn lc_relation_narrower_clock_composition() -> TestResult {
    let source = shared_phase()?;
    let phase_cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(phase_cover.rate(), &[number("2")?, number("2")?]);
    assert_eq!(
        phase_cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
        [number("-1/5")?, number("3/20")?]
    );
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        let amplitude = LcSharedMeasurements::new(
            source.clone(),
            channel,
            vec![interval(["1", "1"])?],
            &interval(["1", "1"])?,
            &mut ledger()?,
        )?;
        let cover = amplitude.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert!(cover.measurements().shares_allocation(&amplitude));
        assert_eq!(
            cover.measurements().phase().clock().offset(),
            &[number("-1/10")?, number("1/10")?]
        );
        assert_eq!(
            cover.measurements().phase().clock().offset_support(),
            &[number("-1/40")?, number("1/20")?]
        );
        for (alpha, clock_offset, phase_offset) in
            [("-1/5", "1/20", "-1/10"), ("3/20", "-1/40", "1/10")]
        {
            let parameters = LcSharedMeasurementParameters {
                rate: number("2")?,
                initial_phase: number(alpha)?,
                gauge: number("0")?,
                clock_offset: number(clock_offset)?,
                phase_offset: number(phase_offset)?,
                measured: number("1")?,
                gain: number("1")?,
                energy: number("1/4")?,
            };
            let point = cover.witness(&parameters, &mut ledger()?)?;
            let values = point.original().original().original().values().clone();
            amplitude.check_witness(
                values,
                parameters.clock_offset,
                parameters.phase_offset,
                parameters.gain,
                &mut ledger()?,
            )?;
        }
    }
    Ok(())
}

/// Physical phase boxes contain the shared phase image while physical timestamps remain narrow.
fn shared_phase() -> TestResult<LcSharedClockPhase> {
    let base = clock(None, ["-1/10", "1/10"], restrictions()?)?;
    let mut inputs = base.relation().inputs().to_vec();
    inputs.retain(|value| !matches!(value.quantity(), Q::Phase(_)));
    for (index, range) in [["19/10", "21/10"], ["39/10", "41/10"]]
        .into_iter()
        .enumerate()
    {
        inputs.push(input(Q::Phase(index), range)?);
    }
    let relation = LcRelation::new(
        base.relation().epoch().into(),
        base.relation().observations(),
        inputs,
        &mut ledger()?,
    )?;
    let clock = LcSharedClock::new(
        relation,
        base.nominal_times().to_vec(),
        &interval(["-1/10", "1/10"])?,
        &mut ledger()?,
    )?;
    Ok(LcSharedClockPhase::new(
        clock,
        vec![interval(["2", "2"])?, interval(["4", "4"])?],
        &interval(["-1/10", "1/10"])?,
        &mut ledger()?,
    )?)
}

/// Disjoint originals and cumulative resource exhaustion remain checked against the unchanged owner.
/// ´test:integration:lc-relation-narrower-clock-originals-work´
#[test]
fn lc_relation_narrower_clock_originals_work() -> TestResult {
    const FIRST: usize = 0;
    const SECOND: usize = 1;
    for extra in [
        vec![input(Q::Time(FIRST), ["2", "3"])?],
        vec![
            input(Q::Time(FIRST), ["9/10", "19/20"])?,
            input(Q::Time(SECOND), ["41/20", "21/10"])?,
        ],
    ] {
        assert!(clock(None, ["-1/10", "1/10"], extra).is_err());
    }
    let source = clock(None, ["-1/10", "1/10"], restrictions()?)?;
    let offset = interval(["-1/10", "1/10"])?;
    work_boundary(|budget| {
        LcSharedClock::new(
            source.relation().clone(),
            source.nominal_times().to_vec(),
            &offset,
            budget,
        )
    })?;
    let pi = pi()?;
    work_boundary(|budget| source.pairs_cover(cycles(), &pi, budget))?;
    let cover = source.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    let parameters = parameters("2", "-1/10", "1/20", "0")?;
    work_boundary(|budget| cover.witness_with_phase(&parameters, budget))?;
    work_boundary(|budget| cover.phase_at_rate(&parameters.rate, budget))?;
    work_boundary(|budget| {
        cover.offset_at_phase(&parameters.rate, &parameters.initial_phase, budget)
    })?;
    work_boundary(|budget| cover.forward_horizon(budget))?;
    Ok(())
}

/// Exact intervals retain coherent original quantities and signed offsets.
fn interval(range: [&str; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(number(range[0])?.into()),
        Some(number(range[1])?.into()),
        &mut ledger()?,
    )?)
}

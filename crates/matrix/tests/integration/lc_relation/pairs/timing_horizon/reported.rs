// SPDX-License-Identifier: AGPL-3.0-only
//! Reported-clock horizons retain original physical extremizers and independent native pi enclosures.

use super::support::{both_timing, clock, cycles, finite, render};
use super::{Pi, TestResult, input, ledger, number, pi_certificate};
use nrz_bindings::readings::algebra::Rational;
use nrz_bindings::{
    LcClockPhaseParameters, LcHorizonCoordinate, LcInterval, LcQuantity as Q, LcRelation,
    LcSharedTiming,
};
use std::cmp::Ordering;

/// Reported and actual horizons differ because the future reading shares the original offset.
/// ´test:integration:lc-relation-reported-clock-horizon´
#[test]
fn lc_relation_reported_clock_horizon() -> TestResult {
    let pi = pi_certificate()?;
    let mut fixtures = Vec::new();
    for known in [true, false] {
        let source = clock(known, true)?;
        let cover = source.pairs_cover(cycles(), &pi, &mut ledger()?)?;
        let actual = cover.forward_horizon(&mut ledger()?)?;
        let reported = cover.forward_reported_horizon(&mut ledger()?)?;
        assert_eq!(reported.coordinate(), LcHorizonCoordinate::ReportedClock);
        assert_eq!(reported.mathematical().reference(), &number("11/5")?);
        finite(&reported, "4/3", "100/3")?;
        finite(&actual, if known { "0" } else { "-16/3" }, "100/3")?;
        for point in reported.extremal_parameters() {
            let alpha = cover.phase_at_rate(&point[1], &mut ledger()?)?;
            let parameters = select(point, &alpha, source.offset_support())?;
            let witness = cover.witness_with_phase(&parameters, &mut ledger()?)?;
            source.check_witness(
                witness.original().values().clone(),
                witness.offset().clone(),
                &mut ledger()?,
            )?;
            future_identity(point, &parameters)?;
        }
        fixtures.push((reported, "100/3", "4/3"));
    }
    let oracle = Pi::new()?;
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)
}

/// Both timing layers and narrowed physical times retain reported-coordinate extrema in both formats.
/// ´test:integration:lc-relation-reported-both-timing-horizon´
#[test]
fn lc_relation_reported_both_timing_horizon() -> TestResult {
    let pi = pi_certificate()?;
    let mut fixtures = Vec::new();
    for (known, narrow, reference, offset, coefficient) in [
        (true, false, "111/50", "4/3", "9991/600"),
        (false, false, "111/50", "4/3", "9991/600"),
        (true, true, "203/100", "-1", "39999/400"),
        (false, true, "203/100", "1", "2499/100"),
    ] {
        let source = question(known, narrow)?;
        let cover = source.pairs_cover(cycles(), &pi, &mut ledger()?)?;
        let reported = cover.forward_reported_horizon(&mut ledger()?)?;
        assert_eq!(reported.coordinate(), LcHorizonCoordinate::ReportedClock);
        assert_eq!(reported.mathematical().reference(), &number(reference)?);
        assert!(
            reported
                .shared_timing_cover()
                .ok_or("original two-layer source")?
                .timing()
                .shares_allocation(&source)
        );
        finite(&reported, offset, coefficient)?;
        for point in reported.extremal_parameters() {
            let alpha = cover.phase_at_rate(&point[1], &mut ledger()?)?;
            let parameters = select(point, &alpha, source.offset_support())?;
            let witness = cover.witness_with_phase(&parameters, &mut ledger()?)?;
            source.check_witness(
                witness.original().values().clone(),
                witness.offset().clone(),
                &mut ledger()?,
            )?;
            future_identity(point, &parameters)?;
        }
        fixtures.push((reported, coefficient, offset));
    }
    let oracle = Pi::new()?;
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)
}

/// Select physical alpha from its conditional intersection at one reported intercept beta.
fn select(
    point: &[Rational; 2],
    alpha: &[Rational; 2],
    offset: &[Rational; 2],
) -> TestResult<LcClockPhaseParameters> {
    let mut lower = point[0].sub(&point[1].mul(&offset[1], &mut ledger()?)?, &mut ledger()?)?;
    let mut upper = point[0].sub(&point[1].mul(&offset[0], &mut ledger()?)?, &mut ledger()?)?;
    if alpha[0].compare(&lower, &mut ledger()?)? == Ordering::Greater {
        lower.clone_from(&alpha[0]);
    }
    if alpha[1].compare(&upper, &mut ledger()?)? == Ordering::Less {
        upper.clone_from(&alpha[1]);
    }
    assert_ne!(lower.compare(&upper, &mut ledger()?)?, Ordering::Greater);
    let clock = point[0]
        .sub(&lower, &mut ledger()?)?
        .div(&point[1], &mut ledger()?)?;
    Ok(LcClockPhaseParameters {
        rate: point[1].clone(),
        initial_phase: lower,
        gauge: number("0")?,
        offset: clock,
        ratio: number("1")?,
        voltage: number("1")?,
    })
}

/// The same physical oscillator realizes alpha+a*(r+d), not an independent future clock box.
fn future_identity(point: &[Rational; 2], parameters: &LcClockPhaseParameters) -> TestResult {
    for reading in ["3", "10"] {
        let reading = number(reading)?;
        let time = reading.add(&parameters.offset, &mut ledger()?)?;
        let actual = parameters
            .initial_phase
            .add(&parameters.rate.mul(&time, &mut ledger()?)?, &mut ledger()?)?;
        let reported = point[0].add(&point[1].mul(&reading, &mut ledger()?)?, &mut ledger()?)?;
        assert_eq!(actual, reported);
    }
    Ok(())
}

/// Keep original shared and independent timing layers, optionally narrowing original physical timestamps.
fn question(known: bool, narrow: bool) -> TestResult<LcSharedTiming> {
    let base = both_timing()?;
    let mut inputs = base.relation().inputs().to_vec();
    if !known {
        inputs.retain(|value| value.quantity() != Q::InitialPhase);
    }
    if narrow {
        inputs.retain(|value| !matches!(value.quantity(), Q::Time(_)));
        for (index, time) in [["99/100", "101/100"], ["199/100", "201/100"]]
            .into_iter()
            .enumerate()
        {
            inputs.push(input(Q::Time(index), time)?);
        }
    }
    let relation = LcRelation::new(
        base.relation().epoch().into(),
        base.relation().observations(),
        inputs,
        &mut ledger()?,
    )?;
    let errors = base
        .errors()
        .iter()
        .map(interval)
        .collect::<TestResult<Vec<_>>>()?;
    Ok(LcSharedTiming::new(
        relation,
        base.nominal_times().to_vec(),
        &errors,
        &interval(base.offset())?,
        &mut ledger()?,
    )?)
}

/// Retain each original coherent signed error support exactly.
fn interval(range: &[Rational; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(range[0].clone().into()),
        Some(range[1].clone().into()),
        &mut ledger()?,
    )?)
}

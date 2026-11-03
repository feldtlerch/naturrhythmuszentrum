// SPDX-License-Identifier: AGPL-3.0-only
//! Unknown physical phase with both time layers retains the actual-time horizon correlation.

use super::support::{both_timing, cycles, finite, render, work_boundary};
use super::{Pi, TestResult, ledger, number, pi_certificate};
use nrz_bindings::{
    LcClockPhaseParameters, LcHorizon, LcInterval, LcPhaseGauge, LcQuantity as Q, LcRelation,
    LcSharedTiming, LcSharedTimingCover,
};
use std::cmp::Ordering;

/// Both formats and restored physical extrema use alpha instead of the auxiliary clock intercept.
/// ´test:integration:lc-relation-both-timing-initial-horizon´
#[test]
fn lc_relation_both_timing_initial_horizon() -> TestResult {
    let source = question()?;
    let cover = source.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
    let horizon = cover.forward_horizon(&mut ledger()?)?;
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::AllIntegers);
    assert_eq!(horizon.mathematical().reference(), &number("53/25")?);
    assert_eq!(
        horizon.mathematical().width(),
        &[number("2400/9991")?, number("1200/9991")?]
    );
    finite(&horizon, "-2", "9991/600")?;
    assert!(
        horizon
            .shared_timing_cover()
            .ok_or("retained original two-layer source")?
            .timing()
            .shares_allocation(&source)
    );
    assert!(horizon.relation().shares_allocation(source.relation()));
    extrema(&cover, &horizon)?;
    work_boundary(|budget| cover.forward_horizon(budget))?;
    let fixtures = [(horizon, "9991/600", "-2")];
    let oracle = Pi::new()?;
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)
}

/// Remove only physical alpha while keeping the same original timing layer declarations.
fn question() -> TestResult<LcSharedTiming> {
    let known = both_timing()?;
    let mut inputs = known.relation().inputs().to_vec();
    inputs.retain(|value| value.quantity() != Q::InitialPhase);
    let relation = LcRelation::new(
        known.relation().epoch().into(),
        known.relation().observations(),
        inputs,
        &mut ledger()?,
    )?;
    let errors = known
        .errors()
        .iter()
        .map(interval)
        .collect::<TestResult<Vec<_>>>()?;
    let source = LcSharedTiming::new(
        relation,
        known.nominal_times().to_vec(),
        &errors,
        &interval(known.offset())?,
        &mut ledger()?,
    )?;
    Ok(source)
}

/// Restored endpoint witnesses and interior fibres agree with the physical future extremizers.
fn extrema(cover: &LcSharedTimingCover, horizon: &LcHorizon) -> TestResult {
    for extrema in horizon.extremal_parameters() {
        let offset = cover.offset_at_phase(&extrema[1], &extrema[0], &mut ledger()?)?[0].clone();
        let point = cover.witness_with_phase(
            &LcClockPhaseParameters {
                rate: extrema[1].clone(),
                initial_phase: extrema[0].clone(),
                offset,
                gauge: number("0")?,
                ratio: number("1")?,
                voltage: number("1")?,
            },
            &mut ledger()?,
        )?;
        cover.timing().check_witness(
            point.original().values().clone(),
            point.offset().clone(),
            &mut ledger()?,
        )?;
    }
    for time in ["53/25", "3", "10"] {
        let time = number(time)?;
        let extrema = horizon.extremal_parameters();
        let lower =
            extrema[0][0].add(&extrema[0][1].mul(&time, &mut ledger()?)?, &mut ledger()?)?;
        let upper =
            extrema[1][0].add(&extrema[1][1].mul(&time, &mut ledger()?)?, &mut ledger()?)?;
        for rate in [&cover.rate()[0], &number("2")?, &cover.rate()[1]] {
            for alpha in cover.phase_at_rate(rate, &mut ledger()?)? {
                let future = alpha.add(&rate.mul(&time, &mut ledger()?)?, &mut ledger()?)?;
                assert_ne!(future.compare(&lower, &mut ledger()?)?, Ordering::Less);
                assert_ne!(future.compare(&upper, &mut ledger()?)?, Ordering::Greater);
            }
        }
    }
    Ok(())
}

/// Original error supports are copied exactly without any independent total-box replacement.
fn interval(range: &[nrz_bindings::readings::algebra::Rational; 2]) -> TestResult<LcInterval> {
    Ok(LcInterval::new(
        Some(range[0].clone().into()),
        Some(range[1].clone().into()),
        &mut ledger()?,
    )?)
}

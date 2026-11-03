// SPDX-License-Identifier: AGPL-3.0-only
//! Timing horizons retain original joint owners and an explicit future actual-time coordinate.

mod refinement;
mod shared_initial;
mod support;

use super::super::support::{TestResult, input, ledger, number, pi_certificate};
use super::super::time::oracle::Pi;
use nrz_bindings::{LcClockParameters, LcHorizonValue, LcPairParameters, LcPhaseGauge};
use support::{clock, cycles, finite, render, timing, work_boundary};

/// Independent timing horizons preserve physical initial-phase fibres, gauges and both numerical formats.
/// ´test:integration:lc-relation-timing-horizon´
#[test]
fn lc_relation_timing_horizon() -> TestResult {
    let mut horizons = Vec::new();
    for (known, width, offset, coefficient) in [
        (true, ["0", "40/99"], "0", "99/20"),
        (false, ["-13/7", "10/7"], "13/10", "7/5"),
    ] {
        let source = timing(known, ["9/10", "11/10"])?;
        let cover = source.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
        let horizon = cover.forward_horizon(&mut ledger()?)?;
        assert!(horizon.relation().shares_allocation(source.relation()));
        assert!(
            horizon
                .timing_cover()
                .ok_or("original timing source")?
                .timing()
                .shares_allocation(&source)
        );
        let gauge = if known {
            LcPhaseGauge::Fixed
        } else {
            LcPhaseGauge::AllIntegers
        };
        assert_eq!(
            horizon
                .timing_cover()
                .ok_or("original gauge source")?
                .phase_gauge(),
            gauge
        );
        assert!(horizon.pairs_cover().is_none());
        assert_eq!(horizon.mathematical().reference(), &number("23/10")?);
        assert_eq!(
            horizon.mathematical().width(),
            &[number(width[0])?, number(width[1])?]
        );
        finite(&horizon, offset, coefficient)?;
        for point in horizon.extremal_parameters() {
            let witness = cover.witness(
                &LcPairParameters {
                    rate: point[1].clone(),
                    initial_phase: point[0].clone(),
                    gauge: number("0")?,
                    ratio: number("1")?,
                    voltage: number("1")?,
                },
                &mut ledger()?,
            )?;
            assert!(witness.relation().shares_allocation(horizon.relation()));
            assert_eq!(witness.values().observations.len(), 2);
        }
        support::timing_future_extrema(&cover, &horizon)?;
        horizons.push((horizon, coefficient, offset));
    }
    let oracle = Pi::new()?;
    render::<f32>(&horizons, &oracle)?;
    render::<f64>(&horizons, &oracle)
}

/// Shared-clock horizons use actual time and retain the original physical alpha/offset correlation.
/// ´test:integration:lc-relation-clock-actual-time-horizon´
#[test]
fn lc_relation_clock_actual_time_horizon() -> TestResult {
    let mut horizons = Vec::new();
    for (known, expected_phase, offset, coefficient) in [
        (true, ["0", "0"], "0", "100/3"),
        (false, ["-157/1000", "163/1000"], "-16/3", "100/3"),
    ] {
        let source = clock(known, true)?;
        let cover = source.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
        let horizon = cover.forward_horizon(&mut ledger()?)?;
        assert!(horizon.relation().shares_allocation(source.relation()));
        assert!(
            horizon
                .clock_cover()
                .ok_or("original clock source")?
                .clock()
                .shares_allocation(&source)
        );
        let gauge = if known {
            LcPhaseGauge::Fixed
        } else {
            LcPhaseGauge::AllIntegers
        };
        assert_eq!(
            horizon
                .clock_cover()
                .ok_or("original gauge source")?
                .phase_gauge(),
            gauge
        );
        assert!(horizon.pairs_cover().is_none());
        assert_eq!(horizon.mathematical().reference(), &number("21/10")?);
        finite(&horizon, offset, coefficient)?;
        let extrema = horizon.extremal_parameters();
        assert_eq!(extrema[0][0], number(expected_phase[0])?);
        assert_eq!(extrema[1][0], number(expected_phase[1])?);
        support::clock_extrema(&cover, &horizon)?;
        horizons.push((horizon, coefficient, offset));
    }
    let oracle = Pi::new()?;
    render::<f32>(&horizons, &oracle)?;
    render::<f64>(&horizons, &oracle)
}

/// Both timing layers retain their joint original source, and zero spread remains explicit infinity.
/// ´test:integration:lc-relation-shared-timing-horizon´
#[test]
fn lc_relation_shared_timing_horizon() -> TestResult {
    let source = support::both_timing()?;
    let cover = source.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
    let horizon = cover.forward_horizon(&mut ledger()?)?;
    finite(&horizon, "0", "9991/600")?;
    assert_eq!(horizon.mathematical().reference(), &number("53/25")?);
    assert!(
        horizon
            .shared_timing_cover()
            .ok_or("both timing source")?
            .timing()
            .shares_allocation(&source)
    );
    assert!(horizon.relation().shares_allocation(source.relation()));
    for extrema in horizon.extremal_parameters() {
        let offset = cover.offset_at_rate(&extrema[1], &mut ledger()?)?[0].clone();
        let point = cover.witness(
            &LcClockParameters {
                rate: extrema[1].clone(),
                offset,
                ratio: number("1")?,
                voltage: number("1")?,
            },
            &mut ledger()?,
        )?;
        source.check_witness(
            point.original().values().clone(),
            point.offset().clone(),
            &mut ledger()?,
        )?;
    }
    let exact = clock(true, false)?
        .pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?
        .forward_horizon(&mut ledger()?)?;
    assert!(matches!(exact.value(), LcHorizonValue::Infinite));
    let unknown = clock(false, false)?
        .pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?
        .forward_horizon(&mut ledger()?)?;
    assert_eq!(
        unknown.mathematical().width(),
        &[number("2/5")?, number("0")?]
    );
    assert!(matches!(unknown.value(), LcHorizonValue::Infinite));
    let fixtures = [
        (horizon, "9991/600", "0"),
        (exact, "", ""),
        (unknown, "", ""),
    ];
    let oracle = Pi::new()?;
    render::<f32>(&fixtures, &oracle)?;
    render::<f64>(&fixtures, &oracle)
}

/// Future references cannot precede observations, and each horizon query spends cumulative caller Work.
/// ´test:integration:lc-relation-timing-horizon-refusals-work´
#[test]
fn lc_relation_timing_horizon_refusals_work() -> TestResult {
    let timing = timing(false, ["9/10", "11/10"])?;
    let cover = timing.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
    assert!(
        cover
            .forward_horizon_from(&number("2")?, &mut ledger()?)
            .is_err()
    );
    let reference = number("3")?;
    let mut measured = ledger()?;
    let horizon = cover.forward_horizon_from(&reference, &mut measured)?;
    assert_eq!(horizon.mathematical().reference(), &reference);
    assert_eq!(
        horizon.usage()[1].work - horizon.usage()[0].work,
        measured.usage().work
    );
    work_boundary(|budget| cover.forward_horizon(budget))?;
    work_boundary(|budget| cover.forward_horizon_from(&reference, budget))?;
    let clock = clock(false, true)?.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
    assert!(
        clock
            .forward_horizon_from(&number("2")?, &mut ledger()?)
            .is_err()
    );
    work_boundary(|budget| clock.forward_horizon(budget))?;
    work_boundary(|budget| clock.forward_horizon_from(&reference, budget))?;
    let both = support::both_timing()?.pairs_cover(cycles(), &pi_certificate()?, &mut ledger()?)?;
    work_boundary(|budget| both.forward_horizon(budget))?;
    Ok(())
}

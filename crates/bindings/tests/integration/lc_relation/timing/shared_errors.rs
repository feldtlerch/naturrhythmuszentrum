// SPDX-License-Identifier: AGPL-3.0-only
//! Every retained shared error is checked on one original physical witness.

mod narrow;
mod support;

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{LcPhaseGauge, LcQuantity as Q, LcSharedMeasurements};
use support::{
    assert_bounds, check_physical, interval, parameters, question, source, work_boundary,
};

/// Both amplitude layers and each layer alone compose with the same shared clock and phase errors.
/// ´test:integration:lc-relation-three-shared-errors-layers´
#[test]
fn lc_relation_three_shared_errors_layers() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        for (readings, gain, expected) in [
            (true, true, ["171/200", "121/100"]),
            (true, false, ["19/20", "11/10"]),
            (false, true, ["9/10", "11/10"]),
        ] {
            let source = question(channel, [readings, gain, true, true])?;
            let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
            assert_bounds(&cover.bounds(channel, &mut ledger()?)?, expected)?;
            assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
            assert!(cover.measurements().shares_allocation(&source));
            assert!(cover.relation().shares_allocation(source.relation()));
            assert_eq!(cover.cycles(), cycles());
            let point = cover.feasibility_witness();
            assert!(point.measurements().shares_allocation(&source));
            assert!(point.original().phase().shares_allocation(source.phase()));
            assert!(
                point
                    .original()
                    .original()
                    .clock()
                    .shares_allocation(source.phase().clock())
            );
            check_physical(&source, point)?;
        }
    }
    Ok(())
}

/// Crossing any one shared-error coordinate refuses despite membership in separate marginals.
/// ´test:integration:lc-relation-three-shared-errors-correlations´
#[test]
fn lc_relation_three_shared_errors_correlations() -> TestResult {
    let source = question(Q::VoltageAmplitude, [true, true, true, true])?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    for (clock, phase) in [("-1/10", "-1/5"), ("0", "0"), ("1/10", "1/5")] {
        let coordinates = parameters("0", clock, phase, "19/20", "9/10", "0")?;
        let point = cover.witness(&coordinates, &mut ledger()?)?;
        check_physical(&source, &point)?;
        assert_eq!(
            cover.clock_offset_at_phases(
                &number("2")?,
                &number("0")?,
                &number(phase)?,
                &mut ledger()?
            )?,
            [number(clock)?, number(clock)?]
        );
        assert_eq!(
            cover.gain_at_amplitude(&number("171/200")?, &mut ledger()?)?,
            [number("9/10")?, number("9/10")?]
        );
        assert!(
            source
                .check_witness(
                    point.original().original().original().values().clone(),
                    number(clock)?,
                    number(phase)?,
                    number("11/10")?,
                    &mut ledger()?
                )
                .is_err()
        );
    }
    assert!(
        cover
            .witness(
                &parameters("0", "-1/10", "1/5", "1", "1", "0")?,
                &mut ledger()?
            )
            .is_err()
    );
    assert_eq!(
        cover.phase_offset_at_phase(&number("2")?, &number("0")?, &mut ledger()?)?,
        [number("-1/5")?, number("1/5")?]
    );
    Ok(())
}

/// Marginal amplitude boxes cannot replace one gain while the clock and phase question is retained.
/// ´test:integration:lc-relation-three-shared-errors-total-box-control´
#[test]
fn lc_relation_three_shared_errors_total_box_control() -> TestResult {
    let phase = source(Q::VoltageAmplitude, true, true, Vec::new())?;
    let total = LcSharedMeasurements::new(
        phase.clone(),
        Q::VoltageAmplitude,
        vec![interval(["1", "2"])?, interval(["2", "4"])?],
        &interval(["1", "1"])?,
        &mut ledger()?,
    )?;
    let cover = total.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_bounds(
        &cover.bounds(Q::VoltageAmplitude, &mut ledger()?)?,
        ["2", "2"],
    )?;
    check_physical(&total, cover.feasibility_witness())?;
    assert!(
        LcSharedMeasurements::new(
            phase,
            Q::VoltageAmplitude,
            vec![interval(["1", "1"])?, interval(["2", "2"])?],
            &interval(["1", "2"])?,
            &mut ledger()?
        )
        .is_err()
    );
    Ok(())
}

/// Exact physical extrema and unknown initial phase retain every original common integer gauge.
/// ´test:integration:lc-relation-three-shared-errors-physical-gauges´
#[test]
fn lc_relation_three_shared_errors_physical_gauges() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        let source = question(channel, [true, true, false, true])?;
        let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert_eq!(cover.phase_gauge(), LcPhaseGauge::AllIntegers);
        assert_eq!(
            cover.phase_at_rate(&number("2")?, &mut ledger()?)?,
            [number("-2/5")?, number("2/5")?]
        );
        let (direct, inverse) = if channel == Q::VoltageAmplitude {
            (Q::Inductance, Q::Capacitance)
        } else {
            (Q::Capacitance, Q::Inductance)
        };
        assert_bounds(
            &cover.bounds(direct, &mut ledger()?)?,
            ["29241/80000", "14641/20000"],
        )?;
        assert_bounds(
            &cover.bounds(inverse, &mut ledger()?)?,
            ["5000/14641", "20000/29241"],
        )?;
        for (alpha, clock, phase) in [("-2/5", "1/10", "-1/5"), ("2/5", "-1/10", "1/5")] {
            for (measured, gain) in [("19/20", "9/10"), ("11/10", "11/10")] {
                for gauge in ["-2", "0", "3"] {
                    let point = cover.witness(
                        &parameters(alpha, clock, phase, measured, gain, gauge)?,
                        &mut ledger()?,
                    )?;
                    check_physical(&source, &point)?;
                }
            }
        }
    }
    Ok(())
}

/// Free energy, original scale refusals, complete cycle evidence and cumulative Work remain explicit.
/// ´test:integration:lc-relation-three-shared-errors-refusals-work´
#[test]
fn lc_relation_three_shared_errors_refusals_work() -> TestResult {
    let free = question(Q::VoltageAmplitude, [true, true, true, false])?;
    let free_cover = free.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        free_cover.free_directions(Q::Energy)?,
        &[nrz_bindings::LcFreeDirection::ElementScale]
    );
    assert!(
        free_cover
            .bounds(Q::Energy, &mut ledger()?)?
            .upper()
            .is_none()
    );
    let source = question(Q::VoltageAmplitude, [true, true, true, true])?;
    let pi = pi()?;
    assert!(
        source
            .pairs_cover(vec![number("0")?, number("1")?], &pi, &mut ledger()?)
            .is_err()
    );
    let cover = source.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    work_boundary(|budget| source.pairs_cover(cycles(), &pi, budget))?;
    let coordinates = parameters("0", "1/10", "1/5", "19/20", "9/10", "0")?;
    work_boundary(|budget| cover.witness(&coordinates, budget))?;
    let magnitude = number("171/200")?;
    work_boundary(|budget| cover.gain_at_amplitude(&magnitude, budget))?;
    work_boundary(|budget| cover.bounds(Q::Capacitance, budget))?;
    support::original_refusals()
}

// SPDX-License-Identifier: AGPL-3.0-only
//! Inductance calibration controls retain all physical equations and original timing uncertainty.

mod capacitance;
mod support;

use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{
    LcCertainty, LcConsistency, LcDeterminacy, LcElementSamples, LcFreeDirection, LcPhaseGauge,
    LcQuantity as Q,
};
use support::{assert_bounds, interval, measured, parameters, readings, timing, work_boundary};

/// Independent readings and a common inductance calibration have distinct separate-layer controls.
/// ´test:integration:lc-relation-inductance-both-layers´
#[test]
fn lc_relation_inductance_both_layers() -> TestResult {
    for (per_sample, shared, expected) in [
        (true, true, ["171/200", "121/100"]),
        (true, false, ["19/20", "11/10"]),
        (false, true, ["9/10", "11/10"]),
    ] {
        let timing = timing(false, true, Vec::new())?;
        let element = measured(timing.clone(), per_sample, shared)?;
        assert_eq!(element.samples().len(), 2);
        assert!(element.timing().shares_allocation(&timing));
        let cover = element.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert_bounds(&cover.bounds(Q::Inductance, &mut ledger()?)?, expected)?;
        assert_eq!(cover.rate(), &[number("2")?, number("2")?]);
        assert_eq!(cover.cycles(), cycles());
        assert_eq!(cover.phase_gauge(), LcPhaseGauge::Fixed);
        assert_eq!(cover.certainty(), LcCertainty::Certified);
        assert_eq!(cover.consistency(), LcConsistency::Feasible);
        assert!(cover.element().shares_allocation(&element));
        assert!(cover.relation().shares_allocation(timing.relation()));
        let point = cover.feasibility_witness();
        assert!(point.element().shares_allocation(&element));
        assert!(
            point
                .original()
                .relation()
                .shares_allocation(timing.relation())
        );
        element.check_witness(
            point.original().values().clone(),
            point.calibration().clone(),
            &mut ledger()?,
        )?;
    }
    Ok(())
}

/// Marginal total boxes and crossed element/calibration endpoints cannot replace one shared gain.
/// ´test:integration:lc-relation-inductance-shared-control´
#[test]
fn lc_relation_inductance_shared_control() -> TestResult {
    let timing = timing(false, true, Vec::new())?;
    let element = measured(timing.clone(), true, true)?;
    let cover = element.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        cover.calibration_at_element(&number("171/200")?, &mut ledger()?)?,
        [number("9/10")?, number("9/10")?]
    );
    let parameters = parameters("2", "0", "171/200", "9/10", "1", "0")?;
    let point = cover.witness(&parameters, &mut ledger()?)?;
    assert!(
        element
            .check_witness(
                point.original().values().clone(),
                number("11/10")?,
                &mut ledger()?
            )
            .is_err()
    );
    // Marginal total boxes [1,2] and [2,4] overlap at L=2 with distinct gains 2 and 1.
    let marginal = LcElementSamples::new(
        timing.clone(),
        Q::Inductance,
        vec![interval(["1", "2"])?, interval(["2", "4"])?],
        &interval(["1", "1"])?,
        &mut ledger()?,
    )?;
    let accepted = marginal.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_bounds(&accepted.bounds(Q::Inductance, &mut ledger()?)?, ["2", "2"])?;
    assert_eq!(
        number("2")?.div(&number("1")?, &mut ledger()?)?,
        number("2")?
    );
    assert_eq!(
        number("2")?.div(&number("2")?, &mut ledger()?)?,
        number("1")?
    );
    assert!(
        LcElementSamples::new(
            timing,
            Q::Inductance,
            vec![interval(["1", "1"])?, interval(["2", "2"])?],
            &interval(["1", "2"])?,
            &mut ledger()?
        )
        .is_err()
    );
    Ok(())
}

/// Both elements and impedance have exact rational extrema while amplitude and energy remain free.
/// ´test:integration:lc-relation-inductance-scale-oracles´
#[test]
fn lc_relation_inductance_scale_oracles() -> TestResult {
    let element = measured(timing(false, true, Vec::new())?, true, true)?;
    let cover = element.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    for (quantity, expected) in [
        (Q::Inductance, ["171/200", "121/100"]),
        (Q::Capacitance, ["25/121", "50/171"]),
        (Q::AmplitudeRatio, ["171/100", "121/50"]),
    ] {
        assert_bounds(&cover.bounds(quantity, &mut ledger()?)?, expected)?;
        assert_eq!(
            cover.determinacy(quantity)?,
            LcDeterminacy::DeterminedFinite
        );
        assert_eq!(cover.free_directions(quantity)?, [] as [LcFreeDirection; 0]);
    }
    for quantity in [Q::VoltageAmplitude, Q::CurrentAmplitude, Q::Energy] {
        assert_eq!(cover.determinacy(quantity)?, LcDeterminacy::Underdetermined);
        assert_eq!(
            cover.free_directions(quantity)?,
            &[LcFreeDirection::AmplitudeScale]
        );
        let range = cover.bounds(quantity, &mut ledger()?)?;
        assert_eq!(range.lower(), Some(&number("0")?.into()));
        assert!(range.upper().is_none());
    }
    for (inductance, gain) in [("171/200", "9/10"), ("1", "1"), ("121/100", "11/10")] {
        for voltage in ["1/10", "1", "100"] {
            let point = cover.witness(
                &parameters("2", "0", inductance, gain, voltage, "0")?,
                &mut ledger()?,
            )?;
            support::check_physical(&element, &point)?;
        }
    }
    Ok(())
}

/// Unknown physical phase and unequal timestamp intervals preserve joint fibres and all gauges.
/// ´test:integration:lc-relation-inductance-timing-phase´
#[test]
fn lc_relation_inductance_timing_phase() -> TestResult {
    let element = measured(timing(true, false, Vec::new())?, true, true)?;
    let cover = element.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.rate(), &[number("10/7")?, number("20/7")?]);
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::AllIntegers);
    for (rate, phase) in [("10/7", "5/7"), ("2", "0"), ("20/7", "-8/7")] {
        for (inductance, gain) in [("171/200", "9/10"), ("121/100", "11/10")] {
            for gauge in ["-2", "0", "3"] {
                let point = cover.witness(
                    &parameters(rate, phase, inductance, gain, "1", gauge)?,
                    &mut ledger()?,
                )?;
                support::check_physical(&element, &point)?;
            }
        }
    }
    assert!(
        cover
            .witness(
                &parameters("10/7", "-8/7", "1", "1", "1", "0")?,
                &mut ledger()?
            )
            .is_err()
    );
    assert_bounds(
        &cover.bounds(Q::Capacitance, &mut ledger()?)?,
        ["49/484", "98/171"],
    )?;
    assert_bounds(
        &cover.bounds(Q::AmplitudeRatio, &mut ledger()?)?,
        ["171/140", "121/35"],
    )?;
    Ok(())
}

/// Repeated original inductance restrictions intersect jointly with the measured product image.
/// ´test:integration:lc-relation-inductance-original-intersection´
#[test]
fn lc_relation_inductance_original_intersection() -> TestResult {
    let original = timing(
        false,
        true,
        vec![
            input(Q::Inductance, ["9/10", "6/5"])?,
            input(Q::Inductance, ["1", "11/10"])?,
        ],
    )?;
    let element = measured(original.clone(), true, true)?;
    assert_eq!(element.values(), &[number("1")?, number("11/10")?]);
    let cover = element.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert!(cover.relation().shares_allocation(original.relation()));
    for value in ["1", "21/20", "11/10"] {
        let point = cover.witness(&parameters("2", "0", value, "1", "1", "0")?, &mut ledger()?)?;
        support::check_physical(&element, &point)?;
    }
    assert!(
        cover
            .calibration_at_element(&number("171/200")?, &mut ledger()?)
            .is_err()
    );
    assert!(
        cover
            .witness(
                &parameters("2", "0", "171/200", "9/10", "1", "0")?,
                &mut ledger()?
            )
            .is_err()
    );
    assert!(
        measured(
            timing(false, true, vec![input(Q::Inductance, ["2", "3"])?])?,
            true,
            true
        )
        .is_err()
    );
    Ok(())
}

/// Unsupported scales, nonpositive readings, incomplete branches and cumulative Work refuse.
/// ´test:integration:lc-relation-inductance-refusals-work´
#[test]
fn lc_relation_inductance_refusals_work() -> TestResult {
    for quantity in [
        Q::Capacitance,
        Q::VoltageAmplitude,
        Q::Energy,
        Q::AmplitudeRatio,
    ] {
        assert!(
            measured(
                timing(false, true, vec![input(quantity, ["1", "1"])?])?,
                true,
                true
            )
            .is_err()
        );
    }
    let timing = timing(false, true, Vec::new())?;
    for bad in [
        interval(["0", "1"])?,
        interval(["-1", "1"])?,
        nrz_bindings::LcInterval::new(Some(number("1")?.into()), None, &mut ledger()?)?,
    ] {
        assert!(
            LcElementSamples::new(
                timing.clone(),
                Q::Inductance,
                vec![bad.clone()],
                &interval(["1", "1"])?,
                &mut ledger()?
            )
            .is_err()
        );
        assert!(
            LcElementSamples::new(
                timing.clone(),
                Q::Inductance,
                readings(true)?,
                &bad,
                &mut ledger()?
            )
            .is_err()
        );
    }
    assert!(
        LcElementSamples::new(
            timing.clone(),
            Q::Inductance,
            Vec::new(),
            &interval(["1", "1"])?,
            &mut ledger()?
        )
        .is_err()
    );
    let samples = readings(true)?;
    let calibration = interval(["9/10", "11/10"])?;
    work_boundary(|budget| {
        LcElementSamples::new(
            timing.clone(),
            Q::Inductance,
            samples.clone(),
            &calibration,
            budget,
        )
    })?;
    let element = measured(timing, true, true)?;
    let pi = pi()?;
    assert!(
        element
            .pairs_cover(vec![number("1")?, number("1")?], &pi, &mut ledger()?)
            .is_err()
    );
    let mut measured = ledger()?;
    let cover = element.pairs_cover(cycles(), &pi, &mut measured)?;
    assert_eq!(
        cover.usage()[1].work - cover.usage()[0].work,
        measured.usage().work
    );
    assert!(cover.retained_payload_bytes() > element.retained_payload_bytes());
    work_boundary(|budget| element.pairs_cover(cycles(), &pi, budget))?;
    let point = parameters("2", "0", "1", "1", "1", "0")?;
    work_boundary(|budget| cover.witness(&point, budget))?;
    work_boundary(|budget| cover.calibration_at_element(&point.element, budget))?;
    work_boundary(|budget| cover.bounds(Q::Capacitance, budget))?;
    Ok(())
}

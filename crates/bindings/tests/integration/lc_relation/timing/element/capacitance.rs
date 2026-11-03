// SPDX-License-Identifier: AGPL-3.0-only
//! Capacitance has its own calibration controls and original physical reconstruction evidence.

use super::support::{
    assert_bounds, check_physical, interval, parameters, readings, timing, work_boundary,
};
use super::{TestResult, cycles, input, ledger, number, pi};
use nrz_bindings::{
    LcDeterminacy, LcElementSamples, LcFreeDirection, LcPhaseGauge, LcQuantity as Q, LcSampleTimes,
};

/// Both capacitance layers and each separate layer retain every original reading and owner.
/// ´test:integration:lc-relation-capacitance-both-layers´
#[test]
fn lc_relation_capacitance_both_layers() -> TestResult {
    for (per_sample, shared, expected) in [
        (true, true, ["171/200", "121/100"]),
        (true, false, ["19/20", "11/10"]),
        (false, true, ["9/10", "11/10"]),
    ] {
        let timing = timing(false, true, Vec::new())?;
        let element = calibrated(timing.clone(), per_sample, shared)?;
        let cover = element.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert_bounds(&cover.bounds(Q::Capacitance, &mut ledger()?)?, expected)?;
        assert_eq!(element.element(), Q::Capacitance);
        assert_eq!(element.samples().len(), 2);
        assert!(element.timing().shares_allocation(&timing));
        assert!(cover.element().shares_allocation(&element));
        assert!(cover.relation().shares_allocation(timing.relation()));
        assert_eq!(cover.cycles(), cycles());
        let point = cover.feasibility_witness();
        assert!(
            point
                .original()
                .relation()
                .shares_allocation(timing.relation())
        );
        check_physical(&element, point)?;
    }
    Ok(())
}

/// Independent total boxes and crossed capacitance/calibration marginals differ from a common gain.
/// ´test:integration:lc-relation-capacitance-calibration-shared-control´
#[test]
fn lc_relation_capacitance_calibration_shared_control() -> TestResult {
    let timing = timing(false, true, Vec::new())?;
    let element = calibrated(timing.clone(), true, true)?;
    let cover = element.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(
        cover.calibration_at_element(&number("171/200")?, &mut ledger()?)?,
        [number("9/10")?, number("9/10")?]
    );
    assert_eq!(
        cover.calibration_at_element(&number("121/100")?, &mut ledger()?)?,
        [number("11/10")?, number("11/10")?]
    );
    let point = cover.witness(
        &parameters("2", "0", "171/200", "9/10", "1", "0")?,
        &mut ledger()?,
    )?;
    check_physical(&element, &point)?;
    assert!(
        element
            .check_witness(
                point.original().values().clone(),
                number("11/10")?,
                &mut ledger()?
            )
            .is_err()
    );
    let marginal = LcElementSamples::new(
        timing.clone(),
        Q::Capacitance,
        vec![interval(["1", "2"])?, interval(["2", "4"])?],
        &interval(["1", "1"])?,
        &mut ledger()?,
    )?;
    let admitted = marginal.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_bounds(
        &admitted.bounds(Q::Capacitance, &mut ledger()?)?,
        ["2", "2"],
    )?;
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
            Q::Capacitance,
            vec![interval(["1", "1"])?, interval(["2", "2"])?],
            &interval(["1", "2"])?,
            &mut ledger()?
        )
        .is_err()
    );
    Ok(())
}

/// Exact capacitance maps agree with independent physical equations and retain free amplitude scale.
/// ´test:integration:lc-relation-capacitance-calibration-scale-oracles´
#[test]
fn lc_relation_capacitance_calibration_scale_oracles() -> TestResult {
    let element = calibrated(timing(false, true, Vec::new())?, true, true)?;
    let cover = element.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    for (quantity, expected) in [
        (Q::Inductance, ["25/121", "50/171"]),
        (Q::Capacitance, ["171/200", "121/100"]),
        (Q::AmplitudeRatio, ["50/121", "100/171"]),
    ] {
        assert_bounds(&cover.bounds(quantity, &mut ledger()?)?, expected)?;
        assert_eq!(
            cover.determinacy(quantity)?,
            LcDeterminacy::DeterminedFinite
        );
    }
    for quantity in [Q::VoltageAmplitude, Q::CurrentAmplitude, Q::Energy] {
        assert_eq!(cover.determinacy(quantity)?, LcDeterminacy::Underdetermined);
        assert_eq!(
            cover.free_directions(quantity)?,
            &[LcFreeDirection::AmplitudeScale]
        );
        assert!(cover.bounds(quantity, &mut ledger()?)?.upper().is_none());
    }
    for (capacitance, gain) in [("171/200", "9/10"), ("1", "1"), ("121/100", "11/10")] {
        for voltage in ["1/10", "1", "100"] {
            let point = cover.witness(
                &parameters("2", "0", capacitance, gain, voltage, "0")?,
                &mut ledger()?,
            )?;
            check_physical(&element, &point)?;
        }
    }
    Ok(())
}

/// Capacitance calibration composes with unknown initial phase and independent timestamp errors.
/// ´test:integration:lc-relation-capacitance-calibration-timing-phase´
#[test]
fn lc_relation_capacitance_calibration_timing_phase() -> TestResult {
    let element = calibrated(timing(true, false, Vec::new())?, true, true)?;
    let cover = element.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert_eq!(cover.rate(), &[number("10/7")?, number("20/7")?]);
    assert_eq!(cover.phase_gauge(), LcPhaseGauge::AllIntegers);
    assert_bounds(
        &cover.bounds(Q::Inductance, &mut ledger()?)?,
        ["49/484", "98/171"],
    )?;
    assert_bounds(
        &cover.bounds(Q::AmplitudeRatio, &mut ledger()?)?,
        ["35/121", "140/171"],
    )?;
    for (rate, phase) in [("10/7", "5/7"), ("20/7", "-8/7")] {
        for (capacitance, gain) in [("171/200", "9/10"), ("121/100", "11/10")] {
            for gauge in ["-2", "0", "3"] {
                let point = cover.witness(
                    &parameters(rate, phase, capacitance, gain, "1", gauge)?,
                    &mut ledger()?,
                )?;
                check_physical(&element, &point)?;
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
    Ok(())
}

/// Original capacitance restrictions intersect jointly; unsupported simultaneous scales refuse.
/// ´test:integration:lc-relation-capacitance-calibration-original-work´
#[test]
fn lc_relation_capacitance_calibration_original_work() -> TestResult {
    let original = timing(
        false,
        true,
        vec![
            input(Q::Capacitance, ["9/10", "6/5"])?,
            input(Q::Capacitance, ["1", "11/10"])?,
        ],
    )?;
    let element = calibrated(original, true, true)?;
    assert_eq!(element.values(), &[number("1")?, number("11/10")?]);
    let pi = pi()?;
    let cover = element.pairs_cover(cycles(), &pi, &mut ledger()?)?;
    for value in ["1", "21/20", "11/10"] {
        let point = cover.witness(&parameters("2", "0", value, "1", "1", "0")?, &mut ledger()?)?;
        check_physical(&element, &point)?;
    }
    assert!(
        cover
            .witness(
                &parameters("2", "0", "171/200", "9/10", "1", "0")?,
                &mut ledger()?
            )
            .is_err()
    );
    for quantity in [Q::Inductance, Q::VoltageAmplitude, Q::Energy] {
        assert!(
            calibrated(
                timing(false, true, vec![input(quantity, ["1", "1"])?])?,
                true,
                true
            )
            .is_err()
        );
    }
    assert!(
        calibrated(
            timing(false, true, vec![input(Q::Capacitance, ["2", "3"])?])?,
            true,
            true
        )
        .is_err()
    );
    work_boundary(|budget| element.pairs_cover(cycles(), &pi, budget))?;
    let point = parameters("2", "0", "1", "1", "1", "0")?;
    work_boundary(|budget| cover.witness(&point, budget))?;
    work_boundary(|budget| cover.calibration_at_element(&point.element, budget))?;
    work_boundary(|budget| cover.bounds(Q::Inductance, budget))?;
    Ok(())
}

/// Capacitance calibration has its own independent reading and shared-gain inputs.
fn calibrated(
    timing: LcSampleTimes,
    per_sample: bool,
    shared: bool,
) -> TestResult<LcElementSamples> {
    let gain = if shared {
        ["9/10", "11/10"]
    } else {
        ["1", "1"]
    };
    Ok(LcElementSamples::new(
        timing,
        Q::Capacitance,
        readings(per_sample)?,
        &interval(gain)?,
        &mut ledger()?,
    )?)
}

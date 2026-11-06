// SPDX-License-Identifier: AGPL-3.0-only
//! Broad shifted phases restore the exact physical future image and every original calibration fibre.

use super::support::interval;
use super::{TestResult, boundary, cycles, input, ledger, number, pi};
use nrz_bindings::{
    LcClockRefinementState, LcHorizon, LcHorizonValue, LcQuantity as Q, LcRelation, LcSharedClock,
    LcSharedClockPhase, LcSharedMeasurements,
};

/// Broad original phases contain the full measured image; fixed initial phase additionally clips it.
fn phase(tight: bool, fixed: bool, channel: Option<Q>) -> TestResult<LcSharedClockPhase> {
    let old = super::support::phase(tight, channel)?;
    let mut inputs = old.relation().inputs().to_vec();
    inputs.retain(|value| !matches!(value.quantity(), Q::Phase(_)));
    for index in 0..old.relation().observations() {
        inputs.push(input(Q::Phase(index), ["-10", "10"])?);
    }
    if fixed {
        inputs.push(input(Q::InitialPhase, ["0", "0"])?);
        inputs.push(input(Q::Rate, ["2", "2"])?);
    }
    let relation = LcRelation::new(old.relation().epoch().into(), 2, inputs, &mut ledger()?)?;
    let offset = interval(if tight {
        ["-1/8", "1/8"]
    } else {
        ["-1/4", "1/4"]
    })?;
    let clock = LcSharedClock::new(
        relation,
        old.clock().nominal_times().to_vec(),
        &offset,
        &mut ledger()?,
    )?;
    Ok(LcSharedClockPhase::new(
        clock,
        old.samples().to_vec(),
        &offset,
        &mut ledger()?,
    )?)
}

/// Positive amplitude scales retain the broad phase family with the same original readings and gain.
fn amplitude(tight: bool) -> TestResult<LcSharedMeasurements> {
    let old = super::support::amplitude(tight, Q::VoltageAmplitude)?;
    Ok(LcSharedMeasurements::new(
        phase(tight, false, Some(old.channel()))?,
        old.channel(),
        old.samples().to_vec(),
        &interval(if tight {
            ["3/4", "5/4"]
        } else {
            ["1/2", "3/2"]
        })?,
        &mut ledger()?,
    )?)
}

/// Independent exact endpoint literals imply thresholds `5/4+pi` and `5/4+2*pi`.
fn expected(horizon: &LcHorizon, tight: bool) -> TestResult {
    let endpoints = if tight {
        [["5/8", "3/2"], ["-5/8", "5/2"]]
    } else {
        [["5/4", "1"], ["-5/4", "3"]]
    };
    for (actual, expected) in horizon.extremal_parameters().iter().zip(endpoints) {
        assert_eq!(actual, &[number(expected[0])?, number(expected[1])?]);
    }
    let LcHorizonValue::Finite(value) = horizon.value() else {
        return Err("finite original broad horizon".into());
    };
    assert_eq!(
        value.coefficients(),
        (&number("5/4")?, &number(if tight { "2" } else { "1" })?)
    );
    Ok(())
}

/// Both broad phase images and full original extrema retain their conditional calibration and clock fibres.
/// ´test:integration:lc-relation-broad-calibrated-future´
#[test]
fn lc_relation_broad_calibrated_future() -> TestResult {
    for tight in [false, true] {
        let source = phase(tight, false, None)?;
        let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        assert!(cover.measured_cycle_cover().is_none());
        let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
        expected(&horizon, tight)?;
        assert!(horizon.relation().shares_allocation(source.relation()));
        let retained = horizon.calibrated_future().ok_or("original broad source")?;
        assert!(
            retained
                .clock_phase_cover()
                .ok_or("phase cover")?
                .shares_allocation(&cover)
        );
        for (point, expected) in retained
            .phase_witnesses()
            .ok_or("original extrema")?
            .iter()
            .zip(horizon.extremal_parameters())
        {
            source.check_witness(
                point.original().original().values().clone(),
                point.original().offset().clone(),
                point.offset().clone(),
                &mut ledger()?,
            )?;
            let physical = point.original().original().values();
            let alpha = physical
                .initial_phase
                .as_ref()
                .and_then(nrz_bindings::ExactConstant::as_rational)
                .ok_or("rational original phase")?;
            let rate = physical.rate.as_rational().ok_or("rational rate")?;
            let intercept = alpha.add(
                &rate.mul(point.original().offset(), &mut ledger()?)?,
                &mut ledger()?,
            )?;
            assert_eq!(&[intercept, rate.clone()], expected);
        }
    }
    Ok(())
}

/// Fixed initial phase clips the shifted marginal image and rejects a crossed offset tuple.
/// ´test:integration:lc-relation-broad-calibrated-fibre-control´
#[test]
fn lc_relation_broad_calibrated_fibre_control() -> TestResult {
    let source = phase(false, true, None)?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
    assert!(matches!(horizon.value(), LcHorizonValue::Infinite));
    assert_eq!(
        horizon.extremal_parameters(),
        &[
            [number("-1/2")?, number("2")?],
            [number("1/2")?, number("2")?]
        ]
    );
    for point in horizon
        .calibrated_future()
        .ok_or("fixed physical future")?
        .phase_witnesses()
        .ok_or("extrema")?
    {
        assert_eq!(
            point.original().original().values().initial_phase,
            Some(number("0")?.into())
        );
        source.check_witness(
            point.original().original().values().clone(),
            point.original().offset().clone(),
            point.offset().clone(),
            &mut ledger()?,
        )?;
    }
    let wrong = nrz_bindings::LcSharedClockPhaseParameters {
        physical: nrz_bindings::LcClockPhaseParameters {
            rate: number("2")?,
            initial_phase: number("0")?,
            offset: number("-1/4")?,
            gauge: number("0")?,
            ratio: number("1")?,
            voltage: number("1")?,
        },
        offset: number("1/4")?,
    };
    assert!(cover.witness(&wrong, &mut ledger()?).is_err());
    boundary(|budget| cover.forward_reported_horizon(budget).is_ok())?;
    Ok(())
}

/// Narrowing only the measurements can enter the broad path without changing the original physical question.
fn enter_broad(old: &LcSharedClockPhase) -> TestResult<LcSharedClockPhase> {
    Ok(LcSharedClockPhase::new(
        old.clock().clone(),
        vec![interval(["2", "2"])?, interval(["4", "4"])?],
        &interval(["-1/8", "1/8"])?,
        &mut ledger()?,
    )?)
}

/// Refined broad families preserve original endpoint evidence and the same reported reference.
/// ´test:integration:lc-relation-broad-calibrated-refinement´
#[test]
fn lc_relation_broad_calibrated_refinement() -> TestResult {
    let old = phase(false, false, None)?;
    let new = phase(true, false, None)?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
    let LcClockRefinementState::Refined(success) = attempt.state() else {
        return Err(format!("broad refinement {:?}", attempt.state()).into());
    };
    expected(success.horizon(), true)?;
    assert_eq!(
        success.horizon().mathematical().reference(),
        horizon.mathematical().reference()
    );
    assert!(
        success
            .clock_phase_cover()
            .ok_or("refined original cover")?
            .phase()
            .shares_allocation(&new)
    );
    boundary(|budget| {
        previous
            .refine(&inclusion, &horizon, cycles(), budget)
            .is_ok_and(|attempt| matches!(attempt.state(), LcClockRefinementState::Refined(_)))
    })?;
    Ok(())
}

/// Entering the shifted path preserves complete phase and amplitude questions, including infinite horizons.
/// ´test:integration:lc-relation-calibrated-refinement-enters-broad´
#[test]
fn lc_relation_calibrated_refinement_enters_broad() -> TestResult {
    let old = super::support::phase(false, None)?;
    let new = enter_broad(&old)?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert!(previous.measured_cycle_cover().is_some());
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
    let LcClockRefinementState::Refined(success) = attempt.state() else {
        return Err(format!("enter broad {:?}", attempt.state()).into());
    };
    assert!(
        success
            .clock_phase_cover()
            .ok_or("new broad cover")?
            .measured_cycle_cover()
            .is_none()
    );
    assert!(matches!(
        success.horizon().value(),
        LcHorizonValue::Infinite
    ));
    amplitude_enters_broad()
}

/// The amplitude owner preserves its own declaration identity when its phase path changes.
fn amplitude_enters_broad() -> TestResult {
    let old = super::support::amplitude(false, Q::VoltageAmplitude)?;
    let new = LcSharedMeasurements::new(
        enter_broad(old.phase())?,
        old.channel(),
        old.samples().to_vec(),
        &interval(["1/2", "3/2"])?,
        &mut ledger()?,
    )?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
    let LcClockRefinementState::Refined(success) = attempt.state() else {
        return Err(format!("amplitude enter broad {:?}", attempt.state()).into());
    };
    assert!(
        success
            .shared_measurement_cover()
            .ok_or("new amplitude")?
            .measurements()
            .shares_allocation(&new)
    );
    assert!(matches!(
        success.horizon().value(),
        LcHorizonValue::Infinite
    ));
    Ok(())
}

/// Original amplitude equations and shared gain are rechecked at both physical broad future extrema.
/// ´test:integration:lc-relation-broad-amplitude-future-work´
#[test]
fn lc_relation_broad_amplitude_future_work() -> TestResult {
    let source = amplitude(false)?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
    expected(&horizon, false)?;
    for point in horizon
        .calibrated_future()
        .ok_or("amplitude future")?
        .amplitude_witnesses()
        .ok_or("all extrema")?
    {
        let phase = point.original();
        source.check_witness(
            phase.original().original().values().clone(),
            phase.original().offset().clone(),
            phase.offset().clone(),
            point.gain().clone(),
            &mut ledger()?,
        )?;
    }
    boundary(|budget| cover.forward_reported_horizon(budget).is_ok())?;
    Ok(())
}

/// Interior-rate original witnesses lie inside the complete future image at and after its reference.
/// ´test:integration:lc-relation-broad-future-interior-oracle´
#[test]
fn lc_relation_broad_future_interior_oracle() -> TestResult {
    for fixed in [false, true] {
        let source = phase(false, fixed, None)?;
        let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
        let middle = cover.rate()[0]
            .add(&cover.rate()[1], &mut ledger()?)?
            .div(&number("2")?, &mut ledger()?)?;
        for rate in [&cover.rate()[0], &middle, &cover.rate()[1]] {
            for alpha in cover.phase_at_rate(rate, &mut ledger()?)? {
                for offset in cover.offset_at_phase(rate, &alpha, &mut ledger()?)? {
                    for clock in
                        cover.clock_offset_at_phases(rate, &alpha, &offset, &mut ledger()?)?
                    {
                        let parameters = nrz_bindings::LcSharedClockPhaseParameters {
                            physical: nrz_bindings::LcClockPhaseParameters {
                                rate: rate.clone(),
                                initial_phase: alpha.clone(),
                                offset: clock,
                                gauge: number("0")?,
                                ratio: number("1")?,
                                voltage: number("1")?,
                            },
                            offset: offset.clone(),
                        };
                        point_in_future(&cover, &horizon, &parameters)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Reconstruct each point independently, then compare its actual-time phase with the reported image.
fn point_in_future(
    cover: &nrz_bindings::LcClockPhaseCover,
    horizon: &LcHorizon,
    parameters: &nrz_bindings::LcSharedClockPhaseParameters,
) -> TestResult {
    let point = cover.witness(parameters, &mut ledger()?)?;
    let physical = &parameters.physical;
    cover.phase().check_witness(
        point.original().original().values().clone(),
        physical.offset.clone(),
        parameters.offset.clone(),
        &mut ledger()?,
    )?;
    for reported in [horizon.mathematical().reference().clone(), number("10")?] {
        let actual = reported.add(&physical.offset, &mut ledger()?)?;
        let future = physical
            .initial_phase
            .add(&physical.rate.mul(&actual, &mut ledger()?)?, &mut ledger()?)?;
        let extrema = horizon.extremal_parameters();
        let lower = extrema[0][0].add(
            &extrema[0][1].mul(&reported, &mut ledger()?)?,
            &mut ledger()?,
        )?;
        let upper = extrema[1][0].add(
            &extrema[1][1].mul(&reported, &mut ledger()?)?,
            &mut ledger()?,
        )?;
        assert_ne!(
            future.compare(&lower, &mut ledger()?)?,
            std::cmp::Ordering::Less
        );
        assert_ne!(
            future.compare(&upper, &mut ledger()?)?,
            std::cmp::Ordering::Greater
        );
    }
    Ok(())
}

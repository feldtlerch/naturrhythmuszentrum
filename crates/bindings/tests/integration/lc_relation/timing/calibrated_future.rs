// SPDX-License-Identifier: AGPL-3.0-only
//! Reported future thresholds retain original calibration fibres and checked physical extremizers.

mod impossible;
mod refinement;
mod support;
use super::super::support::{TestResult, ledger};
use super::support::{cycles, input, number, pi};
use nrz_bindings::{LcHorizonCoordinate, LcQuantity as Q};
use support::{amplitude, boundary, finite, phase};

/// A phase-calibrated future query keeps both offsets and full original extremizers.
/// ´test:integration:lc-relation-calibrated-phase-horizon´
#[test]
fn lc_relation_calibrated_phase_horizon() -> TestResult {
    for tight in [false, true] {
        let source = phase(tight, None)?;
        let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
        finite(&horizon, tight)?;
        assert_eq!(horizon.coordinate(), LcHorizonCoordinate::ReportedClock);
        assert!(horizon.relation().shares_allocation(source.relation()));
        assert!(horizon.clock_cover().is_none());
        let future = horizon
            .calibrated_future()
            .ok_or("original calibrated source")?;
        assert!(
            future
                .clock_phase_cover()
                .ok_or("phase cover")?
                .shares_allocation(&cover)
        );
        for (point, expected) in future
            .phase_witnesses()
            .ok_or("phase extremizers")?
            .iter()
            .zip(horizon.extremal_parameters())
        {
            source.check_witness(
                point.original().original().values().clone(),
                point.original().offset().clone(),
                point.offset().clone(),
                &mut ledger()?,
            )?;
            assert_eq!(
                point.original().original().values().rate.as_rational(),
                Some(&expected[1])
            );
        }
        assert!(
            cover
                .forward_reported_horizon_from(&number("0")?, &mut ledger()?)
                .is_err()
        );
    }
    Ok(())
}

/// Either amplitude channel retains every reading and the same original clock/phase future domain.
/// ´test:integration:lc-relation-calibrated-amplitude-horizon´
#[test]
fn lc_relation_calibrated_amplitude_horizon() -> TestResult {
    for channel in [Q::VoltageAmplitude, Q::CurrentAmplitude] {
        let source = amplitude(false, channel)?;
        let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
        let horizon = cover.forward_reported_horizon(&mut ledger()?)?;
        finite(&horizon, false)?;
        let future = horizon
            .calibrated_future()
            .ok_or("amplitude future owner")?;
        assert!(
            future
                .shared_measurement_cover()
                .ok_or("amplitude cover")?
                .shares_allocation(&cover)
        );
        assert!(horizon.relation().shares_allocation(source.relation()));
        for point in future
            .amplitude_witnesses()
            .ok_or("amplitude extremizers")?
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
    }
    Ok(())
}

/// Future construction charges its original extremizers and reuses prepared circle evidence.
/// ´test:integration:lc-relation-calibrated-horizon-work´
#[test]
fn lc_relation_calibrated_horizon_work() -> TestResult {
    let phase = phase(false, None)?.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    boundary(|budget| phase.forward_reported_horizon(budget).is_ok())?;
    let amplitude =
        amplitude(false, Q::VoltageAmplitude)?.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    boundary(|budget| amplitude.forward_reported_horizon(budget).is_ok())?;
    Ok(())
}

/// The broad shifted-phase adapter never publishes its auxiliary intercept as physical future evidence.
/// ´test:integration:lc-relation-calibrated-future-domain-refusal´
#[test]
fn lc_relation_calibrated_future_domain_refusal() -> TestResult {
    use nrz_bindings::{LcRelation, LcSharedClock, LcSharedClockPhase};
    let original = phase(false, None)?;
    let mut inputs = original.relation().inputs().to_vec();
    inputs.retain(|value| !matches!(value.quantity(), Q::Phase(_)));
    for index in 0..2 {
        inputs.push(input(Q::Phase(index), ["-10", "10"])?);
    }
    let relation = LcRelation::new(
        original.relation().epoch().into(),
        2,
        inputs,
        &mut ledger()?,
    )?;
    let clock = LcSharedClock::new(
        relation,
        original.clock().nominal_times().to_vec(),
        &support::interval(["-1/4", "1/4"])?,
        &mut ledger()?,
    )?;
    let source = LcSharedClockPhase::new(
        clock,
        original.samples().to_vec(),
        &support::interval(["-1/4", "1/4"])?,
        &mut ledger()?,
    )?;
    let cover = source.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert!(matches!(
        cover.forward_reported_horizon(&mut ledger()?),
        Err(nrz_bindings::LcRelationError::Projection)
    ));
    Ok(())
}

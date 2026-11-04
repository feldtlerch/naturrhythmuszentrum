// SPDX-License-Identifier: AGPL-3.0-only
//! Whole-original calibrated inclusion preserves authority, conditional extrema and exhausted evidence.

use super::{TestResult, amplitude, boundary, cycles, finite, ledger, phase, pi};
use nrz_bindings::{
    Budget, LcClockRefinementState, LcQuantity as Q, LcRelationError, Limits, LimitsOverride,
};

/// Shared clock and phase refinement preserves the same reported reference and all original fibres.
/// ´test:integration:lc-relation-calibrated-phase-refinement´
#[test]
fn lc_relation_calibrated_phase_refinement() -> TestResult {
    let old = phase(false, None)?;
    let new = phase(true, None)?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
    let LcClockRefinementState::Refined(success) = attempt.state() else {
        return Err(format!("calibrated phase refinement {:?}", attempt.state()).into());
    };
    finite(success.horizon(), true)?;
    assert!(
        attempt
            .previous_clock_phase()
            .ok_or("previous phase")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .inclusion()
            .refined_clock_phase()
            .ok_or("refined phase")?
            .shares_allocation(&new)
    );
    assert!(
        success
            .clock_phase_cover()
            .ok_or("new cover")?
            .phase()
            .shares_allocation(&new)
    );
    assert_eq!(
        success.horizon().mathematical().reference(),
        horizon.mathematical().reference()
    );
    assert!(
        !success
            .finite_horizon_margin()
            .ok_or("finite margin")?
            .is_negative()
    );
    for point in success
        .horizon()
        .calibrated_future()
        .ok_or("future")?
        .phase_witnesses()
        .ok_or("extrema")?
    {
        old.check_witness(
            point.original().original().values().clone(),
            point.original().offset().clone(),
            point.offset().clone(),
            &mut ledger()?,
        )?;
    }
    Ok(())
}

/// Amplitude readings and gain refine jointly with both shared offsets and original physical scale bounds.
/// ´test:integration:lc-relation-calibrated-amplitude-refinement´
#[test]
fn lc_relation_calibrated_amplitude_refinement() -> TestResult {
    let old = amplitude(false, Q::VoltageAmplitude)?;
    let new = amplitude(true, Q::VoltageAmplitude)?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut ledger()?)?;
    let LcClockRefinementState::Refined(success) = attempt.state() else {
        return Err(format!("calibrated amplitude refinement {:?}", attempt.state()).into());
    };
    finite(success.horizon(), true)?;
    assert!(
        attempt
            .previous_shared_measurement()
            .ok_or("previous amplitude")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .inclusion()
            .refined_shared_measurement()
            .ok_or("refined amplitude")?
            .shares_allocation(&new)
    );
    assert!(
        success
            .shared_measurement_cover()
            .ok_or("new cover")?
            .measurements()
            .shares_allocation(&new)
    );
    assert_eq!(
        success.horizon().mathematical().reference(),
        horizon.mathematical().reference()
    );
    for point in success
        .horizon()
        .calibrated_future()
        .ok_or("future")?
        .amplitude_witnesses()
        .ok_or("extrema")?
    {
        let phase = point.original();
        old.check_witness(
            phase.original().original().values().clone(),
            phase.original().offset().clone(),
            phase.offset().clone(),
            point.gain().clone(),
            &mut ledger()?,
        )?;
    }
    Ok(())
}

/// Equal-valued sources never replace authority; widening any complete original declaration refuses.
/// ´test:integration:lc-relation-calibrated-refinement-authority´
#[test]
fn lc_relation_calibrated_refinement_authority() -> TestResult {
    let old = phase(false, None)?;
    let new = phase(true, None)?;
    assert!(new.refinement_inputs(&old, &mut ledger()?).is_err());
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let foreign = phase(false, None)?.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    assert!(matches!(
        foreign.refine(&inclusion, &horizon, cycles(), &mut ledger()?),
        Err(LcRelationError::Containment)
    ));
    let foreign_horizon = foreign.forward_reported_horizon(&mut ledger()?)?;
    assert!(matches!(
        previous.refine(&inclusion, &foreign_horizon, cycles(), &mut ledger()?),
        Err(LcRelationError::Containment)
    ));
    let old = amplitude(false, Q::VoltageAmplitude)?;
    let new = amplitude(true, Q::VoltageAmplitude)?;
    assert!(new.refinement_inputs(&old, &mut ledger()?).is_err());
    assert!(
        old.refinement_inputs(&amplitude(true, Q::CurrentAmplitude)?, &mut ledger()?)
            .is_err()
    );
    Ok(())
}

/// Exhaustion retains old calibrated cover, old horizon and every proposed refined declaration.
/// ´test:integration:lc-relation-calibrated-refinement-exhaustion´
#[test]
fn lc_relation_calibrated_refinement_exhaustion() -> TestResult {
    let old = amplitude(false, Q::VoltageAmplitude)?;
    let new = amplitude(true, Q::VoltageAmplitude)?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    let mut budget = Budget::new(Limits::default().resolve(&LimitsOverride {
        work: Some(1_000),
        ..LimitsOverride::default()
    })?)?;
    let attempt = previous.refine(&inclusion, &horizon, cycles(), &mut budget)?;
    assert!(matches!(
        attempt.state(),
        LcClockRefinementState::Undecided(LcRelationError::Algebra(_))
    ));
    assert!(attempt.refined_horizon().is_none());
    assert!(
        attempt
            .previous_shared_measurement()
            .ok_or("old original")?
            .shares_allocation(&previous)
    );
    assert!(
        attempt
            .inclusion()
            .refined_shared_measurement()
            .ok_or("new original")?
            .shares_allocation(&new)
    );
    assert_eq!(attempt.proposed_cycles(), cycles());
    assert_eq!(attempt.usage()[1].work, budget.usage().work);
    assert!(attempt.solving_usage().is_some());
    Ok(())
}

/// Inclusion and complete refined future publication charge the exact cumulative caller ledger.
/// ´test:integration:lc-relation-calibrated-refinement-work´
#[test]
fn lc_relation_calibrated_refinement_work() -> TestResult {
    let old = phase(false, None)?;
    let new = phase(true, None)?;
    boundary(|budget| old.refinement_inputs(&new, budget).is_ok())?;
    let inclusion = old.refinement_inputs(&new, &mut ledger()?)?;
    let previous = old.pairs_cover(cycles(), &pi()?, &mut ledger()?)?;
    let horizon = previous.forward_reported_horizon(&mut ledger()?)?;
    boundary(|budget| {
        previous
            .refine(&inclusion, &horizon, cycles(), budget)
            .is_ok_and(|attempt| matches!(attempt.state(), LcClockRefinementState::Refined(_)))
    })?;
    Ok(())
}

/// Measurement-only widening is rejected even when every original physical input stays identical.
/// ´test:integration:lc-relation-calibrated-refinement-layer-refusals´
#[test]
fn lc_relation_calibrated_refinement_layer_refusals() -> TestResult {
    use nrz_bindings::{LcSharedClockPhase, LcSharedMeasurements};
    let old = phase(true, None)?;
    let wider = LcSharedClockPhase::new(
        old.clock().clone(),
        old.samples().to_vec(),
        &super::support::interval(["-1/4", "1/4"])?,
        &mut ledger()?,
    )?;
    assert!(old.refinement_inputs(&wider, &mut ledger()?).is_err());
    let wider_readings = phase(false, None)?.samples().to_vec();
    let wider = LcSharedClockPhase::new(
        old.clock().clone(),
        wider_readings,
        &super::support::interval(["-1/8", "1/8"])?,
        &mut ledger()?,
    )?;
    assert!(old.refinement_inputs(&wider, &mut ledger()?).is_err());
    let old = amplitude(true, Q::VoltageAmplitude)?;
    let wider = LcSharedMeasurements::new(
        old.phase().clone(),
        old.channel(),
        old.samples().to_vec(),
        &super::support::interval(["1/2", "3/2"])?,
        &mut ledger()?,
    )?;
    assert!(old.refinement_inputs(&wider, &mut ledger()?).is_err());
    let missing = LcSharedMeasurements::new(
        old.phase().clone(),
        old.channel(),
        vec![old.samples()[0].clone()],
        &super::support::interval(["3/4", "5/4"])?,
        &mut ledger()?,
    )?;
    assert!(old.refinement_inputs(&missing, &mut ledger()?).is_err());
    Ok(())
}
